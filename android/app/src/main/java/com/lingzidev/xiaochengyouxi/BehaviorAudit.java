package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.app.Dialog;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import android.util.Log;
import android.view.MotionEvent;
import com.device.trust.secure.TouchAudit;
import android.view.View;
import android.widget.EditText;
import androidx.appcompat.view.WindowCallbackWrapper;
import java.security.KeyPairGenerator;
import java.security.KeyStore;
import java.security.PrivateKey;
import java.security.spec.ECGenParameterSpec;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.UUID;
import org.json.JSONObject;

/** UI-thread state, one bounded buffer and one in-flight frame. No disk history. */
final class BehaviorAudit {
  private static final String KEY_ALIAS = "xiaochengyouxi-behavior-p256-v1";
  private final Activity activity;
  private final Api api;
  private final Handler handler = new Handler(Looper.getMainLooper());
  private final ArrayList<BehaviorProtocol.Event> events = new ArrayList<>();
  private final LinkedHashMap<Object, Integer> overlays = new LinkedHashMap<>();
  private String owner = "", session = "";
  private byte[] key, prefix, challenge, previous = new byte[32];
  private PrivateKey signingKey;
  private long origin, sequence = 1, maxElapsed = 1800000, screenSince, lastFlush;
  private int epoch, dropped, baseScreen, screen, externalScreen;
  private int interval = 15, heartbeat = 15, maximum = 200, touchSample = 100;
  private boolean foreground, enabled, preparing, sending, stopped, destroyed, sensitive;
  private Touch touch;
  private long downTime;
  private int downX, downY;
  private final Runnable tick = this::tick;

  BehaviorAudit(Activity activity, Api api) {
    this.activity = activity;
    this.api = api;
  }

  void loginUnavailable() {
    // DeviceTrust login reporting remains independent; no behavior session is opened.
  }

  void begin() {
    if (api.token.isEmpty() || !consented()) return;
    if (!owner.equals(api.token)) {
      reset();
      owner = api.token;
    }
    stopped = false;
    refreshPolicy();
  }

  void refreshPolicy() {
    if (destroyed || api.token.isEmpty() || !consented() || !owner.equals(api.token)) return;
    int generation = epoch;
    api.get(
        "/behavior/config",
        value -> {
          if (!current(generation)) return;
          try {
            JSONObject config = (JSONObject) value;
            if (!config.getString("protocol").equals("OBA1"))
              throw new IllegalArgumentException("Unsupported behavior protocol");
            enabled = config.getBoolean("enabled");
            interval = config.getInt("batch_interval_seconds");
            heartbeat = config.getInt("heartbeat_seconds");
            maximum = config.getInt("max_events");
            touchSample = config.getInt("touch_sample_percent");
            if (interval < 1
                || heartbeat < 1
                || maximum < 1
                || maximum > 200
                || touchSample < 0
                || touchSample > 100)
              throw new IllegalArgumentException("Invalid behavior collection policy");
            if (!enabled) clearSession();
            else if (session.isEmpty() && !preparing && !stopped) prepare();
          } catch (Exception error) {
            fail("session", error);
          }
        },
        failure("session"));
  }

  private boolean consented() {
    return api.preferences.getBoolean(DeviceTrustClient.CONSENT, false);
  }

  private boolean current(int generation) {
    return !destroyed && generation == epoch && owner.equals(api.token) && !owner.isEmpty();
  }

  private boolean live() {
    return !session.isEmpty()
        && !stopped
        && enabled
        && consented()
        && owner.equals(api.token)
        && SystemClock.elapsedRealtime() - origin <= maxElapsed;
  }

  private void prepare() {
    preparing = true;
    int generation = epoch;
    new Thread(
            () -> {
              try {
                KeyStore store = KeyStore.getInstance("AndroidKeyStore");
                store.load(null);
                if (!store.containsAlias(KEY_ALIAS)) {
                  KeyPairGenerator generator =
                      KeyPairGenerator.getInstance(
                          KeyProperties.KEY_ALGORITHM_EC, "AndroidKeyStore");
                  generator.initialize(
                      new KeyGenParameterSpec.Builder(KEY_ALIAS, KeyProperties.PURPOSE_SIGN)
                          .setAlgorithmParameterSpec(new ECGenParameterSpec("secp256r1"))
                          .setDigests(KeyProperties.DIGEST_SHA256)
                          .build());
                  generator.generateKeyPair();
                }
                PrivateKey privateKey = (PrivateKey) store.getKey(KEY_ALIAS, null);
                String publicKey =
                    Base64.encodeToString(
                        store.getCertificate(KEY_ALIAS).getPublicKey().getEncoded(),
                        Base64.NO_WRAP);
                JSONObject body =
                    Api.object(
                        "public_key",
                        publicKey,
                        "client_nonce",
                        UUID.randomUUID().toString(),
                        "consent",
                        true,
                        "device_model",
                        (Build.MANUFACTURER + " " + Build.MODEL).trim(),
                        "os_version",
                        Build.VERSION.RELEASE,
                        "app_version",
                        BuildConfig.VERSION_NAME);
                activity.runOnUiThread(
                    () -> {
                      if (!current(generation)) return;
                      signingKey = privateKey;
                      api.deviceTrust.assess(
                          "behavior",
                          Api.object(
                              "method", "POST", "path", "/api/behavior/session", "body", body),
                          proof ->
                              api.requestWithRisk(
                                  "/behavior/session",
                                  body,
                                  proof,
                                  result -> accept((JSONObject) result, generation),
                                  failure("session")),
                          failure("session"));
                    });
              } catch (Exception | LinkageError error) {
                activity.runOnUiThread(
                    () -> {
                      if (current(generation)) fail("session", error);
                    });
              }
            },
            "behavior-key")
        .start();
  }

  private void accept(JSONObject response, int generation) {
    if (!current(generation)) return;
    try {
      if (!response.getString("protocol").equals("OBA1"))
        throw new IllegalArgumentException("Unsupported behavior protocol");
      session = response.getString("session_id");
      UUID.fromString(session);
      key = Base64.decode(response.getString("key_base64"), Base64.DEFAULT);
      prefix = Base64.decode(response.getString("nonce_prefix"), Base64.DEFAULT);
      challenge = Base64.decode(response.getString("challenge"), Base64.DEFAULT);
      maxElapsed = response.getLong("max_elapsed_ms");
      if (key.length != 32 || prefix.length != 4 || challenge.length != 32 || maxElapsed <= 0)
        throw new IllegalArgumentException("Invalid behavior session material");
      origin = SystemClock.elapsedRealtime();
      long elapsedOffset = response.optLong("elapsed_offset_ms", 0);
      origin -= elapsedOffset;
      previous = new byte[32];
      sequence = response.optLong("next_seq", 1);
      if (sequence <= 0) throw new IllegalArgumentException("Invalid behavior sequence");
      String prior = response.optString("previous_hash", "");
      if (!prior.isEmpty()) {
        if (prior.length() != 64)
          throw new IllegalArgumentException("Invalid behavior previous hash");
        for (int i = 0; i < 32; i++) {
          int high = Character.digit(prior.charAt(i * 2), 16);
          int low = Character.digit(prior.charAt(i * 2 + 1), 16);
          if (high < 0 || low < 0)
            throw new IllegalArgumentException("Invalid behavior previous hash");
          previous[i] = (byte) ((high << 4) | low);
        }
      }
      preparing = false;
      lastFlush = origin;
      if (foreground) {
        record(3, 0, 0, 0, "");
        record(1, 0, 0, 0, "");
        screenSince = origin;
        schedule();
      }
    } catch (Exception error) {
      fail("session", error);
    }
  }

  void resume() {
    if (foreground || destroyed) return;
    foreground = true;
    if (live()) {
      record(3, 0, 0, 0, "");
      record(1, 0, 0, 0, "");
      screenSince = SystemClock.elapsedRealtime();
      schedule();
    } else if (!owner.isEmpty() && owner.equals(api.token)) {
      if (!session.isEmpty()) clearSession();
      stopped = false;
      refreshPolicy();
    }
  }

  void pause() {
    if (!foreground) return;
    leaveScreen();
    record(4, 0, 0, 0, "");
    foreground = false;
    touch = null;
    downTime = 0;
    handler.removeCallbacks(tick);
    flush();
  }

  void page(int value) {
    baseScreen = value;
    updateScreen();
  }

  void sensitive(boolean value) {
    sensitive = value;
  }

  void external(int value) {
    externalScreen = value;
    updateScreen();
  }

  void overlay(Object token, int value) {
    if (value == 0) overlays.remove(token);
    else overlays.put(token, value);
    updateScreen();
  }

  void dialog(Dialog dialog, int value) {
    View decor = dialog.getWindow().getDecorView();
    dialog
        .getWindow()
        .setCallback(
            new WindowCallbackWrapper(dialog.getWindow().getCallback()) {
              @Override
              public boolean dispatchTouchEvent(MotionEvent event) {
                TouchAudit.record(event);
                touch(event);
                return super.dispatchTouchEvent(event);
              }
            });
    decor.addOnAttachStateChangeListener(
        new View.OnAttachStateChangeListener() {
          @Override
          public void onViewAttachedToWindow(View view) {
            overlay(dialog, value);
          }

          @Override
          public void onViewDetachedFromWindow(View view) {
            overlay(dialog, 0);
          }
        });
    if (dialog.isShowing()) overlay(dialog, value);
  }

  private void updateScreen() {
    int next = baseScreen;
    for (int value : overlays.values()) next = value;
    if (externalScreen != 0) next = externalScreen;
    if (next == screen) return;
    leaveScreen();
    screen = next;
    if (foreground && live()) {
      screenSince = SystemClock.elapsedRealtime();
      record(1, 0, 0, 0, "");
    }
  }

  private void leaveScreen() {
    if (foreground && live())
      record(2, 0, 0, Math.max(0, SystemClock.elapsedRealtime() - screenSince), "");
  }

  void click(int control, String reference) {
    if (!foreground || !live()) return;
    Touch point = touch;
    if (point != null && !point.used && SystemClock.elapsedRealtime() - point.ended < 1000) {
      point.used = true;
      append(
          new BehaviorProtocol.Event(
              elapsed(),
              5,
              screen,
              control,
              0,
              point.x,
              point.y,
              point.endX,
              point.endY,
              0,
              safeReference(reference)));
    } else record(5, control, 0, 0, reference);
  }

  void business(int action, String reference) {
    record(7, 0, action, 0, reference);
  }

  void response(String path, JSONObject response) {
    int action =
        path.equals("/rewards/start")
            ? 1
            : path.startsWith("/rewards/") && path.endsWith("/complete")
                ? 2
                : path.equals("/check-in")
                    ? 3
                    : path.equals("/rain/join")
                        ? 4
                        : path.equals("/rain/claim") ? 5 : path.equals("/withdrawals") ? 6 : 0;
    if (action != 0) business(action, response.optString("operation_id"));
  }

  private String safeReference(String value) {
    return value != null && value.length() <= 96 && value.matches("[A-Za-z0-9:_-]*") ? value : "";
  }

  private long elapsed() {
    return Math.max(0, SystemClock.elapsedRealtime() - origin);
  }

  private void record(int type, int control, int action, long duration, String reference) {
    int unknown = BehaviorProtocol.UNKNOWN_POINT;
    append(
        new BehaviorProtocol.Event(
            elapsed(),
            type,
            screen,
            control,
            action,
            unknown,
            unknown,
            unknown,
            unknown,
            duration,
            safeReference(reference)));
  }

  private void append(BehaviorProtocol.Event event) {
    if (!live()) return;
    if (events.size() >= maximum) {
      dropped++;
      return;
    }
    events.add(event);
    if (events.size() >= maximum) flush();
  }

  void touch(MotionEvent event) {
    if (!foreground
        || !live()
        || sensitive
        || screen == 7
        || activity.getCurrentFocus() instanceof EditText) return;
    View decor = activity.getWindow().getDecorView();
    if (decor.getWidth() <= 0 || decor.getHeight() <= 0) return;
    int[] origin = new int[2];
    decor.getLocationOnScreen(origin);
    int x =
        Math.round(
            Math.max(0, Math.min(1, (event.getRawX() - origin[0]) / decor.getWidth())) * 10000);
    int y =
        Math.round(
            Math.max(0, Math.min(1, (event.getRawY() - origin[1]) / decor.getHeight())) * 10000);
    if (event.getActionMasked() == MotionEvent.ACTION_DOWN) {
      downTime = SystemClock.elapsedRealtime();
      downX = x;
      downY = y;
      touch = null;
    } else if (event.getActionMasked() == MotionEvent.ACTION_CANCEL) downTime = 0;
    else if (event.getActionMasked() == MotionEvent.ACTION_UP && downTime != 0) {
      Touch point =
          new Touch(
              downX,
              downY,
              x,
              y,
              SystemClock.elapsedRealtime(),
              SystemClock.elapsedRealtime() - downTime);
      downTime = 0;
      touch = point;
      handler.post(
          () -> {
            if (point.used || !foreground || !live() || Math.random() * 100 >= touchSample) return;
            point.used = true;
            append(
                new BehaviorProtocol.Event(
                    elapsed(),
                    6,
                    screen,
                    0,
                    0,
                    point.x,
                    point.y,
                    point.endX,
                    point.endY,
                    point.duration,
                    ""));
          });
    }
  }

  private static final class Touch {
    final int x, y, endX, endY;
    final long ended, duration;
    boolean used;

    Touch(int x, int y, int endX, int endY, long ended, long duration) {
      this.x = x;
      this.y = y;
      this.endX = endX;
      this.endY = endY;
      this.ended = ended;
      this.duration = duration;
    }
  }

  private void schedule() {
    handler.removeCallbacks(tick);
    if (foreground && live()) handler.postDelayed(tick, heartbeat * 1000L);
  }

  private void tick() {
    if (!foreground) return;
    if (!live()) {
      if (!stopped && enabled && !owner.isEmpty()) {
        clearSession();
        refreshPolicy();
      }
      return;
    }
    record(8, 0, 0, 0, "");
    if (SystemClock.elapsedRealtime() - lastFlush >= interval * 1000L) flush();
    schedule();
  }

  private void flush() {
    if (!live() || sending || events.isEmpty()) return;
    sending = true;
    int generation = epoch;
    String id = session;
    long nextSequence = sequence;
    byte[] sessionKey = key.clone(),
        currentChallenge = challenge.clone(),
        noncePrefix = prefix.clone(),
        chain = previous.clone();
    PrivateKey signer = signingKey;
    ArrayList<BehaviorProtocol.Event> batch = new ArrayList<>(events);
    events.clear();
    int lost = dropped;
    dropped = 0;
    View decor = activity.getWindow().getDecorView();
    float density = activity.getResources().getDisplayMetrics().density;
    int width = Math.min(65535, Math.round(decor.getWidth() / density));
    int height = Math.min(65535, Math.round(decor.getHeight() / density));
    new Thread(
            () -> {
              String stage = "encode";
              try {
                byte[] plain =
                    BehaviorProtocol.payload(
                        System.currentTimeMillis(), width, height, lost, batch);
                stage = "encrypt";
                BehaviorProtocol.Frame frame =
                    BehaviorProtocol.seal(
                        UUID.fromString(id),
                        nextSequence,
                        currentChallenge,
                        noncePrefix,
                        chain,
                        sessionKey,
                        plain,
                        signer);
                Arrays.fill(plain, (byte) 0);
                activity.runOnUiThread(
                    () -> {
                      if (current(generation) && session.equals(id))
                        upload(frame, generation, id, nextSequence, false);
                    });
              } catch (Exception | LinkageError error) {
                String failedStage = stage;
                activity.runOnUiThread(
                    () -> {
                      if (current(generation)) fail(failedStage, error);
                    });
              } finally {
                Arrays.fill(sessionKey, (byte) 0);
              }
            },
            "behavior-encrypt")
        .start();
  }

  private void upload(
      BehaviorProtocol.Frame frame, int generation, String id, long sentSequence, boolean retried) {
    api.binary(
        "/behavior/batches",
        frame.bytes,
        value -> {
          if (!current(generation) || !session.equals(id)) return;
          try {
            JSONObject ack = (JSONObject) value;
            if (!ack.getString("session_id").equals(id)
                || ack.getLong("seq") != sentSequence
                || !ack.getString("packet_hash").equals(BehaviorProtocol.hex(frame.hash)))
              throw new IllegalArgumentException("Behavior acknowledgement mismatch");
            byte[] next = Base64.decode(ack.getString("next_challenge"), Base64.DEFAULT);
            if (next.length != 32)
              throw new IllegalArgumentException("Invalid behavior acknowledgement challenge");
            previous = frame.hash;
            challenge = next;
            sequence = sentSequence + 1;
            sending = false;
            lastFlush = SystemClock.elapsedRealtime();
          } catch (Exception error) {
            fail("upload", error);
          }
        },
        new Api.Failure() {
          @Override
          public void failed(String message) {
            failed(0, message);
          }

          @Override
          public void failed(int status, String message) {
            if (!current(generation) || !session.equals(id)) return;
            if (!retried && (status == 0 || status >= 500))
              upload(frame, generation, id, sentSequence, true);
            else
              fail(
                  "upload",
                  new IllegalStateException(
                      status == 0 ? message : "HTTP " + status + ": " + message));
          }
        });
  }

  private Api.Failure failure(String stage) {
    int generation = epoch;
    return new Api.Failure() {
      @Override
      public void failed(String message) {
        if (current(generation)) fail(stage, new IllegalStateException(message));
      }

      @Override
      public void failed(int status, String message) {
        failed("HTTP " + status + ": " + message);
      }
    };
  }

  private void fail(String stage, Throwable error) {
    if (destroyed || owner.isEmpty() || !owner.equals(api.token)) return;
    Log.e("BehaviorAudit", "Behavior " + stage + " failed; session=" + session, error);
    stopped = true;
    preparing = false;
    sending = false;
    handler.removeCallbacks(tick);
    String detail = DeviceTrustClient.detail(error, owner);
    if (key != null)
      detail = detail.replace(Base64.encodeToString(key, Base64.NO_WRAP), "[redacted]");
    api.post(
        "/behavior/failure",
        Api.object(
            "session_id",
            session,
            "stage",
            stage,
            "code",
            error.getClass().getSimpleName(),
            "detail",
            detail),
        ignored -> {},
        message -> Log.e("BehaviorAudit", "Failure record not delivered: " + message));
  }

  private void clearSession() {
    epoch++;
    handler.removeCallbacks(tick);
    if (key != null) Arrays.fill(key, (byte) 0);
    key = null;
    prefix = null;
    challenge = null;
    previous = new byte[32];
    signingKey = null;
    session = "";
    events.clear();
    dropped = 0;
    sending = false;
    preparing = false;
  }

  void reset() {
    clearSession();
    owner = "";
    enabled = false;
    stopped = false;
    overlays.clear();
    externalScreen = 0;
    touch = null;
    downTime = 0;
  }

  void destroy() {
    reset();
    destroyed = true;
  }
}
