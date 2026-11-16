package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.SharedPreferences;
import android.net.Uri;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import org.json.*;

final class Api {
  interface Result {
    void done(Object value);
  }

  interface Failure {
    void failed(String message);

    default void failed(int status, String message) {
      failed(message);
    }

    default void failed(int status, String message, String code) {
      failed(status, message);
    }
  }

  final Activity activity;
  final SharedPreferences preferences;
  final DeviceTrustClient deviceTrust;
  final BehaviorAudit behavior;
  boolean configurationLoaded;
  boolean deviceTrustEnabled;
  boolean updateRequired;
  String token;

  Api(Activity activity) {
    this.activity = activity;
    preferences = activity.getSharedPreferences("account", 0);
    token = preferences.getString("token", "");
    deviceTrust = new DeviceTrustClient(activity, this);
    behavior = new BehaviorAudit(activity, this);
  }

  void saveToken(String value) {
    if (!value.equals(token)) behavior.reset();
    token = value;
    preferences.edit().putString("token", value).apply();
  }

  static JSONObject object(Object... pairs) {
    JSONObject j = new JSONObject();
    try {
      for (int i = 0; i < pairs.length; i += 2) j.put((String) pairs[i], pairs[i + 1]);
    } catch (JSONException e) {
      throw new IllegalArgumentException(e);
    }
    return j;
  }

  void get(String path, Result result, Failure failure) {
    request("GET", path, null, result, failure);
  }

  void productClick(String productId, String kind) {
    post(
        "/products/" + Uri.encode(productId) + "/click",
        object("kind", kind),
        ignored -> {},
        message ->
            android.util.Log.e(
                "ProductClick",
                "POST /api/products/" + productId + "/click kind=" + kind + ": " + message));
  }

  void post(String path, JSONObject body, Result result, Failure failure) {
    String kind =
        path.equals("/rewards/start")
            ? "reward"
            : path.equals("/check-in")
                ? "check_in"
                : path.equals("/withdrawals")
                    ? "withdraw"
                    : path.equals("/rain/join") ? "rain" : null;
    if (updateRequired && kind != null) {
      failure.failed("请先更新至最新版本。");
      return;
    }
    if (kind != null && !configurationLoaded) {
      failure.failed("服务配置尚未加载，请稍后再试。");
      return;
    }
    if (kind != null && deviceTrustEnabled)
      deviceTrust.assess(
          kind,
          object("method", "POST", "path", "/api" + path, "body", body),
          eventId -> request("POST", path, body, result, failure, eventId),
          failure);
    else request("POST", path, body, result, failure);
  }

  void quoteReward(String taskId, Result result, Failure failure) {
    if (!configurationLoaded) {
      failure.failed("服务配置尚未加载，请稍后再试。");
      return;
    }
    if (deviceTrustEnabled) deviceTrust.quoteReward(taskId, result, failure);
    else post("/rewards/quote", object("task_id", taskId), result, failure);
  }

  void reportLogin(Failure failure) {
    deviceTrust.reportStartupFailure();
    if (configurationLoaded && deviceTrustEnabled)
      deviceTrust.assess(
          "login",
          object("method", "POST", "path", "/api/auth/login/report", "body", object()),
          eventId -> behavior.begin(),
          message -> {
            behavior.loginUnavailable();
            failure.failed(message);
          });
    else behavior.begin();
  }

  void requestWithRisk(String path, JSONObject body, String event, Result result, Failure failure) {
    request("POST", path, body, result, failure, event);
  }

  void binary(String path, byte[] body, Result result, Failure failure) {
    send("POST", path, body, "application/octet-stream", result, failure, "");
  }

  void request(String method, String path, JSONObject body, Result result, Failure failure) {
    request(method, path, body, result, failure, "");
  }

  private void request(
      String method,
      String path,
      JSONObject body,
      Result result,
      Failure failure,
      String riskEvent) {
    if (updateRequired
        && method.equals("POST")
        && (path.equals("/rewards/start")
            || path.equals("/check-in")
            || path.equals("/withdrawals")
            || path.equals("/rain/join")
            || path.equals("/rain/claim"))) {
      failure.failed("请先更新至最新版本。");
      return;
    }
    send(
        method,
        path,
        body == null ? null : body.toString().getBytes(StandardCharsets.UTF_8),
        "application/json",
        result,
        failure,
        riskEvent);
  }

  private void send(
      String method,
      String path,
      byte[] body,
      String contentType,
      Result result,
      Failure failure,
      String riskEvent) {
    String requestToken = token;
    new Thread(
            () -> {
              HttpURLConnection c = null;
              try {
                c =
                    (HttpURLConnection)
                        new URL(BuildConfig.API_BASE_URL + "/api" + path).openConnection();
                c.setRequestMethod(method);
                c.setUseCaches(false);
                c.setConnectTimeout(7000);
                c.setReadTimeout(12000);
                c.setRequestProperty("Accept", "application/json");
                if (!riskEvent.isEmpty()) c.setRequestProperty("x-risk-event", riskEvent);
                if (!requestToken.isEmpty())
                  c.setRequestProperty("Authorization", "Bearer " + requestToken);
                if (body != null) {
                  c.setDoOutput(true);
                  c.setRequestProperty("Content-Type", contentType);
                  try (OutputStream out = c.getOutputStream()) {
                    out.write(body);
                  }
                }
                int status = c.getResponseCode();
                InputStream stream = status < 400 ? c.getInputStream() : c.getErrorStream();
                ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                byte[] buffer = new byte[4096];
                int read;
                try (InputStream input = stream) {
                  while ((read = input.read(buffer)) != -1) bytes.write(buffer, 0, read);
                }
                String raw = bytes.toString("UTF-8");
                if (status >= 400) {
                  String message = raw;
                  String errorCode = "";
                  try {
                    JSONObject error = new JSONObject(raw);
                    message = error.optString("error", raw);
                    errorCode = error.optString("code", "");
                  } catch (JSONException ignored) {
                    // Keep the upstream response visible when it is not JSON.
                  }
                  final String finalMessage = message;
                  final String finalCode = errorCode;
                  activity.runOnUiThread(() -> failure.failed(status, finalMessage, finalCode));
                  return;
                }
                Object value = new JSONTokener(raw).nextValue();
                activity.runOnUiThread(
                    () -> {
                      if (requestToken.equals(token)
                          && method.equals("POST")
                          && value instanceof JSONObject)
                        behavior.response(path, (JSONObject) value);
                      result.done(value);
                    });
              } catch (Exception e) {
                String message = e.getMessage();
                activity.runOnUiThread(() -> failure.failed(message));
              } finally {
                if (c != null) c.disconnect();
              }
            },
            "http-" + path)
        .start();
  }
}
