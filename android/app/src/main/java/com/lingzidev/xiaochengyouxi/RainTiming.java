package com.lingzidev.xiaochengyouxi;

import java.text.ParseException;
import java.text.SimpleDateFormat;
import java.util.Locale;
import java.util.TimeZone;

final class RainTiming {
  static long parse(String value) {
    SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.ROOT);
    format.setTimeZone(TimeZone.getTimeZone("UTC"));
    format.setLenient(false);
    try {
      return format.parse(value).getTime();
    } catch (ParseException error) {
      throw new IllegalArgumentException("红包雨时间格式错误：" + value, error);
    }
  }

  static String remaining(long milliseconds) {
    long seconds = Math.max(0, (milliseconds + 999) / 1000);
    return String.format(
        Locale.ROOT, "%02d:%02d:%02d", seconds / 3600, seconds / 60 % 60, seconds % 60);
  }

  static float progress(long reveal, long expires, long now) {
    if (expires <= reveal) throw new IllegalArgumentException("红包可领取窗口错误");
    return Math.max(0, Math.min(1, (now - reveal) / (float) (expires - reveal)));
  }
}
