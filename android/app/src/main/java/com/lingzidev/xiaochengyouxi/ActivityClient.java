package com.lingzidev.xiaochengyouxi;

import android.util.Log;
import org.json.JSONObject;

/** Loads server-configured activities; endpoint paths and wheel rules come from server data. */
final class ActivityClient {
  private final Api api;
  ActivityConfig config;
  private String configPath = "";
  private String token = "";
  Runnable onChanged;

  ActivityClient(Api api) {
    this.api = api;
  }

  void configure(JSONObject appConfig) {
    configPath = "/campaign";
    load();
  }

  void load() {
    if (configPath.isEmpty() || api.token.isEmpty()) return;
    token = api.token;
    api.get(
        configPath,
        value -> {
          if (!token.equals(api.token) || !(value instanceof JSONObject)) return;
          config = new ActivityConfig((JSONObject) value);
          if (!config.enabled) config = null;
          if (onChanged != null) onChanged.run();
        },
        message -> Log.e("ActivityClient", "GET " + configPath + ": " + message));
  }

  void clear() {
    config = null;
    token = "";
  }

  void spin(ActivityResult result, Api.Failure failure) {
    if (config == null || config.spinPath.isEmpty()) {
      failure.failed("当前活动暂未开放转盘");
      return;
    }
    String opportunity = "";
    for (org.json.JSONObject item : JsonObjects(config.raw.optJSONArray("opportunities"))) {
      if (!item.optBoolean("used", true)) {
        opportunity = item.optString("id");
        break;
      }
    }
    if (opportunity.isEmpty()) {
      failure.failed("当前没有可用机会");
      return;
    }
    api.post(
        normalize(config.spinPath),
        Api.object("opportunity_id", opportunity),
        result::done,
        failure);
  }

  void settle(String id, ActivityResult result, Api.Failure failure) {
    if (id == null || id.isEmpty()) {
      failure.failed("活动奖励编号无效");
      return;
    }
    api.post(normalize("/campaign/rewards/" + id + "/settle"), Api.object(), result::done, failure);
  }

  private static String normalize(String path) {
    return path.startsWith("/api/") ? path.substring(4) : path;
  }

  private static java.util.List<org.json.JSONObject> JsonObjects(org.json.JSONArray array) {
    java.util.ArrayList<org.json.JSONObject> result = new java.util.ArrayList<>();
    if (array == null) return result;
    for (int i = 0; i < array.length(); i++)
      if (array.optJSONObject(i) != null) result.add(array.optJSONObject(i));
    return result;
  }

  interface ActivityResult {
    void done(Object value);
  }
}
