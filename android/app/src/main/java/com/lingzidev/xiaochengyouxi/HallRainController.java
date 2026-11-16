package com.lingzidev.xiaochengyouxi;

import android.animation.*;
import android.app.Dialog;
import android.graphics.Color;
import android.os.*;
import android.view.*;
import android.view.animation.LinearInterpolator;
import android.widget.*;
import java.util.*;
import org.json.*;

final class HallRainController {
  private final MainActivity app;
  private final Api api;
  private final Handler timer = new Handler(Looper.getMainLooper());
  private final Map<String, Drop> falling = new LinkedHashMap<>();
  private final Set<String> shown = new HashSet<>(), claiming = new HashSet<>();
  private final Set<Integer> invalidPackets = new HashSet<>();
  private final Map<String, long[]> packetTimes = new HashMap<>();
  private final ArrayList<View> creditViews = new ArrayList<>();
  private FrameLayout surface, sky;
  private View collapse;
  private TextView clock, countNumber;
  private Dialog countdown;
  private long countdownSecond = -1;
  private JSONObject status, assignment;
  private long serverAtSync, elapsedAtSync;
  private int generation;
  private boolean attached, joinInFlight;
  private boolean polling;
  private Runnable scheduledPoll;
  private String attemptedRound = "", dismissedCountdown = "", hiddenRound = "", lastBoundary = "";

  HallRainController(MainActivity app, Api api) {
    this.app = app;
    this.api = api;
  }

  boolean busy() {
    return countdown != null || sky != null || !claiming.isEmpty() || !creditViews.isEmpty();
  }

  void attach(FrameLayout surface, TextView clock) {
    detach();
    this.surface = surface;
    this.clock = clock;
    attached = true;
    int epoch = generation;
    poll(epoch);
    timer.post(tick);
  }

  void detach() {
    attached = false;
    generation++;
    timer.removeCallbacksAndMessages(null);
    scheduledPoll = null;
    polling = false;
    attemptedRound = "";
    closeCountdown();
    clearSky();
    surface = null;
    clock = null;
  }

  void reset() {
    detach();
    status = null;
    assignment = null;
    attemptedRound = "";
    dismissedCountdown = "";
    hiddenRound = "";
    lastBoundary = "";
    claiming.clear();
    invalidPackets.clear();
    packetTimes.clear();
    joinInFlight = false;
  }

  void markClaimed(String packetId) {
    JSONObject packet = findPacket(packetId);
    if (packet != null) putStatus(packet, "claimed");
    Drop drop = falling.remove(packetId);
    if (drop != null) drop.remove();
  }

  void releasePacket(String packetId) {
    if (packetId.isEmpty()) return;
    shown.remove(packetId);
  }

  private final Runnable tick =
      new Runnable() {
        @Override
        public void run() {
          if (!attached) return;
          try {
            updateScene();
          } catch (JSONException | IllegalArgumentException error) {
            app.tip(error.getMessage());
          }
          timer.postDelayed(this, 100);
        }
      };

  private void poll(int epoch) {
    if (!active(epoch) || polling) return;
    polling = true;
    api.get(
        "/rain/status",
        result -> {
          if (!active(epoch)) return;
          polling = false;
          status = (JSONObject) result;
          try {
            sync(status.getString("server_now"));
            updateScene();
          } catch (JSONException | IllegalArgumentException error) {
            app.tip(error.getMessage());
          }
          schedulePoll(epoch, status.optString("phase").equals("active") ? 2000 : 5000);
        },
        message -> {
          if (!active(epoch)) return;
          polling = false;
          clock.setText("刷新红包雨");
          clock.setOnClickListener(v -> poll(epoch));
          app.tip(message);
        });
  }

  private void schedulePoll(int epoch, long delay) {
    if (scheduledPoll != null) timer.removeCallbacks(scheduledPoll);
    scheduledPoll = () -> poll(epoch);
    timer.postDelayed(scheduledPoll, delay);
  }

  private void refreshBoundary(String boundary) {
    if (!lastBoundary.equals(boundary)) {
      lastBoundary = boundary;
      schedulePoll(generation, 0);
    }
  }

  private boolean active(int epoch) {
    return attached && epoch == generation && !app.isDestroyed();
  }

  private long now() {
    return serverAtSync + SystemClock.elapsedRealtime() - elapsedAtSync;
  }

  private void sync(String serverNow) {
    serverAtSync = RainTiming.parse(serverNow);
    elapsedAtSync = SystemClock.elapsedRealtime();
  }

  private void updateScene() throws JSONException {
    if (status == null || clock == null) return;
    String phase = status.getString("phase");
    if (phase.equals("disabled")) {
      clock.setText("敬请期待");
      clock.setOnClickListener(null);
      closeCountdown();
      clearSky();
      return;
    }
    if (phase.equals("waiting")) {
      clearSky();
      long starts = RainTiming.parse(status.getString("next_start_at")), remaining = starts - now();
      clock.setText("距下场 " + RainTiming.remaining(remaining));
      clock.setOnClickListener(null);
      String round = Long.toString(starts);
      if (remaining > 0 && remaining <= 10000 && !round.equals(dismissedCountdown))
        showCountdown(round, remaining);
      else if (remaining <= 0) {
        closeCountdown();
        refreshBoundary("start:" + round);
      }
      return;
    }
    JSONObject round = status.getJSONObject("round");
    String roundId = round.getString("id");
    long ends = RainTiming.parse(round.getString("end_at"));
    closeCountdown();
    if (now() >= ends) {
      clock.setText("本场已结束");
      clearSky();
      refreshBoundary("end:" + roundId);
      return;
    }
    clock.setText("本场剩余 " + RainTiming.remaining(ends - now()));
    if (assignment == null || !assignment.getJSONObject("round").getString("id").equals(roundId)) {
      if (!joinInFlight && !attemptedRound.equals(roundId))
        join(roundId, status.optBoolean("joined"));
      if (joinInFlight) {
        clock.setText("正在迎接红包雨");
        clock.setOnClickListener(null);
      } else {
        clock.setText("点击加入本场红包雨");
        clock.setOnClickListener(v -> join(roundId, status.optBoolean("joined")));
      }
      return;
    }
    if (hiddenRound.equals(roundId)) {
      clock.setText("展开本场红包雨");
      clock.setOnClickListener(
          v -> {
            hiddenRound = "";
            try {
              updateScene();
            } catch (JSONException error) {
              app.tip(error.getMessage());
            }
          });
      return;
    }
    clock.setOnClickListener(null);
    if (surface.getWidth() == 0 || surface.getHeight() == 0) return;
    ensureSky();
    JSONArray packets = assignment.getJSONArray("packets");
    int maximum = assignment.getJSONObject("config").getInt("max_visible");
    long serverNow = now();
    for (Drop drop : new ArrayList<>(falling.values()))
      if (serverNow >= drop.expires) {
        drop.remove();
        falling.remove(drop.packet.optString("id"));
      }
    for (int i = 0; i < packets.length(); i++) {
      if (invalidPackets.contains(i)) continue;
      try {
        JSONObject packet = packets.getJSONObject(i);
        String id = packet.getString("id"), state = packet.getString("status");
        if (state.equals("claimed") || shown.contains(id) || claiming.contains(id)) continue;
        long[] window = packetTimes.get(id);
        if (window == null) {
          window =
              new long[] {
                RainTiming.parse(packet.getString("reveal_at")),
                RainTiming.parse(packet.getString("expires_at"))
              };
          packetTimes.put(id, window);
        }
        long reveal = window[0], expires = window[1];
        if (serverNow >= reveal && serverNow < expires && falling.size() < maximum) {
          shown.add(id);
          Drop drop = new Drop(packet, reveal, expires);
          falling.put(id, drop);
          drop.start(serverNow);
        }
      } catch (JSONException | IllegalArgumentException error) {
        invalidPackets.add(i);
        app.tip(error.getMessage());
      }
    }
  }

  private void join(String roundId, boolean previouslyJoined) {
    if (api.updateRequired && !previouslyJoined) return;
    joinInFlight = true;
    attemptedRound = roundId;
    int epoch = generation;
    String token = api.token;
    Api.Result ready =
        result -> {
          if (!token.equals(api.token)) return;
          joinInFlight = false;
          JSONObject incoming = (JSONObject) result;
          if (assignment != null
              && !assignment
                  .optJSONObject("round")
                  .optString("id")
                  .equals(incoming.optJSONObject("round").optString("id"))) clearSky();
          assignment = incoming;
          shown.clear();
          invalidPackets.clear();
          packetTimes.clear();
          if (!attached) return;
          try {
            sync(assignment.getString("server_now"));
            updateScene();
          } catch (JSONException | IllegalArgumentException error) {
            app.tip(error.getMessage());
          }
        };
    Api.Failure failed =
        message -> {
          if (!token.equals(api.token)) return;
          joinInFlight = false;
          if (!active(epoch)) return;
          clock.setText("点击加入本场红包雨");
          clock.setOnClickListener(v -> join(roundId, status.optBoolean("joined")));
          app.error(message);
        };
    JSONObject intent = Api.object("round_id", roundId);
    if (previouslyJoined) api.request("POST", "/rain/join", intent, ready, failed);
    else api.post("/rain/join", intent, ready, failed);
  }

  private void showCountdown(String round, long remaining) {
    if (countdown != null) {
      long seconds = (remaining + 999) / 1000;
      if (countdownSecond != seconds) {
        countdownSecond = seconds;
        countNumber.setText(Long.toString(seconds));
        if (enabledAnimations()) {
          countNumber.setScaleX(.85f);
          countNumber.setScaleY(.85f);
          countNumber.animate().scaleX(1).scaleY(1).setDuration(280).start();
        }
      }
      return;
    }
    LinearLayout box = app.col();
    box.setGravity(Gravity.CENTER_HORIZONTAL);
    box.setPadding(app.dp(22), app.dp(14), app.dp(22), app.dp(22));
    box.setBackground(app.grad(0xfffff3df, 0xffffddbd, 28));
    LinearLayout top = app.row();
    app.flex(top, app.text("一场小欢喜，即将洒落", 17, MainActivity.INK, true));
    LineIcon close = new LineIcon(app, "close", MainActivity.MUTED);
    app.add(top, close, 22, 22);
    app.add(box, top, -1, 35);
    ImageView skyArt = app.art("rain_sky");
    app.add(box, skyArt, -1, 100);
    countNumber = app.text(Long.toString((remaining + 999) / 1000), 65, MainActivity.RED, true);
    countdownSecond = (remaining + 999) / 1000;
    app.add(box, countNumber, -2, -2);
    app.gap(box, 10);
    app.add(box, app.text("红包雨马上来啦", 18, MainActivity.INK, true), -2, -2);
    app.gap(box, 8);
    app.add(box, app.label("接住小红包，收下一点好心情", 12, MainActivity.MUTED), -2, -2);
    countdown = app.sheet(box, 9);
    close.setOnClickListener(
        v -> {
          dismissedCountdown = round;
          closeCountdown();
        });
    countdown.setOnCancelListener(dialog -> dismissedCountdown = round);
    countdown.setOnDismissListener(
        dialog -> {
          countdown = null;
          countNumber = null;
        });
    countdown.show();
    countdown
        .getWindow()
        .setLayout(
            Math.min(app.getResources().getDisplayMetrics().widthPixels - app.dp(40), app.dp(370)),
            -2);
  }

  private void closeCountdown() {
    if (countNumber != null) countNumber.animate().cancel();
    if (countdown != null) countdown.dismiss();
    countdown = null;
    countNumber = null;
    countdownSecond = -1;
  }

  private void ensureSky() throws JSONException {
    if (sky != null) return;
    sky = new FrameLayout(app);
    api.behavior.overlay(this, 9);
    sky.setClickable(true);
    int alpha =
        (int) Math.round(255 * assignment.getJSONObject("config").getDouble("overlay_opacity"));
    sky.setBackgroundColor(Color.argb(alpha, 31, 14, 6));
    ImageView clouds = app.art("rain_sky");
    clouds.setScaleType(ImageView.ScaleType.FIT_XY);
    sky.addView(clouds, new FrameLayout.LayoutParams(-1, app.dp(200), Gravity.TOP));
    TextView title = app.text("接住好运，小欢喜正在洒落", 15, 0xffffedc5, true);
    title.setGravity(Gravity.CENTER);
    FrameLayout.LayoutParams tp = new FrameLayout.LayoutParams(-1, app.dp(45), Gravity.TOP);
    tp.topMargin = app.dp(88);
    sky.addView(title, tp);
    surface.addView(sky, new FrameLayout.LayoutParams(-1, -1));
    LineIcon close = new LineIcon(app, "close", 0xff9e512a);
    FrameLayout closeButton = new FrameLayout(app);
    closeButton.addView(
        close, new FrameLayout.LayoutParams(app.dp(22), app.dp(22), Gravity.CENTER));
    closeButton.setBackground(app.bg(0xe6fff4dd, 22));
    closeButton.setContentDescription("收起本场红包雨");
    FrameLayout.LayoutParams cp =
        new FrameLayout.LayoutParams(app.dp(44), app.dp(44), Gravity.TOP | Gravity.RIGHT);
    cp.topMargin = app.dp(12);
    cp.rightMargin = app.dp(12);
    sky.addView(closeButton, cp);
    collapse = closeButton;
    closeButton.setOnClickListener(
        v -> {
          api.behavior.click(12, "");
          hiddenRound = assignment.optJSONObject("round").optString("id");
          clearSky();
        });
  }

  private void clearSky() {
    api.behavior.overlay(this, 0);
    for (View credit : creditViews) {
      credit.animate().cancel();
      if (credit.getParent() instanceof FrameLayout)
        ((FrameLayout) credit.getParent()).removeView(credit);
    }
    creditViews.clear();
    for (Drop drop : new ArrayList<>(falling.values())) drop.remove();
    falling.clear();
    shown.clear();
    if (sky != null && sky.getParent() instanceof FrameLayout)
      ((FrameLayout) sky.getParent()).removeView(sky);
    sky = null;
    collapse = null;
  }

  private JSONObject findPacket(String id) {
    if (assignment == null) return null;
    JSONArray packets = assignment.optJSONArray("packets");
    for (int i = 0; i < packets.length(); i++) {
      JSONObject packet = packets.optJSONObject(i);
      if (packet.optString("id").equals(id)) return packet;
    }
    return null;
  }

  private static void putStatus(JSONObject packet, String state) {
    try {
      packet.put("status", state);
    } catch (JSONException error) {
      throw new IllegalArgumentException(error);
    }
  }

  private void claim(Drop drop) {
    String id = drop.packet.optString("id"), token = api.token;
    api.behavior.click(10, id);
    if (!claiming.add(id)) return;
    int epoch = generation;
    float x = drop.view.getX() + drop.size / 2f, y = drop.view.getY() + drop.size / 2f;
    drop.remove();
    falling.remove(id);
    api.post(
        "/rain/claim",
        Api.object("packet_id", id),
        result -> {
          claiming.remove(id);
          if (!token.equals(api.token)) return;
          JSONObject response = (JSONObject) result;
          if (response.optString("kind").equals("small")) {
            putStatus(drop.packet, "claimed");
            boolean credited = response.optBoolean("credited");
            app.rainCredited(
                response.optJSONObject("user"),
                credited ? response.optInt("reward") : 0,
                active(epoch));
            if (active(epoch) && credited) showCredit(response.optInt("reward"), x, y);
          } else {
            putStatus(drop.packet, "opened");
            if (active(epoch)) app.openPacket(response.optJSONObject("task"));
          }
        },
        message -> {
          claiming.remove(id);
          if (!token.equals(api.token)) return;
          if (active(epoch)) {
            shown.remove(id);
            app.tip(message);
          }
        });
  }

  private void showCredit(int coins, float x, float y) {
    if (sky == null || coins <= 0) return;
    TextView earned = app.text("+" + coins + " 金币", 24, 0xffffda80, true);
    earned.setShadowLayer(app.dp(5), 0, app.dp(2), 0xff9c4420);
    sky.addView(earned, new FrameLayout.LayoutParams(app.dp(135), app.dp(42)));
    creditViews.add(earned);
    earned.setX(Math.max(0, Math.min(sky.getWidth() - app.dp(135), x - app.dp(50))));
    earned.setY(y);
    earned
        .animate()
        .translationY(y - app.dp(70))
        .alpha(0)
        .setDuration(1000)
        .withEndAction(
            () -> {
              if (earned.getParent() instanceof FrameLayout)
                ((FrameLayout) earned.getParent()).removeView(earned);
              creditViews.remove(earned);
            })
        .start();
  }

  private final class Drop {
    final JSONObject packet;
    final long reveal, expires;
    final int size;
    final FrameLayout view;
    final ArrayList<Animator> animators = new ArrayList<>();
    boolean removed;

    Drop(JSONObject packet, long reveal, long expires) throws JSONException {
      this.packet = packet;
      this.reveal = reveal;
      this.expires = expires;
      String kind = packet.getString("kind");
      if (!kind.equals("small") && !kind.equals("large"))
        throw new IllegalArgumentException("红包类型错误：" + kind);
      boolean large = kind.equals("large");
      size = app.dp(large ? 104 : 62);
      view = new FrameLayout(app);
      view.setClipChildren(false);
      ImageView image = app.art(large ? "rain_large" : "rain_small");
      view.addView(image, new FrameLayout.LayoutParams(-1, -1));
      view.setContentDescription(large ? "金喜大红包，观看视频领取奖励" : "小红包，点击领取金币");
      view.setOnClickListener(v -> claim(this));
      sky.addView(view, new FrameLayout.LayoutParams(size, size));
      collapse.bringToFront();
      int hash = packet.getString("id").hashCode() & 0x7fffffff;
      float fraction = (hash % 10000) / 10000f;
      view.setX(fraction * Math.max(0, surface.getWidth() - size));
      if (large && enabledAnimations()) {
        ObjectAnimator pulse = ObjectAnimator.ofFloat(image, View.SCALE_X, .92f, 1.08f, .92f);
        ObjectAnimator pulseY = ObjectAnimator.ofFloat(image, View.SCALE_Y, .92f, 1.08f, .92f);
        for (ObjectAnimator animator : new ObjectAnimator[] {pulse, pulseY}) {
          animator.setDuration(1200);
          animator.setRepeatCount(ValueAnimator.INFINITE);
          animators.add(animator);
          animator.start();
        }
      }
    }

    void start(long serverNow) {
      float height = surface.getHeight(), distance = height + size * 2f;
      float progress = RainTiming.progress(reveal, expires, serverNow);
      float start = -size + distance * progress;
      if (!enabledAnimations()) {
        view.setY(height * .25f + (packet.optString("id").hashCode() & 255) / 255f * height * .5f);
        return;
      }
      ObjectAnimator fall = ObjectAnimator.ofFloat(view, View.TRANSLATION_Y, start, height + size);
      fall.setInterpolator(new LinearInterpolator());
      fall.setDuration(Math.max(1, expires - serverNow));
      fall.addListener(
          new AnimatorListenerAdapter() {
            @Override
            public void onAnimationEnd(Animator animation) {
              if (!removed) {
                remove();
                falling.remove(packet.optString("id"));
              }
            }
          });
      animators.add(fall);
      fall.start();
    }

    void remove() {
      if (removed) return;
      removed = true;
      for (Animator animator : animators) animator.cancel();
      animators.clear();
      if (view.getParent() instanceof FrameLayout)
        ((FrameLayout) view.getParent()).removeView(view);
    }
  }

  private static boolean enabledAnimations() {
    return Build.VERSION.SDK_INT < 26 || ValueAnimator.areAnimatorsEnabled();
  }
}
