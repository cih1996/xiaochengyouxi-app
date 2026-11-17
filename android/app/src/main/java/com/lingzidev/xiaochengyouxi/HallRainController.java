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
  private TextView clock, countNumber, hudTime, hudCoins;
  private RainBackdrop backdrop;
  private final ArrayList<Animator> skyAnimators = new ArrayList<>();
  private int roundCoins, roundPackets;
  private String summaryRound = "";
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
      boolean wasRaining = sky != null;
      clearSky();
      if (wasRaining && !summaryRound.equals(roundId)) {
        summaryRound = roundId;
        showSummary();
      }
      refreshBoundary("end:" + roundId);
      return;
    }
    clock.setText("本场剩余 " + RainTiming.remaining(ends - now()));
    if (hudTime != null) hudTime.setText(Math.max(0, (ends - now() + 999) / 1000) + "s");
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
          roundCoins = 0;
          roundPackets = 0;
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

  private FrameLayout host() {
    return app.stage;
  }

  private void ensureSky() throws JSONException {
    if (sky != null) return;
    FrameLayout host = host();
    sky = new FrameLayout(app);
    sky.setClipChildren(false);
    api.behavior.overlay(this, 9);
    sky.setClickable(true);
    backdrop = new RainBackdrop(app);
    backdrop.setAnimated(enabledAnimations());
    sky.addView(backdrop, new FrameLayout.LayoutParams(-1, -1));

    // Top garland: edge to edge, slides in, then gently bobs and breathes.
    int width = host.getWidth();
    int garlandHeight = Math.round(width * 1024f / 1536f);
    ImageView clouds = app.art("rain_sky");
    clouds.setScaleType(ImageView.ScaleType.FIT_XY);
    clouds.setPivotY(0);
    sky.addView(clouds, new FrameLayout.LayoutParams(-1, garlandHeight, Gravity.TOP));
    if (enabledAnimations()) {
      clouds.setTranslationY(-garlandHeight);
      ObjectAnimator in = ObjectAnimator.ofFloat(clouds, View.TRANSLATION_Y, -garlandHeight, 0);
      in.setDuration(650);
      in.setInterpolator(new android.view.animation.OvershootInterpolator(1.1f));
      ObjectAnimator bob = ObjectAnimator.ofFloat(clouds, View.TRANSLATION_Y, 0, app.dp(7), 0);
      bob.setDuration(2600);
      bob.setStartDelay(650);
      bob.setRepeatCount(ValueAnimator.INFINITE);
      ObjectAnimator breathe = ObjectAnimator.ofFloat(clouds, View.SCALE_X, 1f, 1.035f, 1f);
      breathe.setDuration(3400);
      breathe.setRepeatCount(ValueAnimator.INFINITE);
      for (ObjectAnimator a : new ObjectAnimator[] {in, bob, breathe}) {
        skyAnimators.add(a);
        a.start();
      }
    }

    // Compact HUD: time left and coins caught this round.
    LinearLayout hud = app.row();
    hud.setPadding(app.dp(12), 0, app.dp(14), 0);
    hud.setBackground(app.bg(0x66400a07, 20));
    app.add(hud, new LineIcon(app, "clock", 0xffffe3a3), 16, 16);
    hudTime = app.text("", 14, 0xfffff1cf, true);
    hudTime.setPadding(app.dp(5), 0, app.dp(12), 0);
    app.add(hud, hudTime, -2, -2);
    app.add(hud, new LineIcon(app, "coin", 0xffffd166), 16, 16);
    hudCoins = app.text("+" + roundCoins, 14, 0xffffd166, true);
    hudCoins.setPadding(app.dp(5), 0, 0, 0);
    app.add(hud, hudCoins, -2, -2);
    FrameLayout.LayoutParams hp = new FrameLayout.LayoutParams(-2, app.dp(36), Gravity.TOP | Gravity.LEFT);
    hp.topMargin = app.dp(14);
    hp.leftMargin = app.dp(16);
    sky.addView(hud, hp);

    host.addView(sky, new FrameLayout.LayoutParams(-1, -1));
    app.rainChrome(true);
    if (enabledAnimations()) {
      sky.setAlpha(0);
      sky.animate().alpha(1).setDuration(300).start();
    }
    LineIcon close = new LineIcon(app, "close", 0xfffff1cf);
    FrameLayout closeButton = new FrameLayout(app);
    closeButton.addView(
        close, new FrameLayout.LayoutParams(app.dp(18), app.dp(18), Gravity.CENTER));
    android.graphics.drawable.GradientDrawable ring = app.bg(0x66400a07, 99);
    ring.setStroke(app.dp(1.5f), 0x88fff1cf);
    closeButton.setBackground(ring);
    closeButton.setContentDescription("收起本场红包雨");
    FrameLayout.LayoutParams cp =
        new FrameLayout.LayoutParams(app.dp(36), app.dp(36), Gravity.TOP | Gravity.RIGHT);
    cp.topMargin = app.dp(14);
    cp.rightMargin = app.dp(16);
    sky.addView(closeButton, cp);
    collapse = closeButton;
    closeButton.setOnClickListener(
        v -> {
          api.behavior.click(12, "");
          hiddenRound = assignment.optJSONObject("round").optString("id");
          clearSky();
        });
  }

  private void showSummary() {
    if (roundPackets == 0 || app.isDestroyed()) return;
    FrameLayout host = host();
    LinearLayout card = app.col();
    card.setGravity(Gravity.CENTER_HORIZONTAL);
    card.setPadding(app.dp(24), app.dp(18), app.dp(24), app.dp(20));
    card.setBackground(app.grad(0xfffff3df, 0xffffdcbc, 26));
    card.setElevation(app.dp(10));
    app.add(card, app.art("packet"), 92, 92);
    app.gap(card, 4);
    app.add(card, app.text("本场接到 " + roundPackets + " 个红包", 15, MainActivity.INK, true), -2, -2);
    app.gap(card, 4);
    app.add(card, app.text("+" + roundCoins + " 金币", 30, MainActivity.RED, true), -2, -2);
    FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(app.dp(240), -2, Gravity.CENTER);
    host.addView(card, lp);
    card.setScaleX(.6f);
    card.setScaleY(.6f);
    card.setAlpha(0);
    card.animate().scaleX(1).scaleY(1).alpha(1).setDuration(380)
        .setInterpolator(new android.view.animation.OvershootInterpolator(1.4f)).start();
    card.setOnClickListener(v -> host.removeView(card));
    timer.postDelayed(
        () -> card.animate().alpha(0).scaleX(.9f).scaleY(.9f).setDuration(250)
            .withEndAction(() -> host.removeView(card)).start(),
        2600);
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
    for (Animator a : skyAnimators) a.cancel();
    skyAnimators.clear();
    if (backdrop != null) backdrop.setAnimated(false);
    backdrop = null;
    hudTime = null;
    hudCoins = null;
    if (sky != null) app.rainChrome(false);
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
    burst(x, y, drop.size);
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
            if (credited) {
              roundCoins += response.optInt("reward");
              roundPackets++;
              if (hudCoins != null) {
                hudCoins.setText("+" + roundCoins);
                hudCoins.animate().cancel();
                hudCoins.setScaleX(1.35f);
                hudCoins.setScaleY(1.35f);
                hudCoins.animate().scaleX(1).scaleY(1).setDuration(260).start();
              }
            }
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

  private void burst(float x, float y, int size) {
    if (sky == null || !enabledAnimations()) return;
    // Opening flash plus a ring of gold sparks.
    View flash = new View(app);
    flash.setBackground(app.bg(0xccffe9a8, 999));
    sky.addView(flash, new FrameLayout.LayoutParams(size, size));
    flash.setX(x - size / 2f);
    flash.setY(y - size / 2f);
    creditViews.add(flash);
    flash.animate().scaleX(1.9f).scaleY(1.9f).alpha(0).setDuration(380)
        .withEndAction(() -> { sky_remove(flash); }).start();
    for (int i = 0; i < 10; i++) {
      View spark = new View(app);
      int d = app.dp(i % 2 == 0 ? 10 : 7);
      android.graphics.drawable.GradientDrawable g = app.grad(0xffffef9a, 0xffffb11f, 99);
      spark.setBackground(g);
      sky.addView(spark, new FrameLayout.LayoutParams(d, d));
      spark.setX(x - d / 2f);
      spark.setY(y - d / 2f);
      creditViews.add(spark);
      double a = Math.PI * 2 * i / 10 + Math.random() * .3;
      float r = app.dp(55 + (float) Math.random() * 35);
      spark.animate()
          .translationXBy((float) Math.cos(a) * r)
          .translationYBy((float) Math.sin(a) * r)
          .alpha(0)
          .scaleX(.4f)
          .scaleY(.4f)
          .setDuration(560)
          .setInterpolator(new android.view.animation.DecelerateInterpolator())
          .withEndAction(() -> sky_remove(spark))
          .start();
    }
  }

  private void sky_remove(View v) {
    if (v.getParent() instanceof FrameLayout) ((FrameLayout) v.getParent()).removeView(v);
    creditViews.remove(v);
  }

  private void showCredit(int coins, float x, float y) {
    if (sky == null || coins <= 0) return;
    TextView earned = app.text("+" + coins, 30, 0xffffe08a, true);
    earned.setGravity(Gravity.CENTER);
    earned.setShadowLayer(app.dp(6), 0, app.dp(2), 0xffa3260f);
    sky.addView(earned, new FrameLayout.LayoutParams(app.dp(120), app.dp(46)));
    creditViews.add(earned);
    earned.setX(Math.max(0, Math.min(sky.getWidth() - app.dp(120), x - app.dp(60))));
    earned.setY(y - app.dp(23));
    earned.setScaleX(.4f);
    earned.setScaleY(.4f);
    earned
        .animate()
        .scaleX(1.15f)
        .scaleY(1.15f)
        .translationYBy(-app.dp(60))
        .setDuration(420)
        .setInterpolator(new android.view.animation.OvershootInterpolator(2f))
        .withEndAction(
            () ->
                earned
                    .animate()
                    .alpha(0)
                    .translationYBy(-app.dp(25))
                    .setDuration(380)
                    .withEndAction(() -> sky_remove(earned))
                    .start())
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
      int hash = packet.getString("id").hashCode() & 0x7fffffff;
      if (large) {
        View halo = new View(app);
        android.graphics.drawable.GradientDrawable glow = new android.graphics.drawable.GradientDrawable();
        glow.setGradientType(android.graphics.drawable.GradientDrawable.RADIAL_GRADIENT);
        glow.setColors(new int[] {0xaaffe27a, 0x00ffb12f});
        glow.setGradientRadius(size * .62f);
        halo.setBackground(glow);
        FrameLayout.LayoutParams hl = new FrameLayout.LayoutParams(size + app.dp(36), size + app.dp(36), Gravity.CENTER);
        view.addView(halo, hl);
        if (enabledAnimations()) {
          ObjectAnimator spinGlow = ObjectAnimator.ofFloat(halo, View.ALPHA, .55f, 1f, .55f);
          spinGlow.setDuration(1000);
          spinGlow.setRepeatCount(ValueAnimator.INFINITE);
          animators.add(spinGlow);
          spinGlow.start();
        }
      }
      ImageView image = app.art(large ? "rain_large" : "rain_small");
      view.addView(image, new FrameLayout.LayoutParams(size, size, Gravity.CENTER));
      view.setContentDescription(large ? "金喜大红包，观看视频领取奖励" : "小红包，点击领取金币");
      view.setOnClickListener(v -> claim(this));
      sky.addView(view, new FrameLayout.LayoutParams(size, size));
      collapse.bringToFront();
      float fraction = (hash % 10000) / 10000f;
      view.setX(fraction * Math.max(0, surface.getWidth() - size));
      if (enabledAnimations()) {
        // Swing like a falling envelope: tilt and drift side to side.
        float tilt = 9f + (hash % 7);
        ObjectAnimator swing = ObjectAnimator.ofFloat(image, View.ROTATION, -tilt, tilt);
        swing.setDuration(900 + hash % 500);
        swing.setRepeatMode(ValueAnimator.REVERSE);
        swing.setRepeatCount(ValueAnimator.INFINITE);
        swing.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
        ObjectAnimator drift = ObjectAnimator.ofFloat(image, View.TRANSLATION_X, -app.dp(9), app.dp(9));
        drift.setDuration(1300 + hash % 700);
        drift.setRepeatMode(ValueAnimator.REVERSE);
        drift.setRepeatCount(ValueAnimator.INFINITE);
        for (ObjectAnimator a : new ObjectAnimator[] {swing, drift}) {
          animators.add(a);
          a.start();
        }
      }
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
      float height = sky.getHeight() > 0 ? sky.getHeight() : host().getHeight(), distance = height + size * 2f;
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
