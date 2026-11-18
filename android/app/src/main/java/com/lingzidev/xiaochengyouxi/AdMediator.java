package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.Context;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.view.View;
import java.util.LinkedHashMap;
import java.util.Map;
import org.json.JSONArray;
import org.json.JSONObject;

/**
 * Walks the server's ad plan: tries each listed network in order until one fills. An empty plan
 * (switched off, or a server-side risk decision) behaves exactly like "no fill": same quiet
 * message, nothing else. Every step is reported to the admin statistics.
 */
final class AdMediator implements RewardedAdProvider {
  /** Shown when no network had an ad — also what a risk-blocked user sees. */
  static final String NO_AD = "暂时没有可观看的视频，请稍后再来";

  private static final long LOAD_TIMEOUT_MS = 15000;
  private final Map<String, AdNetwork> networks = new LinkedHashMap<>();
  private final Map<String, String> appIds = new LinkedHashMap<>();
  private final Handler main = new Handler(Looper.getMainLooper());
  private final AdEvents events;
  private Context app;
  /** Called on the UI thread when a rewarded video is about to appear (hides the spinner). */
  Runnable onAdVisible;

  AdMediator(Api api) {
    events = new AdEvents(api);
    for (AdNetwork n :
        new AdNetwork[] {new CsjNetwork(), new KsNetwork(), new BdNetwork(), new GdtNetwork()})
      networks.put(n.id(), n);
  }

  /** Initialises every network the server enabled (app ids from /config "ads.networks"). */
  void start(Context context, JSONObject config) {
    app = context.getApplicationContext();
    JSONObject ids = config == null ? null : config.optJSONObject("networks");
    if (ids == null) {
      // Older servers: 穿山甲 only.
      ids = Api.object("csj", BuildConfig.CSJ_APP_ID);
    }
    for (String id : networks.keySet()) {
      String appId = ids.optString(id, "");
      if (appId.isEmpty()) continue;
      appIds.put(id, appId);
      ensure(id, appId, () -> Log.i("AdMediator", id + " ready"), (c, m) -> {});
    }
  }

  void flush() {
    events.flush();
  }

  private void ensure(String id, String appId, Runnable ready, AdNetwork.Failed failed) {
    AdNetwork net = networks.get(id);
    if (net == null || app == null || appId == null || appId.isEmpty()) {
      failed.failed("unavailable", "not configured");
      return;
    }
    // Once started, an SDK keeps its app id until the app restarts (admin note says so).
    if (net.ready()) {
      ready.run();
      return;
    }
    appIds.put(id, appId);
    main.post(
        () ->
            net.init(
                app,
                appId,
                () -> main.post(ready),
                (c, m) -> {
                  Log.w("AdMediator", id + " init failed: " + c + " " + m);
                  events.log(id, "rewarded", null, "error", null, c, "init: " + m, 0);
                  main.post(() -> failed.failed(c, m));
                }));
  }

  // ---------------- rewarded ----------------

  @Override
  public void show(
      Activity activity,
      JSONObject session,
      Closed closed,
      Api.Result clientEvent,
      Api.Failure failure) {
    JSONArray plan = session.optJSONArray("plan");
    if (plan == null && !session.optString("ad_code_id").isEmpty()) {
      plan = new JSONArray();
      plan.put(
          Api.object(
              "network",
              "csj",
              "app_id",
              session.optString("ad_app_id", BuildConfig.CSJ_APP_ID),
              "code_id",
              session.optString("ad_code_id")));
    }
    if (plan == null) plan = new JSONArray();
    tryRewarded(activity, session, plan, 0, closed, clientEvent, failure);
  }

  private void tryRewarded(
      Activity activity,
      JSONObject session,
      JSONArray plan,
      int index,
      Closed closed,
      Api.Result clientEvent,
      Api.Failure failure) {
    if (activity.isDestroyed()) return;
    if (index >= plan.length()) {
      events.flush();
      failure.failed(NO_AD);
      return;
    }
    JSONObject slot = plan.optJSONObject(index);
    String id = slot == null ? "" : slot.optString("network");
    String code = slot == null ? "" : slot.optString("code_id");
    AdNetwork net = networks.get(id);
    Runnable next =
        () -> main.post(() -> tryRewarded(activity, session, plan, index + 1, closed, clientEvent, failure));
    if (net == null || code.isEmpty()) {
      next.run();
      return;
    }
    String sid = session.optString("id");
    ensure(
        id,
        slot.optString("app_id"),
        () -> loadRewarded(activity, net, code, session, sid, closed, clientEvent, next),
        (c, m) -> next.run());
  }

  private void loadRewarded(
      Activity activity,
      AdNetwork net,
      String code,
      JSONObject session,
      String sid,
      Closed closed,
      Api.Result clientEvent,
      Runnable next) {
    String id = net.id();
    long started = System.currentTimeMillis();
    events.log(id, "rewarded", code, "request", sid, null, null, 0);
    final Notice notice = new Notice();
    final boolean[] flags = new boolean[3]; // [settled(loaded or given up), closed, timedOut]
    Runnable timeout =
        () -> {
          if (flags[0]) return;
          flags[0] = true;
          flags[2] = true;
          events.log(id, "rewarded", code, "timeout", sid, null, null, started);
          next.run();
        };
    main.postDelayed(timeout, LOAD_TIMEOUT_MS);
    try {
      net.loadRewarded(
          activity,
          code,
          session,
          new AdNetwork.Rewarded() {
            @Override
            public void loaded(Runnable show) {
              ui(
                  activity,
                  () -> {
                    if (flags[0]) return; // too late: we already moved on
                    flags[0] = true;
                    main.removeCallbacks(timeout);
                    events.log(id, "rewarded", code, "fill", sid, null, null, started);
                    if (onAdVisible != null) {
                      onAdVisible.run();
                      onAdVisible = null;
                    }
                    try {
                      show.run();
                    } catch (RuntimeException error) {
                      events.log(id, "rewarded", code, "error", sid, "show", String.valueOf(error), 0);
                      next.run();
                    }
                  });
            }

            @Override
            public void failed(String c, String m) {
              ui(
                  activity,
                  () -> {
                    if (flags[0]) return;
                    flags[0] = true;
                    main.removeCallbacks(timeout);
                    events.log(id, "rewarded", code, "no_fill", sid, c, m, started);
                    next.run();
                  });
            }

            @Override
            public void shown() {
              events.log(id, "rewarded", code, "show", sid, null, null, 0);
            }

            @Override
            public void clicked() {
              events.log(id, "rewarded", code, "click", sid, null, null, 0);
            }

            @Override
            public void verified(boolean valid, Integer c, String m) {
              ui(
                  activity,
                  () -> {
                    notice.arrived(valid, c, m);
                    events.log(id, "rewarded", code, "reward", sid, c, valid ? null : m, 0);
                    JSONObject e = Api.object("event", "reward_arrived", "valid", valid, "network", id);
                    try {
                      if (c != null) e.put("error_code", c);
                      if (m != null) e.put("error_message", m.substring(0, Math.min(500, m.length())));
                    } catch (org.json.JSONException ignored) {
                    }
                    clientEvent.done(e);
                  });
            }

            @Override
            public void completed() {
              ui(activity, () -> notice.complete = true);
            }

            @Override
            public void skipped() {
              ui(
                  activity,
                  () -> {
                    notice.skipped = true;
                    events.log(id, "rewarded", code, "skip", sid, null, null, 0);
                  });
            }

            @Override
            public void playError(String m) {
              ui(
                  activity,
                  () -> {
                    notice.playbackError = true;
                    events.log(id, "rewarded", code, "error", sid, "play", m, 0);
                    clientEvent.done(Api.object("event", "video_error", "error_message", "视频播放失败"));
                  });
            }

            @Override
            public void closed() {
              ui(
                  activity,
                  () -> {
                    if (flags[1] || flags[2]) return;
                    flags[1] = true;
                    events.log(id, "rewarded", code, "close", sid, null, null, 0);
                    events.flush();
                    clientEvent.done(Api.object("event", "ad_closed", "network", id));
                    closed.done(notice);
                  });
            }
          });
    } catch (RuntimeException | LinkageError error) {
      main.removeCallbacks(timeout);
      if (!flags[0]) {
        flags[0] = true;
        events.log(id, "rewarded", code, "error", sid, "load", String.valueOf(error), started);
        next.run();
      }
    }
  }

  // ---------------- feed ----------------

  interface FeedResult {
    void rendered(View view, Runnable destroy);

    void empty();

    void dismissed();
  }

  /** Tries the plan for one hall feed card. */
  void loadFeed(Activity activity, JSONArray plan, int widthPx, FeedResult result) {
    tryFeed(activity, plan, 0, widthPx, result);
  }

  private void tryFeed(Activity activity, JSONArray plan, int index, int widthPx, FeedResult result) {
    if (activity.isDestroyed()) return;
    if (plan == null || index >= plan.length()) {
      result.empty();
      return;
    }
    JSONObject slot = plan.optJSONObject(index);
    String id = slot == null ? "" : slot.optString("network");
    String code = slot == null ? "" : slot.optString("code_id");
    AdNetwork net = networks.get(id);
    Runnable next = () -> main.post(() -> tryFeed(activity, plan, index + 1, widthPx, result));
    if (net == null || code.isEmpty()) {
      next.run();
      return;
    }
    ensure(
        id,
        slot.optString("app_id"),
        () -> {
          long started = System.currentTimeMillis();
          events.log(id, "feed", code, "request", null, null, null, 0);
          final boolean[] settled = new boolean[1];
          Runnable timeout =
              () -> {
                if (settled[0]) return;
                settled[0] = true;
                events.log(id, "feed", code, "timeout", null, null, null, started);
                next.run();
              };
          main.postDelayed(timeout, LOAD_TIMEOUT_MS);
          try {
            net.loadFeed(
                activity,
                code,
                widthPx,
                new AdNetwork.Feed() {
                  @Override
                  public void rendered(View view, Runnable destroy) {
                    ui(
                        activity,
                        () -> {
                          if (settled[0]) {
                            destroy.run();
                            return;
                          }
                          settled[0] = true;
                          main.removeCallbacks(timeout);
                          events.log(id, "feed", code, "fill", null, null, null, started);
                          result.rendered(view, destroy);
                        });
                  }

                  @Override
                  public void failed(String c, String m) {
                    ui(
                        activity,
                        () -> {
                          if (settled[0]) return;
                          settled[0] = true;
                          main.removeCallbacks(timeout);
                          events.log(id, "feed", code, "no_fill", null, c, m, started);
                          next.run();
                        });
                  }

                  @Override
                  public void shown() {
                    events.log(id, "feed", code, "show", null, null, null, 0);
                  }

                  @Override
                  public void clicked() {
                    events.log(id, "feed", code, "click", null, null, null, 0);
                  }

                  @Override
                  public void dismissed() {
                    events.log(id, "feed", code, "close", null, null, null, 0);
                    ui(activity, result::dismissed);
                  }
                });
          } catch (RuntimeException | LinkageError error) {
            main.removeCallbacks(timeout);
            if (!settled[0]) {
              settled[0] = true;
              events.log(id, "feed", code, "error", null, "load", String.valueOf(error), started);
              next.run();
            }
          }
        },
        (c, m) -> next.run());
  }

  private static void ui(Activity activity, Runnable action) {
    activity.runOnUiThread(
        () -> {
          if (!activity.isDestroyed()) action.run();
        });
  }
}
