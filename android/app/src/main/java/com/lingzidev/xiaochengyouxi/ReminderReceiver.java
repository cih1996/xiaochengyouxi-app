package com.lingzidev.xiaochengyouxi;

import android.Manifest;
import android.app.AlarmManager;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import java.util.Calendar;
import java.util.TimeZone;
import org.json.JSONObject;

/**
 * Local reminders in the notification bar (no push service). MainActivity schedules them when the
 * App goes to the background and cancels them when it comes back; this receiver shows them.
 * Config (times, texts, switches, quiet hours) comes from the server: GET /api/app/reminders.
 */
public final class ReminderReceiver extends BroadcastReceiver {
  static final String CHANNEL = "reminders";
  static final String EXTRA_TARGET = "reminder_target";
  private static final String[] KINDS = {"rain", "checkin", "wheel"};
  private static final TimeZone BJ = TimeZone.getTimeZone("Asia/Shanghai");

  static boolean allowed(Context c) {
    if (!c.getSharedPreferences("account", 0).getBoolean("reminders_on", true)) return false;
    if (Build.VERSION.SDK_INT >= 33
        && c.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED)
      return false;
    return ((NotificationManager) c.getSystemService(Context.NOTIFICATION_SERVICE)).areNotificationsEnabled();
  }

  private static PendingIntent pending(Context c, String kind, String title, String body, boolean create) {
    return pending(c, kind, title, body, 0, create);
  }

  private static PendingIntent pending(Context c, String kind, String title, String body, long eventAt, boolean create) {
    Intent i = new Intent(c, ReminderReceiver.class).setAction("reminder." + kind)
        .putExtra("kind", kind).putExtra("title", title).putExtra("body", body).putExtra("event_at", eventAt);
    int flags = PendingIntent.FLAG_IMMUTABLE | (create ? PendingIntent.FLAG_UPDATE_CURRENT : PendingIntent.FLAG_NO_CREATE);
    return PendingIntent.getBroadcast(c, kind.hashCode(), i, flags);
  }

  static void cancelAll(Context c) {
    AlarmManager am = (AlarmManager) c.getSystemService(Context.ALARM_SERVICE);
    for (String k : KINDS) {
      PendingIntent p = pending(c, k, "", "", false);
      if (p != null) {
        am.cancel(p);
        p.cancel();
      }
    }
  }

  /** Minutes since midnight Beijing time for "HH:MM". */
  private static int minutes(String hhmm) {
    String[] p = hhmm.split(":");
    return Integer.parseInt(p[0]) * 60 + Integer.parseInt(p[1]);
  }

  static boolean quiet(JSONObject cfg, long at) {
    Calendar cal = Calendar.getInstance(BJ);
    cal.setTimeInMillis(at);
    int m = cal.get(Calendar.HOUR_OF_DAY) * 60 + cal.get(Calendar.MINUTE);
    int s = minutes(cfg.optString("quiet_start", "22:30")), e = minutes(cfg.optString("quiet_end", "08:00"));
    return s <= e ? (m >= s && m < e) : (m >= s || m < e);
  }

  /** Next time (ms) the clock shows hh:mm in Beijing; tomorrow if `skipToday`. */
  private static long nextAt(String hhmm, boolean skipToday, long now) {
    Calendar cal = Calendar.getInstance(BJ);
    cal.setTimeInMillis(now);
    int m = minutes(hhmm);
    cal.set(Calendar.HOUR_OF_DAY, m / 60);
    cal.set(Calendar.MINUTE, m % 60);
    cal.set(Calendar.SECOND, 0);
    cal.set(Calendar.MILLISECOND, 0);
    if (skipToday || cal.getTimeInMillis() <= now + 60_000) cal.add(Calendar.DAY_OF_YEAR, 1);
    return cal.getTimeInMillis();
  }

  /**
   * Fires somewhere in [from, from + window]. A window alarm needs no exact-alarm permission
   * (which Android 14 denies by default) and is far tighter than a plain inexact one (up to 1h).
   */
  private static void at(Context c, String kind, long from, long window, String title, String body, long eventAt) {
    AlarmManager am = (AlarmManager) c.getSystemService(Context.ALARM_SERVICE);
    am.setWindow(AlarmManager.RTC_WAKEUP, from, window, pending(c, kind, title, body, eventAt, true));
  }

  /**
   * @param nextRainMs next red-packet rain start (0 if unknown)
   * @param checkedInToday whether today's check-in is done
   * @param wheelOn whether the lucky wheel runs and has spins left today
   */
  static void schedule(Context c, JSONObject cfg, long nextRainMs, boolean checkedInToday, boolean wheelOn) {
    cancelAll(c);
    if (cfg == null || !cfg.optBoolean("enabled") || !allowed(c)) return;
    long now = System.currentTimeMillis();
    try {
      if (cfg.optBoolean("rain_enabled") && nextRainMs > now) {
        // Between max(lead,10) and 1 minute before the start; the minutes in the text are
        // computed when it actually fires.
        int lead = Math.max(cfg.optInt("rain_lead_minutes", 3), 10);
        long from = Math.max(nextRainMs - lead * 60_000L, now + 30_000), window = nextRainMs - 60_000 - from;
        if (window > 0 && !quiet(cfg, from))
          at(c, "rain", from, window, cfg.optString("rain_title"), cfg.optString("rain_body"), nextRainMs);
      }
      if (cfg.optBoolean("checkin_enabled")) {
        long when = nextAt(cfg.optString("checkin_time", "20:00"), checkedInToday, now);
        if (!quiet(cfg, when)) at(c, "checkin", when, 15 * 60_000L, cfg.optString("checkin_title"), cfg.optString("checkin_body"), 0);
      }
      if (cfg.optBoolean("wheel_enabled") && wheelOn) {
        long when = nextAt(cfg.optString("wheel_time", "12:30"), false, now);
        if (!quiet(cfg, when)) at(c, "wheel", when, 15 * 60_000L, cfg.optString("wheel_title"), cfg.optString("wheel_body"), 0);
      }
    } catch (RuntimeException e) {
      android.util.Log.w("Reminders", "schedule failed", e);
    }
  }

  static void ensureChannel(Context c) {
    if (Build.VERSION.SDK_INT < 26) return;
    NotificationManager nm = (NotificationManager) c.getSystemService(Context.NOTIFICATION_SERVICE);
    if (nm.getNotificationChannel(CHANNEL) != null) return;
    NotificationChannel ch = new NotificationChannel(CHANNEL, "活动提醒", NotificationManager.IMPORTANCE_DEFAULT);
    ch.setDescription("红包雨开场、签到、幸运转盘等提醒");
    nm.createNotificationChannel(ch);
  }

  @Override
  public void onReceive(Context c, Intent intent) {
    if (!allowed(c)) return;
    String kind = intent.getStringExtra("kind"), title = intent.getStringExtra("title"), body = intent.getStringExtra("body");
    if (kind == null || title == null || title.isEmpty()) return;
    long eventAt = intent.getLongExtra("event_at", 0);
    if (eventAt > 0) {
      long left = eventAt - System.currentTimeMillis();
      if (left < 30_000) return; // the rain has started: too late to remind
      body = body == null ? "" : body.replace("{minutes}", String.valueOf(Math.max(1, (left + 59_999) / 60_000)));
    }
    ensureChannel(c);
    Intent open = new Intent(c, MainActivity.class)
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP | Intent.FLAG_ACTIVITY_SINGLE_TOP)
        .putExtra(EXTRA_TARGET, kind);
    PendingIntent tap = PendingIntent.getActivity(c, kind.hashCode(), open, PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);
    Notification.Builder b = Build.VERSION.SDK_INT >= 26 ? new Notification.Builder(c, CHANNEL) : new Notification.Builder(c);
    b.setSmallIcon(R.drawable.ic_notify).setColor(0xfff65331).setContentTitle(title).setContentText(body)
        .setAutoCancel(true).setContentIntent(tap).setShowWhen(true);
    ((NotificationManager) c.getSystemService(Context.NOTIFICATION_SERVICE)).notify(kind.hashCode(), b.build());
  }
}
