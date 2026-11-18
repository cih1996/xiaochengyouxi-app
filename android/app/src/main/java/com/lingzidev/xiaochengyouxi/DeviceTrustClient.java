package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
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

  interface Ready {
    void done(String eventId);
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
            new Thread(
                    () -> {
                      try {
                        com.device.trust.secure.DeviceTrustClient.initialize(
                            activity.getApplicationContext(),
                            new com.device.trust.secure.DeviceTrustClient.Config(appId, base, pins),
                            api.preferences.getBoolean(CONSENT, false),
                            null);
                      } catch (Exception | LinkageError error) {
                        startupCode = code(error);
                        startupError = detail(error, api.token);
                        android.util.Log.e(
                            "DeviceTrust", "SDK 0.7 startup registration failed", error);
                      }
                      activity.runOnUiThread(
                          () -> {
                            if (!activity.isDestroyed()) done.run();
                          });
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
                  stage = "upload";
                  JSONObject uploaded =
                      SecureReporter.upload(
                          activity.getApplicationContext(), base, pins, challenge, report);
                  String reportId = uploaded.getString("reportId");
                  activity.runOnUiThread(
                      () -> {
                        if (active()) complete(reportId);
                      });
                } catch (Exception | LinkageError error) {
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
