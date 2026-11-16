package com.lingzidev.xiaochengyouxi;

import java.nio.ByteBuffer;
import java.security.KeyFactory;
import java.security.PublicKey;
import java.security.Signature;
import java.security.spec.X509EncodedKeySpec;
import java.util.Base64;
import javax.crypto.Cipher;
import javax.crypto.spec.GCMParameterSpec;
import javax.crypto.spec.SecretKeySpec;

/** Pure-JDK cross-check for server/src/behavior-vector-v1.json. */
public final class BehaviorProtocolCheck {
  static final String KEY = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
  static final String PUBLIC = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAESd4RTKFo/BE/zKNr8+T684/mSZpODzy3y6suCMdXdj60DkX/0caaaL6IqAWoaBndTd/hZWVuRsPKWLR90+3Ytg==";
  static final String EXPECTED = "69d9534a0aa891dc35f5d490659b99836fcd506aa9890148fe7e45936629bb2a";
  static final String HEADER = "T0JBMQABAgMEBUYHiAkKCwwNDg8AAAAAAAAAASAhIiMkJSYnKCkqKywtLi8wMTIzNDU2Nzg5Ojs8PT4/oKGiowAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABo";
  static final String CIPHER = "sK+5FB69RLQasq5me2sNik0ZmV1oYB8sqdVyodEB2GwUASseZbu61mBUFE4OaYge49Jg7vNNlZzq+gOFcpW8c01oYbway483GELjoqvVbYV7HDc5Cs2hbDqM8uBice8Kqbvs/cvbNmM=";
  static final String SIGNATURE = "MEYCIQCdud9R6+IHsqxsSO6pnczxdahcYQ0IcOF3nYe0M3a3wwIhAIjiMF4AKikUNe4wZZM38zhtT5+pgkI+M6SCpaTOq3Tf";

  public static void main(String[] args) throws Exception {
    Base64.Decoder decoder = Base64.getDecoder();
    byte[] header = decoder.decode(HEADER), ciphertext = decoder.decode(CIPHER);
    if (header.length != BehaviorProtocol.HEADER_BYTES || ByteBuffer.wrap(header, 104, 4).getInt() != ciphertext.length)
      throw new AssertionError("header length field");
    Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
    cipher.init(Cipher.DECRYPT_MODE, new SecretKeySpec(decoder.decode(KEY), "AES"),
        new GCMParameterSpec(128, slice(header, 60, 12)));
    cipher.updateAAD(header);
    byte[] plaintext = cipher.doFinal(ciphertext);
    if (plaintext.length != 88) throw new AssertionError("payload length");
    PublicKey key = KeyFactory.getInstance("EC").generatePublic(new X509EncodedKeySpec(decoder.decode(PUBLIC)));
    Signature verifier = Signature.getInstance("SHA256withECDSA");
    verifier.initVerify(key);
    byte[] signed = ByteBuffer.allocate(header.length + ciphertext.length).put(header).put(ciphertext).array();
    verifier.update(signed);
    if (!verifier.verify(decoder.decode(SIGNATURE))) throw new AssertionError("ECDSA signature");
    byte[] hash = java.security.MessageDigest.getInstance("SHA-256").digest(signed);
    if (!BehaviorProtocol.hex(hash).equals(EXPECTED)) throw new AssertionError("packet hash");
    System.out.println("OBA1 vector verified: header=108, payload=88, ECDSA, AES-GCM, packet hash");
  }

  private static byte[] slice(byte[] value, int offset, int length) {
    byte[] result = new byte[length];
    System.arraycopy(value, offset, result, 0, length);
    return result;
  }
}
