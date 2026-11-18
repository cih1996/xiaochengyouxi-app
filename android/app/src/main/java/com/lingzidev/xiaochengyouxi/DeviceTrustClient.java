package com.lingzidev.xiaochengyouxi;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.pm.PackageManager;
import com.device.trust.EnvironmentCollector;
import com.device.trust.DeviceTrust;
import com.device.trust.secure.ResourceCollector;
import com.device.trust.secure.SecureReporter;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

final class DeviceTrustClient {
  static final String CONSENT = "device_trust_consent_v4";
  private final Activity activity;
  private final Api api;
  private boolean busy;
  // Checks run one at a time; later requests wait instead of failing (e.g. tapping 签到 while the
  // first-login check is still uploading).
  private final java.util.ArrayDeque<Attempt> waiting = new java.util.ArrayDeque<>();
  private String startupError = "", startupCode = "";
  // Registration parameters from /device/config, kept for re-registering after a key reset.
  private String sdkBase = "";
  private String[] sdkPins;

  interface Ready {
    void done(String eventId);
  }

  /** Re-register when the app returns to the foreground and the last report is this old. */
  private static final long REFRESH_AFTER_MS = 30 * 60 * 1000L;
  private volatile long lastReportAt;
  private volatile boolean refreshing;

  /**
   * Cell towers and location (DeviceTrust environment module). Android only exposes the cell list
   * with location permission; without it nothing is read and the platform leaves those rules out
   * of scoring.
   */
  static boolean locationGranted(Context context) {
    return context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION)
            == PackageManager.PERMISSION_GRANTED
        || context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION)
            == PackageManager.PERMISSION_GRANTED;
  }

  private void collectEnvironment(Context context, JSONObject report, boolean consent) {
    if (!consent || !locationGranted(context)) return;
    try {
      EnvironmentCollector.collect(
          context,
          report,
          true,
          EnvironmentCollector.Collection.CELL,
          EnvironmentCollector.Collection.LOCATION);
    } catch (Exception | LinkageError error) {
      // Collection failure is not a risk signal; the report goes out without it.
      android.util.Log.w("DeviceTrust", "environment collection failed", error);
    }
  }

  private com.device.trust.secure.DeviceTrustClient.ExtraCollector extra() {
    boolean consent = api.preferences.getBoolean(CONSENT, false);
    return (context, report) -> collectEnvironment(context, report, consent);
  }

  /**
   * Called from onResume. The platform judges the device environment by its latest report, so
   * a fresh one after 30 minutes lets the score recover (e.g. USB debugging switched off).
   */
  /** Location was just granted: send a report that includes cell/location right away. */
  void refreshNow() {
    lastReportAt = 0;
    refreshIfStale();
  }

  void refreshIfStale() {
    if (refreshing
        || sdkPins == null
        || sdkBase.isEmpty()
        || !api.preferences.getBoolean(CONSENT, false)
        || System.currentTimeMillis() - lastReportAt < REFRESH_AFTER_MS) return;
    refreshing = true;
    new Thread(
            () -> {
              try {
                com.device.trust.secure.DeviceTrustClient.initialize(
                    activity.getApplicationContext(),
                    new com.device.trust.secure.DeviceTrustClient.Config(
                        BuildConfig.DEVICE_TRUST_APP_ID, sdkBase, sdkPins),
                    true,
                    extra());
                lastReportAt = System.currentTimeMillis();
              } catch (Exception | LinkageError error) {
                if (keyInvalidated(error)) resetKey();
                android.util.Log.w("DeviceTrust", "foreground refresh failed", error);
              } finally {
                refreshing = false;
              }
            },
            "device-trust-refresh")
        .start();
  }

  DeviceTrustClient(Activity activity, Api api) {
    this.activity = activity;
    this.api = api;
  }

  void initialize(Runnable done) {
    api.get(
        "/device/config",
        result -> {
          try {
            JSONObject config = (JSONObject) result;
            if (!config.getBoolean("enabled")) {
              done.run();
              return;
            }
            String appId = config.getString("app_id");
            if (!appId.equals(BuildConfig.DEVICE_TRUST_APP_ID))
              throw new IllegalArgumentException("DeviceTrust application mismatch");
            JSONArray list = config.getJSONArray("spki_pins");
            String[] pins = new String[list.length()];
            for (int i = 0; i < pins.length; i++) pins[i] = list.getString(i);
            String base = config.getString("business_url");
            sdkBase = base;
            sdkPins = pins;
            // Startup never waits on the network for long: a slow DNS/registration must not keep
            // the user on the splash screen. After 2s the App continues and registration finishes
            // in the background (business checks still attach their own fresh report).
            final java.util.concurrent.atomic.AtomicBoolean proceeded = new java.util.concurrent.atomic.AtomicBoolean();
            Runnable proceed =
                () -> {
                  if (proceeded.compareAndSet(false, true) && !activity.isDestroyed()) done.run();
                };
            new android.os.Handler(android.os.Looper.getMainLooper()).postDelayed(proceed, 2000);
            new Thread(
                    () -> {
                      try {
                        com.device.trust.secure.DeviceTrustClient.initialize(
                            activity.getApplicationContext(),
                            new com.device.trust.secure.DeviceTrustClient.Config(appId, base, pins),
                            api.preferences.getBoolean(CONSENT, false),
                            extra());
                        lastReportAt = System.currentTimeMillis();
                      } catch (Exception | LinkageError error) {
                        startupCode = code(error);
                        startupError = detail(error, api.token);
                        android.util.Log.e(
                            "DeviceTrust", "SDK 0.7 startup registration failed", error);
                      }
                      activity.runOnUiThread(proceed);
                    },
                    "device-trust-initialize")
                .start();
          } catch (Exception error) {
            startupCode = code(error);
            startupError = detail(error, api.token);
            android.util.Log.e("DeviceTrust", "SDK startup configuration failed", error);
            done.run();
          }
        },
        message -> {
          startupCode = "StartupConfigurationError";
          startupError =
              message == null
                  ? startupCode
                  : message.replace(api.token.isEmpty() ? "\u0000" : api.token, "[redacted]");
          android.util.Log.e("DeviceTrust", "GET /device/config: " + startupError);
          done.run();
        });
  }

  /**
   * Android permanently invalidated the SDK's Keystore key (e.g. the lock screen was changed).
   * That key can never sign again, so this one case — and only this one — resets the key and
   * registers a new one; every other key error is still reported as is.
   */
  static boolean keyInvalidated(Throwable error) {
    for (Throwable e = error; e != null; e = e.getCause()) {
      if (e.getClass().getSimpleName().equals("KeyPermanentlyInvalidatedException")) return true;
      String text = String.valueOf(e.getMessage());
      if (text.contains("KeyPermanentlyInvalidatedException") || text.contains("Key permanently invalidated"))
        return true;
      if (e.getCause() == e) break;
    }
    return false;
  }

  /** Runs on a worker thread. Returns true when a fresh key is registered. */
  private static final String KEY_RESETS = "device_trust_key_resets";

  private synchronized boolean resetKey() {
    if (sdkPins == null || sdkBase.isEmpty()) return false;
    // At most twice per 24 hours per installation (kept across restarts): an environment that
    // keeps invalidating keys (sandboxes, cloud phones) must not be able to rotate keys freely.
    long now = System.currentTimeMillis();
    StringBuilder kept = new StringBuilder();
    int recent = 0;
    for (String part : api.preferences.getString(KEY_RESETS, "").split(",")) {
      try {
        long at = Long.parseLong(part.trim());
        if (now - at < 86_400_000L) {
          recent++;
          kept.append(at).append(',');
        }
      } catch (NumberFormatException ignored) {
      }
    }
    if (recent >= 2) return false;
    api.preferences.edit().putString(KEY_RESETS, kept.append(now).toString()).apply();
    try {
      SecureReporter.reset(activity.getApplicationContext(), BuildConfig.DEVICE_TRUST_APP_ID);
      com.device.trust.secure.DeviceTrustClient.initialize(
          activity.getApplicationContext(),
          new com.device.trust.secure.DeviceTrustClient.Config(
              BuildConfig.DEVICE_TRUST_APP_ID, sdkBase, sdkPins),
          api.preferences.getBoolean(CONSENT, false),
          extra());
      lastReportAt = System.currentTimeMillis();
      return true;
    } catch (Exception | LinkageError error) {
      android.util.Log.e("DeviceTrust", "key reset failed", error);
      return false;
    }
  }

  void reportStartupFailure() {
    if (startupError.isEmpty() || api.token.isEmpty()) return;
    String error = startupError, errorCode = startupCode;
    startupError = "";
    reportFailure("", "login", "initialize", errorCode, error, ignored -> {});
  }

  void assess(String kind, JSONObject intent, Ready ready, Api.Failure failure) {
    if (!api.preferences.getBoolean(CONSENT, false)) {
      failure.failed("请先阅读并同意隐私政策。");
      return;
    }
    Attempt attempt = new Attempt(kind, intent, api.token, ready, failure);
    if (busy) {
      if (waiting.size() >= 4) {
        failure.failed("操作有点频繁，请稍后再试。");
        return;
      }
      waiting.add(attempt);
      return;
    }
    busy = true;
    attempt.start();
  }

  /** Marks the current check finished and starts the next queued one, if still relevant. */
  private void next() {
    busy = false;
    while (!waiting.isEmpty()) {
      Attempt attempt = waiting.poll();
      if (activity.isDestroyed() || !attempt.token.equals(api.token)) continue;
      busy = true;
      attempt.start();
      return;
    }
  }

  void quoteReward(String taskId, Api.Result result, Api.Failure failure) {
    if (!api.preferences.getBoolean(CONSENT, false)) {
      failure.failed("请先阅读并同意隐私政策。");
      return;
    }
    String token = api.token;
    new Thread(
            () -> {
              try {
                String keyId =
                    SecureReporter.keyId(
                        activity.getApplicationContext(), BuildConfig.DEVICE_TRUST_APP_ID);
                activity.runOnUiThread(
                    () -> {
                      if (!activity.isDestroyed() && token.equals(api.token))
                        api.post(
                            "/rewards/quote",
                            Api.object("task_id", taskId, "key_id", keyId),
                            result,
                            failure);
                    });
              } catch (Exception | LinkageError error) {
                if (keyInvalidated(error) && resetKey()) {
                  activity.runOnUiThread(
                      () -> {
                        if (activity.isDestroyed() || !token.equals(api.token)) return;
                        reportFailure("", "reward", "key_reset", "KeyPermanentlyInvalidated", "", ignored -> {});
                        quoteReward(taskId, result, failure);
                      });
                  return;
                }
                activity.runOnUiThread(
                    () -> {
                      if (!activity.isDestroyed() && token.equals(api.token))
                        reportFailure(
                            "",
                            "reward",
                            "quote_key_id",
                            code(error),
                            detail(error, token),
                            message -> {
                              if (!activity.isDestroyed() && token.equals(api.token))
                                failure.failed(message);
                            });
                    });
              }
            },
            "device-trust-quote-key")
        .start();
  }

  private final class Attempt {
    final String kind, token;
    final JSONObject intent;
    final Ready ready;
    final Api.Failure failure;
    String eventId = "";
    boolean keyReset;

    Attempt(String kind, JSONObject intent, String token, Ready ready, Api.Failure failure) {
      this.kind = kind;
      this.intent = intent;
      this.token = token;
      this.ready = ready;
      this.failure = failure;
    }

    boolean active() {
      if (activity.isDestroyed() || !token.equals(api.token)) {
        next();
        return false;
      }
      return true;
    }

    void start() {
      new Thread(
              () -> {
                try {
                  String keyId =
                      SecureReporter.keyId(
                          activity.getApplicationContext(), BuildConfig.DEVICE_TRUST_APP_ID);
                  activity.runOnUiThread(
                      () -> {
                        if (active()) prepare(keyId);
                      });
                } catch (Exception | LinkageError error) {
                  if (recoverKey(error)) return;
                  activity.runOnUiThread(
                      () -> {
                        if (active()) recordFailure("key_id", code(error), detail(error, token));
                      });
                }
              },
              "device-trust-key")
          .start();
    }

    void prepare(String keyId) {
      api.post(
          "/risk/prepare",
          Api.object("kind", kind, "key_id", keyId, "intent", intent),
          result -> {
            if (!active()) return;
            try {
              JSONObject prepared = (JSONObject) result;
              eventId = prepared.getString("event_id");
              upload(prepared.getJSONObject("sdk"), prepared.getJSONObject("challenge"));
            } catch (JSONException error) {
              recordFailure("prepare", code(error));
            }
          },
          httpFailure("prepare"));
    }

    void upload(JSONObject sdk, JSONObject challenge) {
      new Thread(
              () -> {
                String stage = "sdk_config";
                try {
                  String appId = sdk.getString("app_id"), base = sdk.getString("base_url");
                  JSONArray configuredPins = sdk.getJSONArray("spki_pins");
                  String[] pins = new String[configuredPins.length()];
                  for (int i = 0; i < pins.length; i++) pins[i] = configuredPins.getString(i);
                  stage = "collect";
                  JSONObject report =
                      DeviceTrust.collect(
                          activity.getApplicationContext(),
                          new DeviceTrust.Options(appId, true),
                          api.preferences.getBoolean(CONSENT, false));
                  stage = "resources";
                  ResourceCollector.collect(
                      activity.getApplicationContext(),
                      report,
                      api.preferences.getBoolean(CONSENT, false));
                  collectEnvironment(
                      activity.getApplicationContext(),
                      report,
                      api.preferences.getBoolean(CONSENT, false));
                  stage = "upload";
                  JSONObject uploaded =
                      SecureReporter.upload(
                          activity.getApplicationContext(), base, pins, challenge, report);
                  String reportId = uploaded.getString("reportId");
                  lastReportAt = System.currentTimeMillis();
                  activity.runOnUiThread(
                      () -> {
                        if (active()) complete(reportId);
                      });
                } catch (Exception | LinkageError error) {
                  if (recoverKey(error)) return;
                  String failedStage = stage;
                  activity.runOnUiThread(
                      () -> {
                        if (active()) recordFailure(failedStage, code(error), detail(error, token));
                      });
                }
              },
              "device-trust-upload")
          .start();
    }

    void complete(String reportId) {
      api.post(
          "/risk/complete",
          Api.object("event_id", eventId, "report_id", reportId),
          result -> {
            if (!active()) return;
            JSONObject response = (JSONObject) result;
            String status = response.optString("status"),
                reference = response.optString("reference", eventId);
            if (status.equals("recorded")) {
              next();
              ready.done(eventId);
            } else if (status.equals("review") || status.equals("restricted")) {
              next();
              failure.failed(message(blocked(kind), reference));
            } else recordFailure("complete", "InvalidBusinessStatus");
          },
          httpFailure("complete"));
    }

    /**
     * On a worker thread: if the key was permanently invalidated (first time for this attempt),
     * note it on the current event, register a new key and start the attempt over.
     */
    boolean recoverKey(Throwable error) {
      if (keyReset || !keyInvalidated(error)) return false;
      keyReset = true;
      String previous = eventId;
      if (!resetKey()) return false;
      activity.runOnUiThread(
          () -> {
            if (!active()) return;
            // Visible in the admin as an assessment error with stage key_reset.
            reportFailure(previous, kind, "key_reset", "KeyPermanentlyInvalidated", "", ignored -> {});
            eventId = "";
            start();
          });
      return true;
    }

    Api.Failure httpFailure(String stage) {
      return new Api.Failure() {
        @Override
        public void failed(String message) {
          if (active()) recordFailure(stage, "NetworkError");
        }

        @Override
        public void failed(int status, String message) {
          if (active()) recordFailure(stage, "HTTP_" + status);
        }
      };
    }

    void recordFailure(String stage, String errorCode) {
      recordFailure(stage, errorCode, "");
    }

    void recordFailure(String stage, String errorCode, String detail) {
      reportFailure(
          eventId,
          kind,
          stage,
          errorCode,
          detail,
          message -> {
            if (!active()) return;
            next();
            failure.failed(message);
          });
    }
  }

  private void reportFailure(
      String eventId,
      String kind,
      String stage,
      String errorCode,
      String detail,
      Api.Failure failure) {
    api.post(
        "/risk/failure",
        Api.object(
            "event_id", eventId, "kind", kind, "stage", stage, "code", errorCode, "detail", detail),
        result -> {
          JSONObject response = (JSONObject) result;
          failure.failed(message("服务暂时繁忙，请稍后再试。", response.optString("reference", eventId)));
        },
        ignored -> failure.failed(message("服务暂时繁忙，请稍后再试。", eventId)));
  }

  private static String code(Throwable error) {
    String text = error.getMessage();
    Matcher http = Pattern.compile("^HTTP\\s+(\\d{3})").matcher(text == null ? "" : text);
    return http.find() ? "HTTP_" + http.group(1) : error.getClass().getSimpleName();
  }

  static String detail(Throwable error, String token) {
    StringBuilder text = new StringBuilder();
    Throwable cause = error;
    for (int depth = 0; cause != null && depth < 4; depth++) {
      if (depth > 0) text.append(" <- ");
      text.append(cause.getClass().getSimpleName());
      String message = cause instanceof JSONException ? null : cause.getMessage();
      if (message != null) {
        if (!token.isEmpty()) message = message.replace(token, "[redacted]");
        Matcher http = Pattern.compile("^HTTP\\s+(\\d{3})").matcher(message);
        if (http.find()) message = "HTTP " + http.group(1);
        message = message.replace('\n', ' ').replace('\r', ' ').replace('\t', ' ');
        text.append(": ").append(message, 0, Math.min(140, message.length()));
      }
      cause = cause.getCause();
    }
    return text.substring(0, Math.min(600, text.length()));
  }

  private static String message(String text, String reference) {
    return reference.isEmpty() ? text : text + "\n如需帮助请联系客服，参考号：" + reference;
  }

  /** Plain wording for an operation the risk check did not let through (no risk jargon). */
  private static String blocked(String kind) {
    switch (kind) {
      case "check_in":
        return "今天的签到暂时没能完成，请稍后再试。";
      case "reward":
        return "这个红包暂时领不了，请稍后再试。";
      case "withdraw":
        return "提现暂时无法提交，请稍后再试。";
      case "rain":
        return "这场红包雨暂时参与不了，请稍后再试。";
      default:
        return "这次操作暂时没能完成，请稍后再试。";
    }
  }
}
