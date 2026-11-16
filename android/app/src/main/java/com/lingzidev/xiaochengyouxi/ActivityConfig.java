package com.lingzidev.xiaochengyouxi;

import org.json.JSONObject;

/** Server-owned activity description. Unknown fields are preserved for forward compatibility. */
final class ActivityConfig {
  final JSONObject raw;
  final boolean enabled;
  final String title, subtitle, entryLabel, heroImage, spinPath;
  final int opportunities;
  final boolean canSpin;

  ActivityConfig(JSONObject raw) {
    this.raw = raw;
    enabled = raw.optBoolean("enabled") && activeWindow(raw);
    title = raw.optString("title", "每日活动");
    subtitle = raw.optString("subtitle", "完成任务，赢取活动奖励");
    entryLabel = raw.optString("entry_label", "参加活动");
    heroImage = raw.optString("hero_image", "");
    JSONObject wheel = raw.optJSONObject("wheel");
    spinPath = raw.optString("spin_path", wheel == null ? "" : wheel.optString("spin_path", ""));
    org.json.JSONArray available = raw.optJSONArray("opportunities");
    int count = 0;
    if (available != null)
      for (int i = 0; i < available.length(); i++) {
        JSONObject item = available.optJSONObject(i);
        if (item != null && !item.optBoolean("used", true)) count++;
      }
    opportunities = raw.optInt("opportunities_count", count);
    canSpin = raw.optBoolean("can_spin", false);
  }

  private static boolean activeWindow(JSONObject value) {
    long now = System.currentTimeMillis();
    String starts = value.optString("starts_at", ""), expires = value.optString("expires_at", "");
    try {
      if (!starts.isEmpty() && java.time.Instant.parse(starts).toEpochMilli() > now) return false;
      if (!expires.isEmpty() && java.time.Instant.parse(expires).toEpochMilli() <= now)
        return false;
    } catch (java.time.format.DateTimeParseException error) {
      return false;
    }
    return true;
  }
}
