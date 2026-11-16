package com.lingzidev.xiaochengyouxi;

import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.PrivateKey;
import java.security.Signature;
import java.util.List;
import java.util.UUID;
import javax.crypto.Cipher;
import javax.crypto.spec.GCMParameterSpec;
import javax.crypto.spec.SecretKeySpec;

/** OBA1 wire encoding. This class has no Android dependency and is checked on the JVM. */
final class BehaviorProtocol {
  static final int HEADER_BYTES = 108;
  static final int UNKNOWN_POINT = 65535;

  static final class Event {
    final long elapsed;
    final int type, screen, control, action, x, y, endX, endY;
    final long duration;
    final String reference;

    Event(
        long elapsed,
        int type,
        int screen,
        int control,
        int action,
        int x,
        int y,
        int endX,
        int endY,
        long duration,
        String reference) {
      this.elapsed = elapsed;
      this.type = type;
      this.screen = screen;
      this.control = control;
      this.action = action;
      this.x = x;
      this.y = y;
      this.endX = endX;
      this.endY = endY;
      this.duration = duration;
      this.reference = reference;
    }
  }

  static final class Frame {
    final byte[] bytes, hash;

    Frame(byte[] bytes, byte[] hash) {
      this.bytes = bytes;
      this.hash = hash;
    }
  }

  static byte[] payload(long wallTime, int width, int height, int dropped, List<Event> events)
      throws Exception {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    DataOutputStream output = new DataOutputStream(bytes);
    output.writeLong(wallTime);
    output.writeShort(width);
    output.writeShort(height);
    output.writeInt(dropped);
    output.writeShort(events.size());
    for (Event event : events) {
      byte[] reference = event.reference.getBytes(StandardCharsets.US_ASCII);
      if (reference.length > 96 || !event.reference.matches("[A-Za-z0-9:_-]*"))
        throw new IllegalArgumentException("Invalid audit operation reference");
      output.writeInt((int) event.elapsed);
      output.writeByte(event.type);
      output.writeByte(event.screen);
      output.writeByte(event.control);
      output.writeByte(event.action);
      output.writeShort(event.x);
      output.writeShort(event.y);
      output.writeShort(event.endX);
      output.writeShort(event.endY);
      output.writeInt((int) event.duration);
      output.writeByte(reference.length);
      output.write(reference);
    }
    return bytes.toByteArray();
  }

  static Frame seal(
      UUID session,
      long sequence,
      byte[] challenge,
      byte[] noncePrefix,
      byte[] previous,
      byte[] key,
      byte[] payload,
      PrivateKey signingKey)
      throws Exception {
    if (sequence <= 0
        || challenge.length != 32
        || noncePrefix.length != 4
        || previous.length != 32
        || key.length != 32) throw new IllegalArgumentException("Invalid OBA1 session material");
    byte[] nonce = ByteBuffer.allocate(12).put(noncePrefix).putLong(sequence).array();
    byte[] header =
        ByteBuffer.allocate(HEADER_BYTES)
            .put("OBA1".getBytes(StandardCharsets.US_ASCII))
            .putLong(session.getMostSignificantBits())
            .putLong(session.getLeastSignificantBits())
            .putLong(sequence)
            .put(challenge)
            .put(nonce)
            .put(previous)
            .putInt(payload.length + 16)
            .array();
    Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
    cipher.init(
        Cipher.ENCRYPT_MODE, new SecretKeySpec(key, "AES"), new GCMParameterSpec(128, nonce));
    cipher.updateAAD(header);
    byte[] encrypted = cipher.doFinal(payload);
    byte[] signed =
        ByteBuffer.allocate(header.length + encrypted.length).put(header).put(encrypted).array();
    Signature signer = Signature.getInstance("SHA256withECDSA");
    signer.initSign(signingKey);
    signer.update(signed);
    byte[] signature = signer.sign();
    byte[] frame =
        ByteBuffer.allocate(signed.length + 2 + signature.length)
            .put(signed)
            .putShort((short) signature.length)
            .put(signature)
            .array();
    return new Frame(frame, MessageDigest.getInstance("SHA-256").digest(signed));
  }

  static String hex(byte[] bytes) {
    StringBuilder value = new StringBuilder(bytes.length * 2);
    for (byte b : bytes) value.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
    return value.toString();
  }
}
