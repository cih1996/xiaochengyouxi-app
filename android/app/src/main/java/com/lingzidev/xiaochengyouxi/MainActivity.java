package com.lingzidev.xiaochengyouxi;

import android.animation.*;
import android.app.*;
import android.content.*;
import android.graphics.*;
import android.graphics.drawable.*;
import android.media.AudioAttributes;
import android.media.AudioManager;
import android.media.SoundPool;
import android.net.Uri;
import android.os.*;
import android.provider.MediaStore;
import android.text.InputFilter;
import android.text.InputType;
import android.view.*;
import android.view.animation.*;
import android.widget.*;
import androidx.core.content.FileProvider;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.text.DecimalFormat;
import java.text.ParseException;
import java.text.SimpleDateFormat;
import java.util.*;
import org.json.*;

public final class MainActivity extends Activity {
  static final int RED = 0xfff65331,
      INK = 0xff392923,
      MUTED = 0xffa38c7e,
      CREAM = 0xfffff8f2,
      WHITE = 0xffffffff,
      PALE = 0xffffe9dd;
  Api api;
  ActivityClient activities;
  JSONObject user;
  JSONObject checkinStatus;
  JSONArray tasks = new JSONArray(), products = new JSONArray();
  final ArrayList<JSONObject> hallItems = new ArrayList<>();
  HallStream hallStream;
  ScrollView hallScroll;
  LinearLayout hallMessages;
  int hallScrollY;
  FrameLayout stage;
  LinearLayout shell, page, header, tabs;
  TextView balance;
  String current = "hall", category = "全部", shopPromoImage = "";
  boolean signBusy = false;
  boolean welcomeShowing;
  CheckInButton checkInButton;
  SoundPool coinSounds;
  int coinArrivalSound;
  final Handler handler = new Handler(Looper.getMainLooper());
  RewardedAdProvider rewarded;
  PangleAds csjAds;
  NativeFeedAds nativeFeeds;
  HallRainController rain;
  FrameLayout hallSurface;
  TextView rainTime;
  AlertDialog rewardStatusDialog;
  PendingReward pendingReward;
  AppUpdates updates;
  boolean servicesReady, startupAdActive, rewardUiBusy;
  TextView hallNotice;
  String hallNoticeText = "";
  long runtimeRevision = -1;
  int runtimeEpoch;
  boolean runtimeLoading;
  boolean foreground;
  boolean productsLoading;
  String productsRequestToken = "";
  String productLoadError = "";
  boolean deletionPage;
  boolean deletionConfirmPage;
  String settingsReturn = "me";
  String deletionReturn = "hall";

  @Override
  public void onCreate(Bundle state) {
    super.onCreate(state);
    setVolumeControlStream(AudioManager.STREAM_MUSIC);
    api = new Api(this);
    activities = new ActivityClient(api);
    activities.onChanged =
        () ->
            runOnUiThread(
                () -> {
                  if (current.equals("hall")) render();
                });
    updates = new AppUpdates(this, api, state);
    csjAds = new PangleAds();
    nativeFeeds = new NativeFeedAds(this, this::error, this::removeHallItem);
    rain = new HallRainController(this, api);
    rewarded = BuildConfig.ADS_ENABLED ? csjAds : new RewardedAdProvider.Mock();
    coinSounds =
        new SoundPool.Builder()
            .setMaxStreams(1)
            .setAudioAttributes(
                new AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_GAME)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                    .build())
            .build();
    coinSounds.setOnLoadCompleteListener(
        (pool, sample, status) -> {
          if (status != 0) android.util.Log.e("CoinSound", "金币音效加载失败，状态：" + status);
        });
    coinArrivalSound = coinSounds.load(this, R.raw.coin_arrival, 1);
    getWindow().setStatusBarColor(CREAM);
    getWindow().setNavigationBarColor(WHITE);
    getWindow()
        .getDecorView()
        .setSystemUiVisibility(
            View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
    stage = new FrameLayout(this);
    stage.setBackgroundColor(CREAM);
    setContentView(stage);
    stage.setOnApplyWindowInsetsListener(
        (v, insets) -> {
          v.setPadding(
              insets.getSystemWindowInsetLeft(),
              insets.getSystemWindowInsetTop(),
              insets.getSystemWindowInsetRight(),
              insets.getSystemWindowInsetBottom());
          return insets.consumeSystemWindowInsets();
        });
    // The login surface already carries the brand hero; privacy consent leads there directly.
    continueStartup();
  }

  void continueStartup() {
    welcomeShowing = false;
    confirmPrivacy();
  }

  void welcome() {
    welcomeShowing = true;
    stage.removeAllViews();
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    scroll.setVerticalScrollBarEnabled(false);
    LinearLayout body = col();
    body.setGravity(Gravity.CENTER_HORIZONTAL);
    body.setPadding(dp(27), dp(10), dp(27), dp(18));
    scroll.addView(body);
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));
    LinearLayout brand = row();
    brand.setGravity(Gravity.CENTER);
    add(brand, new LineIcon(this, "gift", RED), 24, 24);
    add(brand, text("  小橙有喜", 22, INK, true), -2, -2);
    add(body, brand, -1, 36);
    WelcomeArt scene = new WelcomeArt(this);
    int sceneHeight =
        Math.min(
            255,
            Math.max(
                180,
                (int)
                        (getResources().getDisplayMetrics().heightPixels
                            / getResources().getDisplayMetrics().density)
                    - 340));
    add(body, scene, -1, sceneHeight);
    gap(body, 5);
    TextView headline = text("把小欢喜\n装进每一天", 31, INK, true);
    headline.setGravity(Gravity.CENTER);
    headline.setLineSpacing(dp(5), 1);
    add(body, headline, -1, -2);
    gap(body, 8);
    TextView subtitle = label("一点小收获，一整天好心情", 13, MUTED);
    subtitle.setGravity(Gravity.CENTER);
    add(body, subtitle, -1, -2);
    gap(body, 16);
    LinearLayout benefits = row();
    String[] icons = {"gift", "shop", "coin"}, labels = {"领福利", "逛好物", "攒金币"};
    for (int i = 0; i < labels.length; i++) {
      LinearLayout item = row();
      item.setGravity(Gravity.CENTER);
      item.setBackground(bg(WHITE, 16));
      LineIcon icon = new LineIcon(this, icons[i], 0xffd27d42);
      add(item, icon, 18, 18);
      add(item, text(" " + labels[i], 12, 0xff93715a, false), -2, -2);
      LinearLayout.LayoutParams itemParams = new LinearLayout.LayoutParams(0, dp(36), 1);
      if (i < 2) itemParams.rightMargin = dp(7);
      benefits.addView(item, itemParams);
    }
    add(body, benefits, -1, -2);
    gap(body, 18);
    add(
        body,
        button(
            "登录 / 注册",
            () -> {
              api.preferences.edit().putBoolean("welcome_seen", true).apply();
              continueStartup();
            }),
        -1,
        54);
    gap(body, 8);
    LinearLayout legal = row();
    legal.setGravity(Gravity.CENTER);
    TextView privacy = label("隐私政策", 11, MUTED), terms = label("用户协议", 11, MUTED);
    privacy.setOnClickListener(v -> openPolicy("/privacy"));
    terms.setOnClickListener(v -> openPolicy("/terms"));
    add(legal, privacy, -2, 26);
    add(legal, label("   ·   ", 11, 0xffcfb9aa), -2, -2);
    add(legal, terms, -2, 26);
    add(body, legal, -1, 26);
  }

  void launchApp() {
    startupAdActive = false;
    if (api.token.isEmpty()) auth("login");
    else loadAccount();
  }

  void confirmPrivacy() {
    if (api.preferences.getBoolean("privacy_accepted", false)
        && api.preferences.getBoolean(DeviceTrustClient.CONSENT, false)) {
      afterPrivacy();
      return;
    }
    loading();
    AlertDialog consent =
        new AlertDialog.Builder(this)
            .setTitle("欢迎来到小橙有喜")
            .setMessage(
                "广西奇易电子商务有限公司为你提供服务。请阅读隐私政策和用户协议。为提供福利、广告与账户安全服务，我们使用穿山甲广告 SDK 和知衡 DeviceTrust"
                    + " SDK，处理必要的设备、系统组件、网络及广告互动信息。登录后记录本应用的页面访问、点击、停留与触点，用于账户安全和服务改进。不会采集输入内容、密码短信、屏幕画面或其他应用信息。")
            .setPositiveButton(
                "同意并继续",
                (dialog, which) -> {
                  api.preferences
                      .edit()
                      .putBoolean("privacy_accepted", true)
                      .putBoolean(DeviceTrustClient.CONSENT, true)
                      .apply();
                  afterPrivacy();
                })
            .setNegativeButton("退出", (dialog, which) -> finish())
            .setNeutralButton("查看政策与协议", null)
            .setCancelable(false)
            .create();
    consent.setOnShowListener(
        dialog ->
            consent
                .getButton(AlertDialog.BUTTON_NEUTRAL)
                .setOnClickListener(
                    view ->
                        new AlertDialog.Builder(this)
                            .setItems(
                                new String[] {"隐私政策", "用户协议"},
                                (choices, index) -> openPolicy(index == 0 ? "/privacy" : "/terms"))
                            .show()));
    consent.show();
  }

  void afterPrivacy() {
    loading();
    api.get(
        "/config",
        result -> {
          try {
            api.deviceTrustEnabled = ((JSONObject) result).getBoolean("device_trust_enabled");
            activities.configure((JSONObject) result);
            api.configurationLoaded = true;
            api.deviceTrust.initialize(
                () -> {
                  servicesReady = true;
                  refreshRuntime();
                  updates.start(
                      () -> {
                        if (BuildConfig.ADS_ENABLED) startSplash();
                        else launchApp();
                      });
                });
          } catch (JSONException error) {
            configurationError("应用与服务版本不兼容，请更新应用后重试。");
          }
        },
        this::configurationError);
  }

  void configurationError(String message) {
    new AlertDialog.Builder(this)
        .setTitle("服务暂不可用")
        .setMessage(message)
        .setPositiveButton("重试", (dialog, which) -> afterPrivacy())
        .setNegativeButton("退出", (dialog, which) -> finish())
        .setCancelable(false)
        .show();
  }

  void startSplash() {
    startupAdActive = true;
    loading();
    csjAds.initialize(
        this,
        () -> {
          FrameLayout host = new FrameLayout(this);
          host.setBackgroundColor(CREAM);
          stage.addView(host, new FrameLayout.LayoutParams(-1, -1));
          csjAds.showSplash(this, host, this::launchApp, this::startupAdError);
        },
        this::startupAdError);
  }

  void startupAdError(String message) {
    new AlertDialog.Builder(this)
        .setTitle("广告加载失败")
        .setMessage(message)
        .setPositiveButton("继续", (dialog, which) -> launchApp())
        .setCancelable(false)
        .show();
  }

  void openPolicy(String path) {
    String url =
        path.equals("/privacy")
            ? BuildConfig.PRIVACY_URL
            : "https://orange-api.lingzidev.com/terms";
    try {
      startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(url)));
    } catch (ActivityNotFoundException failure) {
      error(failure.getMessage());
    }
  }

  int dp(float n) {
    return Math.round(n * getResources().getDisplayMetrics().density);
  }

  GradientDrawable bg(int color, float radius) {
    GradientDrawable d = new GradientDrawable();
    d.setColor(color);
    d.setCornerRadius(dp(radius));
    return d;
  }

  GradientDrawable grad(int a, int b, float radius) {
    GradientDrawable d = new GradientDrawable(GradientDrawable.Orientation.TL_BR, new int[] {a, b});
    d.setCornerRadius(dp(radius));
    return d;
  }

  LinearLayout col() {
    LinearLayout v = new LinearLayout(this);
    v.setOrientation(1);
    return v;
  }

  LinearLayout row() {
    LinearLayout v = new LinearLayout(this);
    v.setGravity(Gravity.CENTER_VERTICAL);
    v.setOrientation(0);
    return v;
  }

  LinearLayout.LayoutParams lp(int w, int h) {
    return new LinearLayout.LayoutParams(w < 0 ? w : dp(w), h < 0 ? h : dp(h));
  }

  TextView text(String s, int size, int color, boolean bold) {
    TextView t = new TextView(this);
    t.setText(s);
    t.setTextSize(size);
    t.setTextColor(color);
    if (bold) t.setTypeface(null, 1);
    t.setIncludeFontPadding(false);
    return t;
  }

  TextView label(String s, int size, int color) {
    return text(s, size, color, false);
  }

  TextView button(String s, Runnable run) {
    TextView b = text(s, 16, WHITE, true);
    b.setGravity(Gravity.CENTER);
    b.setBackground(grad(0xffff7c3e, RED, 18));
    b.setPadding(dp(12), 0, dp(12), 0);
    b.setMinHeight(dp(52));
    b.setOnClickListener(
        v -> {
          press(v);
          run.run();
        });
    return b;
  }

  void press(View v) {
    v.animate()
        .scaleX(.96f)
        .scaleY(.96f)
        .setDuration(70)
        .withEndAction(() -> v.animate().scaleX(1).scaleY(1).setDuration(150).start())
        .start();
  }

  void gap(LinearLayout p, int h) {
    View g = new View(this);
    p.addView(g, lp(1, h));
  }

  void pad(View v, int a) {
    v.setPadding(dp(a), dp(a), dp(a), dp(a));
  }

  void add(LinearLayout p, View v, int w, int h) {
    p.addView(v, lp(w, h));
  }

  void flex(LinearLayout p, View v) {
    p.addView(v, new LinearLayout.LayoutParams(0, -2, 1));
  }

  void line(LinearLayout p) {
    View v = new View(this);
    v.setBackgroundColor(0xfff5ece4);
    add(p, v, -1, 1);
  }

  ImageView art(String name) {
    ImageView v = new ImageView(this);
    int res = getResources().getIdentifier(name.replace('-', '_'), "drawable", getPackageName());
    if (res != 0) v.setImageResource(res);
    v.setScaleType(ImageView.ScaleType.FIT_CENTER);
    return v;
  }

  void error(String message) {
    new AlertDialog.Builder(this)
        .setTitle("操作未完成")
        .setMessage(message)
        .setPositiveButton("知道了", null)
        .show();
  }

  void tip(String message) {
    Toast.makeText(this, message, Toast.LENGTH_SHORT).show();
  }

  void loading() {
    stage.removeAllViews();
    LinearLayout c = col();
    c.setGravity(Gravity.CENTER);
    ProgressBar p = new ProgressBar(this);
    add(c, p, 36, 36);
    gap(c, 20);
    add(c, label("正在打开你的小确幸", 15, MUTED), -2, -2);
    stage.addView(c, new FrameLayout.LayoutParams(-1, -1));
  }

  void loadAccount() {
    loading();
    api.get(
        "/me",
        data -> {
          user = (JSONObject) data;
          enterAccount();
          api.reportLogin(this::error);
        },
        m -> {
          api.saveToken("");
          error(m);
          auth("login");
        });
  }

  void acceptAuth(Object data) {
    JSONObject j = (JSONObject) data;
    api.saveToken(j.optString("token"));
    user = j.optJSONObject("user");
    enterAccount();
    api.reportLogin(this::error);
  }

  void enterAccount() {
    checkDeletionStatus(this::enterAccountHome);
  }

  void enterAccountHome() {
    if (current.equals("deletion")) current = deletionReturn.isEmpty() ? "hall" : deletionReturn;
    deletionPage = false;
    activities.load();
    refreshCheckinStatus();
    if (user.optBoolean("invitation_required")) {
      invitationBinding();
      return;
    }
    home();
    refreshData();
    startHallSession();
  }

  void checkDeletionStatus(Runnable continueAccount) {
    api.get(
        "/account/deletion-status",
        value -> {
          JSONObject status = (JSONObject) value;
          String state = status.optString("status", "none");
          if (state.equals("approved")) {
            finishDeletion();
          } else if (state.equals("pending")) {
            showDeletionPage(status);
          } else {
            if (state.equals("rejected")) tip("注销申请未通过");
            continueAccount.run();
          }
        },
        new Api.Failure() {
          @Override
          public void failed(String message) {
            error(message);
            continueAccount.run();
          }

          @Override
          public void failed(int status, String message) {
            if (status == 401) finishDeletion();
            else failed(message);
          }
        });
  }

  void refreshDeletionStatus() {
    api.get(
        "/account/deletion-status",
        value -> {
          JSONObject status = (JSONObject) value;
          String state = status.optString("status", "none");
          if (state.equals("approved")) finishDeletion();
          else if (state.equals("pending") || state.equals("rejected")) showDeletionPage(status);
          else enterAccountHome();
        },
        new Api.Failure() {
          @Override
          public void failed(String message) {
            error(message);
          }

          @Override
          public void failed(int status, String message) {
            if (status == 401) finishDeletion();
            else failed(message);
          }
        });
  }

  void showDeletionPage(JSONObject status) {
    if (!current.equals("deletion") && !current.equals("deletion_confirm"))
      deletionReturn = current;
    current = "deletion";
    deletionPage = true;
    deletionConfirmPage = false;
    endHallSession();
    page = null;
    stage.removeAllViews();
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    LinearLayout body = col();
    body.setPadding(dp(26), dp(18), dp(26), dp(28));
    scroll.addView(body);
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));

    LinearLayout top = row();
    LineIcon back = new LineIcon(this, "back", MUTED);
    back.setContentDescription("返回账号");
    back.setOnClickListener(v -> enterAccountHome());
    add(top, back, 28, 28);
    flex(top, text("账号状态", 18, INK, true));
    add(body, top, -1, 42);
    gap(body, 22);

    LinearLayout card = col();
    card.setGravity(Gravity.CENTER_HORIZONTAL);
    card.setPadding(dp(24), dp(28), dp(24), dp(24));
    card.setBackground(grad(0xffffead8, 0xfffff4e7, 26));
    String state = status.optString("status", "pending");
    String title = state.equals("rejected") ? "注销申请未通过" : "注销申请审核中";
    String detail =
        state.equals("rejected") ? "账号仍可正常使用，如需注销可以重新提交申请。" : "申请提交后，我们会完成必要审核。审核通过后账号资料将被处理。";
    add(card, new LineIcon(this, state.equals("rejected") ? "notice" : "clock", RED), 48, 48);
    gap(card, 16);
    add(card, text(title, 23, INK, true), -1, -2);
    gap(card, 10);
    TextView copy = label(detail, 13, MUTED);
    copy.setGravity(Gravity.CENTER);
    copy.setLineSpacing(dp(5), 1);
    add(card, copy, -1, -2);
    gap(card, 24);
    add(card, button("刷新状态", this::refreshDeletionStatus), -1, 50);
    add(body, card, -1, -2);
    gap(body, 16);
    TextView backAccount = label("返回账号", 13, MUTED);
    backAccount.setGravity(Gravity.CENTER);
    backAccount.setOnClickListener(v -> enterAccountHome());
    add(body, backAccount, -1, 36);
  }

  void finishDeletion() {
    deletionPage = false;
    deletionConfirmPage = false;
    activities.clear();
    endHallSession();
    api.saveToken("");
    user = null;
    page = null;
    current = "hall";
    tip("账号已注销");
    auth("login");
  }

  void refreshCheckinStatus() {
    api.get(
        "/check-in/status",
        value -> {
          checkinStatus = (JSONObject) value;
          if (current.equals("hall") || current.equals("me")) render();
        },
        message -> android.util.Log.e("CheckIn", "GET /api/check-in/status: " + message));
  }

  void invitationBinding() {
    endHallSession();
    page = null;
    stage.removeAllViews();
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    LinearLayout body = col();
    body.setPadding(dp(28), dp(18), dp(28), dp(28));
    scroll.addView(body);
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));
    TextView back = label("‹  返回登录", 14, MUTED);
    back.setOnClickListener(v -> exitAccount());
    add(body, back, -1, 42);
    gap(body, 20);
    ImageView mascot = art("mascot");
    mascot.setBackground(grad(0xffffead8, CREAM, 30));
    add(body, mascot, -1, 190);
    gap(body, 28);
    add(body, text("绑定邀请码", 29, INK, true), -1, -2);
    gap(body, 12);
    TextView subtitle = label("输入朋友的邀请码，一起开启今日福利", 14, MUTED);
    subtitle.setLineSpacing(dp(4), 1);
    add(body, subtitle, -1, -2);
    gap(body, 30);
    EditText code =
        input("输入 6 位邀请码", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS);
    code.setFilters(
        new InputFilter[] {
          new InputFilter.AllCaps(),
          new InputFilter.LengthFilter(8),
          (source, start, end, dest, dstart, dend) ->
              source.subSequence(start, end).toString().replaceAll("[^A-Z0-9]", "")
        });
    code.setTextSize(22);
    code.setLetterSpacing(.14f);
    add(body, code, -1, 62);
    gap(body, 20);
    TextView bind = button("确认绑定", () -> {});
    bind.setOnClickListener(
        v -> {
          bind.setEnabled(false);
          bind.setAlpha(.65f);
          api.post(
              "/invite/bind",
              Api.object("invite_code", code.getText().toString().trim()),
              result -> {
                user = ((JSONObject) result).optJSONObject("user");
                current = "hall";
                enterAccount();
              },
              message -> {
                bind.setEnabled(true);
                bind.setAlpha(1);
                error(message);
              });
        });
    add(body, bind, -1, 54);
    gap(body, 16);
    TextView leave = label("暂不绑定，返回登录", 13, MUTED);
    leave.setGravity(Gravity.CENTER);
    leave.setOnClickListener(v -> exitAccount());
    add(body, leave, -1, 38);
  }

  void exitAccount() {
    api.post("/auth/logout", Api.object(), result -> {}, this::error);
    activities.clear();
    api.saveToken("");
    endHallSession();
    user = null;
    page = null;
    current = "hall";
    auth("login");
  }

  void auth(String mode) {
    api.behavior.sensitive(true);
    api.behavior.page(mode.equals("register") ? 11 : 10);
    rain.detach();
    hallSurface = null;
    rainTime = null;
    rememberHallScroll();
    hallScroll = null;
    hallMessages = null;
    stage.removeAllViews();
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    scroll.setClipToPadding(false);
    LinearLayout body = col();
    body.setPadding(dp(24), dp(12), dp(24), dp(20));
    scroll.addView(body);
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));
    LinearLayout top = row();
    LineIcon brand = new LineIcon(this, "gift", RED);
    add(top, brand, 24, 24);
    TextView brandText = text("  小橙有喜", 19, INK, true);
    flex(top, brandText);
    TextView back = label(mode.equals("login") ? "自在生活 · 天天有礼" : "返回登录", 12, MUTED);
    back.setOnClickListener(v -> auth("login"));
    add(top, back, -2, 36);
    add(body, top, -1, -2);
    gap(body, 12);
    FrameLayout hero = new FrameLayout(this);
    hero.setBackground(grad(0xffffe8d0, 0xfffff0df, 25));
    LinearLayout heroCopy = col();
    heroCopy.setPadding(dp(17), dp(26), dp(110), dp(15));
    add(heroCopy, text("小欢喜，正在发生", 19, INK, true), -1, -2);
    gap(heroCopy, 10);
    add(heroCopy, label("今天，也给自己一份期待", 11, 0xffad7e59), -1, -2);
    hero.addView(heroCopy, new FrameLayout.LayoutParams(-1, -1));
    ImageView mascot = art("mascot");
    FrameLayout.LayoutParams imageParams =
        new FrameLayout.LayoutParams(dp(112), dp(112), Gravity.RIGHT | Gravity.BOTTOM);
    imageParams.rightMargin = dp(2);
    hero.addView(mascot, imageParams);
    add(body, hero, -1, 112);
    gap(body, 20);
    String title =
        mode.equals("register") ? "创建你的账号" : mode.equals("reset") ? "找回登录密码" : "登录，遇见小欢喜";
    TextView heading = text(title, 24, INK, true);
    add(body, heading, -1, -2);
    gap(body, 8);
    add(
        body,
        label(
            mode.equals("login")
                ? "输入手机号和验证码即可登录"
                : mode.equals("register") ? "用手机号，开启你的好福利" : "验证手机号，设置新密码",
            13,
            MUTED),
        -1,
        -2);
    gap(body, 20);
    EditText phone = input("请输入手机号", InputType.TYPE_CLASS_PHONE);
    phone.setText(BuildConfig.DEBUG && mode.equals("login") ? "13800138000" : "");
    formField(body, "手机号", phone);
    gap(body, 14);
    EditText code = input("验证码", InputType.TYPE_CLASS_NUMBER);
    LinearLayout codeRow = row();
    codeRow.setBackground(bg(WHITE, 16));
    flex(codeRow, code);
    TextView send = text("获取验证码", 13, RED, true);
    send.setPadding(dp(12), 0, dp(16), 0);
    send.setOnClickListener(
        v ->
            api.post(
                "/auth/sms",
                Api.object(
                    "phone",
                    phone.getText().toString(),
                    "purpose",
                    mode.equals("login") ? "login" : mode.equals("reset") ? "reset" : "register"),
                d -> {
                  JSONObject r = (JSONObject) d;
                  code.setText(r.optString("code"));
                  tip("验证码已发送");
                },
                this::error));
    add(codeRow, send, -2, 52);
    formField(body, "验证码", codeRow);
    gap(body, 14);
    final EditText password;
    if (!mode.equals("login")) {
      password =
          input(
              mode.equals("reset") ? "设置新密码（至少 8 位）" : "请输入密码",
              InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
      formField(body, mode.equals("reset") ? "新密码" : "登录密码", passwordField(password));
    } else {
      password = null;
    }
    gap(body, 10);
    if (mode.equals("login")) {
      TextView forgot = label("忘记密码？", 13, MUTED);
      forgot.setGravity(Gravity.RIGHT);
      forgot.setOnClickListener(v -> auth("reset"));
      add(body, forgot, -1, 22);
      gap(body, 12);
    }
    final String[] captchaId = {""};
    final String[] captchaAnswer = {""};
    TextView submit =
        button(
            mode.equals("login") ? "登录，领取今日福利" : mode.equals("register") ? "注册并登录" : "保存新密码",
            () -> {});
    final Runnable[] submitRequest = new Runnable[1];
    submitRequest[0] =
        () -> {
          String path =
              mode.equals("login")
                  ? "/auth/login"
                  : mode.equals("register") ? "/auth/register" : "/auth/reset-password";
          JSONObject request =
              Api.object(
                  "phone",
                  phone.getText().toString(),
                  "code",
                  code.getText().toString(),
                  "password",
                  mode.equals("login") ? "" : password.getText().toString());
          if (mode.equals("login") && !captchaId[0].isEmpty()) {
            try {
              request.put("captcha_id", captchaId[0]);
              request.put("captcha_answer", captchaAnswer[0]);
            } catch (JSONException e) {
              throw new RuntimeException(e);
            }
          }
          api.post(
              path,
              request,
              d -> {
                submit.setEnabled(true);
                submit.setAlpha(1);
                if (mode.equals("reset")) {
                  endHallSession();
                  api.saveToken("");
                  user = null;
                  page = null;
                  tip("密码已更新");
                  auth("login");
                } else acceptAuth(d);
              },
              new Api.Failure() {
                @Override
                public void failed(String message) {
                  submit.setEnabled(true);
                  submit.setAlpha(1);
                  error(message);
                }

                @Override
                public void failed(int status, String message, String errorCode) {
                  if (mode.equals("login") && "LOGIN_CAPTCHA_REQUIRED".equals(errorCode)) {
                    requestLoginCaptcha(phone, submit, captchaId, captchaAnswer, submitRequest[0]);
                    return;
                  }
                  failed(message);
                }
              });
        };
    submit.setOnClickListener(
        v -> {
          submit.setEnabled(false);
          submit.setAlpha(.65f);
          submitRequest[0].run();
        });
    add(body, submit, -1, 52);
    gap(body, 12);
    TextView switchMode = text(mode.equals("login") ? "还没账号？立即注册" : "已有账号？去登录", 14, RED, false);
    switchMode.setGravity(Gravity.CENTER);
    switchMode.setOnClickListener(v -> auth(mode.equals("login") ? "register" : "login"));
    add(body, switchMode, -1, 32);
    gap(body, 2);
    LinearLayout legal = row();
    legal.setGravity(Gravity.CENTER);
    TextView privacy = label("隐私政策", 11, MUTED), terms = label("用户协议", 11, MUTED);
    privacy.setOnClickListener(v -> openPolicy("/privacy"));
    terms.setOnClickListener(v -> openPolicy("/terms"));
    add(legal, privacy, -2, 28);
    add(legal, label("   ·   ", 11, 0xffcfb9aa), -2, -2);
    add(legal, terms, -2, 28);
    add(body, legal, -1, 28);
    if (BuildConfig.DEBUG && mode.equals("login")) {
      gap(body, 6);
    }
  }

  void requestLoginCaptcha(
      EditText phone, TextView submit, String[] captchaId, String[] captchaAnswer, Runnable retry) {
    api.post(
        "/auth/login/captcha",
        Api.object("phone", phone.getText().toString()),
        value -> {
          JSONObject response = (JSONObject) value;
          if (!response.optBoolean("required")) {
            submit.setEnabled(true);
            submit.setAlpha(1);
            error("请重新获取验证码后登录");
            return;
          }
          JSONObject challenge = response.optJSONObject("captcha");
          if (challenge == null || challenge.optString("captcha_id").isEmpty()) {
            submit.setEnabled(true);
            submit.setAlpha(1);
            error("安全验证暂不可用");
            return;
          }
          LinearLayout card = col();
          card.setPadding(dp(22), dp(18), dp(22), dp(8));
          add(card, text("安全验证", 20, INK, true), -1, -2);
          gap(card, 7);
          add(card, label(challenge.optString("question"), 14, MUTED), -1, -2);
          gap(card, 12);
          EditText answer = input("请输入答案", InputType.TYPE_CLASS_NUMBER);
          add(card, answer, -1, 52);
          AlertDialog dialog =
              new AlertDialog.Builder(this)
                  .setView(card)
                  .setNegativeButton("取消", null)
                  .setPositiveButton("继续", null)
                  .create();
          dialog.setOnShowListener(
              ignored ->
                  dialog
                      .getButton(AlertDialog.BUTTON_POSITIVE)
                      .setOnClickListener(
                          ignoredButton -> {
                            String result = answer.getText().toString().trim();
                            if (result.isEmpty()) {
                              tip("请输入验证答案");
                              return;
                            }
                            captchaId[0] = challenge.optString("captcha_id");
                            captchaAnswer[0] = result;
                            dialog.dismiss();
                            retry.run();
                          }));
          dialog.setOnCancelListener(
              ignored -> {
                submit.setEnabled(true);
                submit.setAlpha(1);
              });
          dialog.show();
        },
        message -> {
          submit.setEnabled(true);
          submit.setAlpha(1);
          error(message);
        });
  }

  void formField(LinearLayout parent, String name, View field) {
    add(parent, text(name, 13, 0xff816759, true), -1, -2);
    gap(parent, 6);
    add(parent, field, -1, 52);
  }

  View passwordField(EditText password) {
    FrameLayout wrapper = new FrameLayout(this);
    wrapper.setBackground(bg(WHITE, 16));
    password.setBackground(null);
    password.setPadding(dp(18), 0, dp(54), 0);
    wrapper.addView(password, new FrameLayout.LayoutParams(-1, -1));
    LineIcon eye = new LineIcon(this, "eye_off", 0xffb7a092);
    FrameLayout toggle = new FrameLayout(this);
    toggle.addView(eye, new FrameLayout.LayoutParams(dp(24), dp(24), Gravity.CENTER));
    FrameLayout.LayoutParams ep =
        new FrameLayout.LayoutParams(dp(44), dp(44), Gravity.RIGHT | Gravity.CENTER_VERTICAL);
    ep.rightMargin = dp(5);
    wrapper.addView(toggle, ep);
    toggle.setContentDescription("显示密码");
    eye.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
    final boolean[] visible = {false};
    toggle.setOnClickListener(
        v -> {
          visible[0] = !visible[0];
          password.setTransformationMethod(
              visible[0]
                  ? android.text.method.HideReturnsTransformationMethod.getInstance()
                  : android.text.method.PasswordTransformationMethod.getInstance());
          password.setSelection(password.length());
          eye.kind = visible[0] ? "eye" : "eye_off";
          toggle.setContentDescription(visible[0] ? "隐藏密码" : "显示密码");
          eye.invalidate();
        });
    return wrapper;
  }

  EditText input(String placeholder, int type) {
    EditText e = new EditText(this);
    e.setSingleLine();
    e.setTextSize(15);
    e.setTextColor(INK);
    e.setHintTextColor(0xffc1ab9d);
    e.setHint(placeholder);
    e.setInputType(type);
    e.setBackground(bg(WHITE, 16));
    e.setPadding(dp(18), 0, dp(16), 0);
    return e;
  }

  void home() {
    if (user != null && user.optBoolean("invitation_required")) {
      invitationBinding();
      return;
    }
    stage.removeAllViews();
    shell = col();
    stage.addView(shell, new FrameLayout.LayoutParams(-1, -1));
    header = row();
    header.setPadding(dp(22), dp(8), dp(20), dp(8));
    add(shell, header, -1, 66);
    page = col();
    shell.addView(page, new LinearLayout.LayoutParams(-1, 0, 1));
    tabs = row();
    tabs.setBackgroundColor(WHITE);
    tabs.setElevation(dp(8));
    tabs.setPadding(dp(14), dp(5), dp(14), dp(5));
    add(shell, tabs, -1, 72);
    render();
  }

  void refreshData() {
    api.get(
        "/tasks",
        d -> {
          updateTasks((JSONArray) d);
          if (current.equals("hall")) render();
        },
        this::error);
    refreshProducts();
  }

  void refreshProducts() {
    if (productsLoading && productsRequestToken.equals(api.token)) return;
    productsLoading = true;
    productLoadError = "";
    String session = api.token;
    productsRequestToken = session;
    api.get(
        "/products",
        data -> {
          if (!session.equals(productsRequestToken)) return;
          productsLoading = false;
          if (!session.equals(api.token) || user == null || isDestroyed()) return;
          products = (JSONArray) data;
          if (current.equals("shop")) render();
        },
        message -> {
          if (!session.equals(productsRequestToken)) return;
          productsLoading = false;
          if (!session.equals(api.token) || user == null || isDestroyed()) return;
          productLoadError = message == null ? "商品列表请求失败" : message;
          android.util.Log.e("Products", "GET /api/products: " + message);
          if (current.equals("shop")) render();
        });
  }

  void refreshProductPromo() {
    api.get(
        "/products/config",
        value -> {
          if (value instanceof JSONObject) {
            JSONObject config = (JSONObject) value;
            shopPromoImage = config.optString("promo_image_url", config.optString("image", ""));
            if (current.equals("shop")) render();
          }
        },
        message -> android.util.Log.e("ProductPromo", "GET /api/products/config: " + message));
  }

  void updateTasks(JSONArray latest) {
    tasks = latest;
    for (JSONObject item : hallItems) {
      JSONObject packet = item.optJSONObject("task");
      if (packet == null) continue;
      for (int i = 0; i < tasks.length(); i++) {
        JSONObject task = tasks.optJSONObject(i);
        if (packet.optString("id").equals(task.optString("id"))) {
          try {
            item.put("task", task);
          } catch (JSONException failure) {
            throw new IllegalArgumentException(failure);
          }
          break;
        }
      }
    }
  }

  void startHallSession() {
    endHallSession();
    if (current.equals("hall")) render();
    hallStream = new HallStream(this, api.token, this::appendHallItem, this::error);
  }

  void endHallSession() {
    rewardUiBusy = false;
    closeRewardStatus();
    pendingReward = null;
    rain.reset();
    if (hallStream != null) hallStream.close();
    hallStream = null;
    hallItems.clear();
    hallScrollY = 0;
    hallScroll = null;
    hallMessages = null;
    nativeFeeds.destroyAll();
  }

  void removeHallItem(String id) {
    hallItems.removeIf(item -> item.optString("id").equals(id));
    if (current.equals("hall") && page != null) render();
  }

  void rememberHallScroll() {
    if (hallScroll != null) hallScrollY = hallScroll.getScrollY();
  }

  void appendHallItem(JSONObject item) {
    hallItems.add(item);
    if (hallMessages == null || hallScroll == null) return;
    boolean nearBottom =
        hallScroll.getChildAt(0).getHeight() - hallScroll.getHeight() - hallScroll.getScrollY()
            <= dp(72);
    hallItem(hallMessages, item, hallItems.size() - 1);
    if (nearBottom) {
      ScrollView scroll = hallScroll;
      scroll.post(() -> scroll.fullScroll(View.FOCUS_DOWN));
    }
  }

  void refreshUser() {
    api.get(
        "/me",
        d -> {
          user = (JSONObject) d;
          render();
        },
        this::error);
  }

  void render() {
    if (page == null || user == null) return;
    api.behavior.sensitive(false);
    api.behavior.page(current.equals("hall") ? 1 : current.equals("shop") ? 2 : 3);
    rain.detach();
    hallSurface = null;
    rainTime = null;
    hallNotice = null;
    checkInButton = null;
    rememberHallScroll();
    hallScroll = null;
    hallMessages = null;
    header.removeAllViews();
    header.setLayoutParams(lp(-1, current.equals("hall") ? 58 : 66));
    page.removeAllViews();
    tabs.removeAllViews();
    TextView title =
        text(
            current.equals("hall") ? "小橙有喜" : current.equals("shop") ? "捡漏好物" : "我的",
            25,
            INK,
            true);
    flex(header, title);
    if (!current.equals("me")) {
      balance = text("● " + number(user.optInt("coins")), 14, RED, true);
      balance.setPadding(dp(13), dp(9), dp(13), dp(9));
      balance.setBackground(bg(PALE, 20));
      balance.setOnClickListener(
          v -> {
            current = "me";
            render();
          });
      add(header, balance, -2, -2);
    } else {
      TextView settings = label("设置", 14, MUTED);
      settings.setOnClickListener(v -> settings());
      add(header, settings, -2, 40);
    }
    if (current.equals("hall")) hall();
    else if (current.equals("shop")) shop();
    else mine();
    nav("hall", "大厅");
    nav("shop", "捡漏");
    nav("me", "我的");
  }

  void nav(String key, String title) {
    LinearLayout item = col();
    item.setGravity(Gravity.CENTER);
    int color = current.equals(key) ? RED : 0xffb4a299;
    LineIcon icon = new LineIcon(this, key, color);
    add(item, icon, 24, 24);
    gap(item, 5);
    add(item, text(title, 11, color, current.equals(key)), -2, -2);
    tabs.addView(item, new LinearLayout.LayoutParams(0, -1, 1));
    item.setOnClickListener(
        v -> {
          api.behavior.click(key.equals("hall") ? 3 : key.equals("shop") ? 4 : 5, "");
          current = key;
          if (key.equals("shop")) {
            refreshProducts();
            refreshProductPromo();
          }
          render();
        });
  }

  LinearLayout scrollPage() {
    ScrollView sc = new ScrollView(this);
    sc.setFillViewport(true);
    sc.setClipToPadding(false);
    sc.setVerticalScrollBarEnabled(false);
    LinearLayout content = col();
    content.setPadding(dp(20), dp(6), dp(20), dp(22));
    sc.addView(content);
    add(page, sc, -1, -1);
    return content;
  }

  String number(int n) {
    return new DecimalFormat("#,###").format(n);
  }

  String money(double n) {
    return String.format(Locale.CHINA, "%.2f", n);
  }

  void hall() {
    addActivityEntry();
    hallNotice = text(hallNoticeText, 12, 0xffa96a3e, false);
    hallNotice.setPadding(dp(12), dp(8), dp(12), dp(8));
    hallNotice.setBackground(bg(0xffffead5, 12));
    hallNotice.setMaxLines(2);
    hallNotice.setEllipsize(android.text.TextUtils.TruncateAt.END);
    hallNotice.setVisibility(hallNoticeText.isEmpty() ? View.GONE : View.VISIBLE);
    hallNotice.setOnClickListener(
        v ->
            new AlertDialog.Builder(this)
                .setTitle("公告")
                .setMessage(hallNoticeText)
                .setPositiveButton("知道了", null)
                .show());
    LinearLayout.LayoutParams noticeParams = lp(-1, -2);
    noticeParams.setMargins(dp(20), 0, dp(20), dp(4));
    page.addView(hallNotice, noticeParams);
    LinearLayout rainStatus = row();
    rainStatus.setPadding(dp(12), 0, dp(12), 0);
    rainStatus.setBackground(bg(WHITE, 14));
    add(rainStatus, new LineIcon(this, "gift", 0xffd68a45), 18, 18);
    TextView rainLabel = text("  红包雨", 12, INK, true);
    flex(rainStatus, rainLabel);
    rainTime = text("正在迎接小欢喜", 12, RED, true);
    add(rainStatus, rainTime, -2, 38);
    LinearLayout.LayoutParams clockParams = lp(-1, 40);
    clockParams.setMargins(dp(20), dp(4), dp(20), dp(8));
    page.addView(rainStatus, clockParams);
    ScrollView hallScroller = new ScrollView(this);
    hallScroller.setFillViewport(true);
    hallScroller.setClipToPadding(false);
    hallScroller.setVerticalScrollBarEnabled(false);
    LinearLayout content = col();
    content.setPadding(dp(20), dp(6), dp(20), dp(22));
    hallScroller.addView(content);
    hallSurface = new FrameLayout(this);
    hallSurface.addView(hallScroller, new FrameLayout.LayoutParams(-1, -1));
    page.addView(hallSurface, new LinearLayout.LayoutParams(-1, 0, 1));
    FrameLayout welcome = new FrameLayout(this);
    welcome.setBackground(grad(0xffffe2c7, 0xffffeedc, 22));
    LinearLayout copy = col();
    copy.setPadding(dp(17), dp(12), dp(98), dp(12));
    add(copy, text("好福利，刚刚好", 20, INK, true), -1, -2);
    gap(copy, 6);
    add(copy, label("把今天的小惊喜装进口袋", 11, 0xffa8744b), -1, -2);
    gap(copy, 4);
    checkInButton = new CheckInButton(this, user.optBoolean("checked_in_today"));
    checkInButton.setBusy(signBusy);
    checkInButton.setOnClickListener(v -> openCheckin());
    add(copy, checkInButton, -2, 33);
    welcome.addView(copy, new FrameLayout.LayoutParams(-1, -2));
    ImageView mascot = art("mascot");
    FrameLayout.LayoutParams mp =
        new FrameLayout.LayoutParams(dp(112), dp(112), Gravity.RIGHT | Gravity.BOTTOM);
    mp.rightMargin = dp(4);
    welcome.addView(mascot, mp);
    add(content, welcome, -1, 110);
    gap(content, 8);
    TextView today = label("今天", 11, 0xffbda89b);
    today.setGravity(Gravity.CENTER);
    add(content, today, -1, 20);
    gap(content, 6);
    hallMessages = col();
    add(content, hallMessages, -1, -2);
    for (int i = 0; i < hallItems.size(); i++) hallItem(hallMessages, hallItems.get(i), i);
    hallScroll = (ScrollView) content.getParent();
    ScrollView scroll = hallScroll;
    int savedScroll = hallScrollY;
    scroll.post(() -> scroll.scrollTo(0, savedScroll));
    if (foreground) rain.attach(hallSurface, rainTime);
  }

  void addActivityEntry() {
    if (activities.config == null) return;
    LinearLayout entry = row();
    entry.setPadding(dp(14), dp(10), dp(14), dp(10));
    entry.setBackground(grad(0xffffd9a8, 0xffffb765, 18));
    LinearLayout copy = col();
    add(copy, text(activities.config.title, 16, INK, true), -1, -2);
    gap(copy, 4);
    add(copy, label(activities.config.subtitle, 11, 0xff8e5a37), -1, -2);
    flex(entry, copy);
    add(entry, text(activities.config.entryLabel, 12, RED, true), -2, 34);
    LinearLayout.LayoutParams params = lp(-1, -2);
    params.setMargins(dp(20), dp(4), dp(20), dp(8));
    page.addView(entry, params);
    entry.setOnClickListener(v -> openActivity());
  }

  void openActivity() {
    ActivityConfig config = activities.config;
    if (config == null) return;
    LinearLayout card = col();
    card.setPadding(dp(22), dp(22), dp(22), dp(22));
    card.setBackground(bg(CREAM, 24));
    add(card, text(config.title, 22, INK, true), -1, -2);
    gap(card, 8);
    add(card, label(config.subtitle, 13, MUTED), -1, -2);
    if (!config.heroImage.isEmpty())
      add(
          card,
          new ProductImage(this, config.heroImage, config.title, ImageView.ScaleType.CENTER_CROP),
          -1,
          160);
    gap(card, 16);
    ActivityWheelView wheel = new ActivityWheelView(this);
    add(card, wheel, -1, 190);
    gap(card, 8);
    add(
        card,
        label("可用机会：" + config.opportunities + (config.canSpin ? "" : "（暂不可抽奖）"), 13, MUTED),
        -1,
        -2);
    gap(card, 12);
    Dialog dialog = sheet(card, 1);
    TextView spin =
        button(config.spinPath.isEmpty() || !config.canSpin ? "活动详情" : "开始转盘", () -> {});
    spin.setOnClickListener(
        v -> {
          if (config.spinPath.isEmpty() || !config.canSpin) return;
          spin.setEnabled(false);
          wheel.spin();
          activities.spin(
              result -> {
                spin.setEnabled(true);
                showActivityResult(dialog, (JSONObject) result);
              },
              message -> {
                spin.setEnabled(true);
                error(message);
              });
        });
    add(card, spin, -1, 50);
    gap(card, 8);
    add(card, label("奖励以服务端结算结果为准，部分活动需要登录、邀请或完成激励视频。", 11, MUTED), -1, -2);
    org.json.JSONArray pending = config.raw.optJSONArray("pending_rewards");
    if (pending != null && pending.length() > 0) {
      gap(card, 12);
      add(card, text("待结算奖励", 14, INK, true), -1, -2);
      for (int i = 0; i < pending.length(); i++) {
        JSONObject item = pending.optJSONObject(i);
        if (item == null) continue;
        String id = item.optString("id"), state = item.optString("status", "pending");
        TextView settle =
            label(
                item.optInt("prize_coins") + " 金币 · " + (state.equals("settled") ? "已到账" : "待结算"),
                12,
                MUTED);
        settle.setPadding(0, dp(7), 0, dp(7));
        if (!state.equals("settled"))
          settle.setOnClickListener(
              v ->
                  activities.settle(
                      id,
                      result -> {
                        showActivityResult(dialog, (JSONObject) result);
                        activities.load();
                      },
                      this::error));
        add(card, settle, -1, -2);
      }
    }
    dialog.show();
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(40), dp(390)), -2);
  }

  void showActivityResult(Dialog dialog, JSONObject result) {
    String state = result.optString("status", result.optString("state", "pending"));
    String reward = result.has("reward") ? result.optString("reward") : "";
    String message = result.optString("message", "活动结果已提交");
    if (state.equals("pending") || state.equals("settlement")) message += "\n奖励待结算";
    if (!reward.isEmpty()) message += "\n奖励：" + reward;
    if (result.optBoolean("threshold_required")) message += "\n达到活动门槛后可领取";
    new AlertDialog.Builder(this)
        .setTitle("活动结果")
        .setMessage(message)
        .setPositiveButton("知道了", null)
        .show();
  }

  void hallItem(LinearLayout content, JSONObject item, int index) {
    if (index > 0) gap(content, 14);
    String kind = item.optString("kind");
    String message = item.optString("text");
    if (!message.isEmpty() && !kind.equals("native_ad")) chatText(content, message, true);
    switch (kind) {
      case "text":
        break;
      case "red_packet":
        if (!message.isEmpty()) gap(content, 10);
        packetMessage(content, item.optJSONObject("task"), index);
        break;
      case "product":
        if (!message.isEmpty()) gap(content, 10);
        JSONObject product = item.optJSONObject("product");
        if (product == null) break;
        LinearLayout productRow = row();
        add(productRow, new View(this), 48, 1);
        productRow.addView(productCard(product), new LinearLayout.LayoutParams(0, -2, 1));
        add(content, productRow, -1, -2);
        break;
      case "native_ad":
        float widthDp =
            getResources().getDisplayMetrics().widthPixels
                    / getResources().getDisplayMetrics().density
                - 64;
        add(
            content,
            nativeFeeds.card(item.optString("id"), item.optString("ad_code_id"), widthDp),
            -1,
            -2);
        break;
    }
  }

  void chatText(LinearLayout content, String message, boolean compact) {
    LinearLayout r = row();
    r.setGravity(Gravity.TOP);
    ImageView avatar = art("mascot");
    avatar.setBackground(bg(0xffffe5cc, 13));
    add(r, avatar, 38, 38);
    LinearLayout bubble = col();
    LinearLayout.LayoutParams bp = new LinearLayout.LayoutParams(0, -2, 1);
    bp.leftMargin = dp(10);
    r.addView(bubble, bp);
    if (!compact) {
      add(bubble, label("小橙助手", 11, MUTED), -1, -2);
      gap(bubble, 7);
    }
    TextView t = label(message, 13, 0xff715b4d);
    t.setLineSpacing(dp(4), 1);
    t.setPadding(dp(14), dp(12), dp(14), dp(12));
    t.setBackground(bg(WHITE, 15));
    add(bubble, t, -1, -2);
    add(content, r, -1, -2);
  }

  void packetMessage(LinearLayout content, JSONObject task, int index) {
    LinearLayout r = row();
    View spacer = new View(this);
    add(r, spacer, 48, 1);
    FrameLayout card = new FrameLayout(this);
    card.setBackground(grad(index % 2 == 0 ? 0xffff7040 : 0xffff8b49, 0xffee442c, 20));
    card.setElevation(dp(3));
    LinearLayout stack = col();
    stack.setPadding(dp(17), dp(14), dp(14), dp(10));
    TextView badge = text(index == 0 ? "今日专属" : "好运加码", 10, 0xffffe7c5, false);
    add(stack, badge, -2, -2);
    gap(stack, 9);
    LinearLayout center = row();
    LinearLayout left = col();
    add(left, text(task.optString("title"), 19, WHITE, true), -1, -2);
    gap(left, 7);
    TextView sub = label(task.optString("subtitle"), 11, 0xffffddbd);
    sub.setMaxLines(2);
    add(left, sub, -1, -2);
    gap(left, 16);
    TextView pill = text(packetAmount(task), 11, 0xffffefb9, true);
    pill.setBackground(bg(0x29ffffff, 12));
    pill.setPadding(dp(9), dp(5), dp(9), dp(5));
    add(left, pill, -2, -2);
    center.addView(left, new LinearLayout.LayoutParams(0, -2, 1));
    ImageView packet = art("packet");
    add(center, packet, 93, 100);
    add(stack, center, -1, -2);
    gap(stack, 10);
    View divider = new View(this);
    divider.setBackgroundColor(0x25ffffff);
    add(stack, divider, -1, 1);
    gap(stack, 9);
    LinearLayout foot = row();
    flex(foot, label("小橙红包", 10, 0xffffdfc5));
    add(
        foot,
        text(task.optInt("claimed_today") > 0 ? "再领一份  ›" : "拆开看看  ›", 11, WHITE, true),
        -2,
        -2);
    add(stack, foot, -1, -2);
    card.addView(stack, new FrameLayout.LayoutParams(-1, -2));
    r.addView(card, new LinearLayout.LayoutParams(0, -2, 1));
    add(content, r, -1, -2);
    card.setOnClickListener(v -> openPacket(task));
    card.setAlpha(0);
    card.setTranslationY(dp(10));
    card.animate().alpha(1).translationY(0).setDuration(300).setStartDelay(index * 70L).start();
  }

  Dialog sheet(LinearLayout content) {
    return sheet(content, current.equals("hall") ? 1 : current.equals("shop") ? 2 : 3);
  }

  Dialog sheet(LinearLayout content, int screen) {
    Dialog d = new Dialog(this);
    d.getWindow();
    d.setContentView(content);
    Window w = d.getWindow();
    if (w != null) {
      w.setBackgroundDrawableResource(android.R.color.transparent);
      w.setDimAmount(.55f);
      w.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
      w.setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(40), dp(390)), -2);
      w.setGravity(Gravity.CENTER);
    }
    api.behavior.dialog(d, screen);
    return d;
  }

  void openPacket(JSONObject task) {
    api.behavior.click(1, task.optString("id"));
    if (updates.required()) {
      updates.maybePresent();
      return;
    }
    rewardUiBusy = true;
    String token = api.token;
    api.quoteReward(
        task.optString("id"),
        data -> {
          if (isDestroyed() || !token.equals(api.token)) return;
          try {
            JSONObject quote = (JSONObject) data;
            JSONObject quotedTask = quote.getJSONObject("task");
            String quoteId = quote.getString("quote_id");
            int reward = quote.getInt("reward");
            boolean updated = false;
            for (JSONObject item : hallItems) {
              JSONObject packet = item.optJSONObject("task");
              if (packet != null && packet.optString("id").equals(quotedTask.getString("id"))) {
                item.put("task", quotedTask);
                updated = true;
              }
            }
            if (updated && current.equals("hall")) render();
            showPacket(quotedTask, quoteId, reward);
          } catch (JSONException error) {
            rewardError(error.getMessage());
          }
        },
        message -> {
          if (!isDestroyed() && token.equals(api.token)) rewardError(message);
        });
  }

  String packetAmount(JSONObject task) {
    int reward = task.optInt("reward"),
        minimum = task.optInt("reward_min", reward),
        maximum = task.optInt("reward_max", reward);
    return (minimum == maximum ? Integer.toString(minimum) : minimum + "～" + maximum) + " 金币";
  }

  void showPacket(JSONObject task, String quoteId, int reward) {
    api.behavior.business(9, task.optString("id"));
    final boolean[] showingAd = {false};
    boolean golden = task.optString("id").startsWith("rain:");
    String rainPacket = golden ? task.optString("id").substring(5) : "";
    LinearLayout card = col();
    card.setGravity(Gravity.CENTER_HORIZONTAL);
    card.setPadding(dp(25), dp(22), dp(25), dp(22));
    GradientDrawable packetBackground = grad(0xffff7646, 0xffe94029, 28);
    if (golden) packetBackground.setStroke(dp(3), 0xffffd57b);
    card.setBackground(packetBackground);
    TextView top = text("小橙助手送来一个红包", 13, 0xffffdfbd, false);
    add(card, top, -1, -2);
    top.setGravity(Gravity.CENTER);
    gap(card, 20);
    add(card, text(golden ? "金喜大红包" : "一份好运，请查收", 25, 0xffffefcf, true), -2, -2);
    gap(card, 4);
    FrameLayout artBox = new FrameLayout(this);
    ImageView packet = art(golden ? "rain_large" : "packet");
    artBox.addView(packet, new FrameLayout.LayoutParams(-1, -1));
    add(card, artBox, 210, 210);
    TextView prize = text(reward + " 金币", 29, 0xffffe9a3, true);
    add(card, prize, -2, -2);
    gap(card, 10);
    add(card, label("看完视频，奖励就能装进口袋", 12, 0xffffd8b4), -2, -2);
    gap(card, 23);
    TextView watch = text("看视频领 " + reward + " 金币", 16, 0xffa44b1f, true);
    watch.setGravity(Gravity.CENTER);
    watch.setBackground(grad(0xffffefc6, 0xffffcf75, 18));
    add(card, watch, -1, 52);
    gap(card, 14);
    TextView close = label("稍后再领", 12, 0xffffd8bd);
    add(card, close, -2, 24);
    Dialog dialog = sheet(card, 4);
    close.setOnClickListener(
        v -> {
          api.behavior.click(12, "");
          dialog.dismiss();
        });
    ObjectAnimator shake = ObjectAnimator.ofFloat(packet, View.ROTATION, -4, 4, -4);
    shake.setDuration(1600);
    shake.setRepeatCount(ValueAnimator.INFINITE);
    shake.setInterpolator(new AccelerateDecelerateInterpolator());
    dialog.setOnDismissListener(
        d -> {
          shake.cancel();
          rain.releasePacket(rainPacket);
          if (!showingAd[0]) rewardUiBusy = false;
        });
    watch.setOnClickListener(
        v -> {
          api.behavior.click(11, task.optString("id"));
          watch.setEnabled(false);
          api.post(
              "/rewards/start",
              Api.object("task_id", task.optString("id"), "quote_id", quoteId),
              data -> {
                JSONObject session = (JSONObject) data;
                packet
                    .animate()
                    .rotationY(90)
                    .scaleX(1.08f)
                    .scaleY(1.08f)
                    .setDuration(220)
                    .withEndAction(
                        () -> {
                          showingAd[0] = true;
                          dialog.dismiss();
                          showRewardAd(session, rainPacket);
                        })
                    .start();
              },
              m -> {
                watch.setEnabled(true);
                error(m);
              });
        });
    dialog.show();
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(48), dp(360)), -2);
    shake.start();
  }

  void showRewardAd(JSONObject session, String rainPacket) {
    String token = api.token, id = session.optString("id");
    api.behavior.external(5);
    api.behavior.business(10, id);
    rewarded.show(
        this,
        session,
        notice -> {
          if (isDestroyed() || !token.equals(api.token)) return;
          api.behavior.business(11, id);
          api.behavior.external(0);
          if (!BuildConfig.ADS_ENABLED) {
            if (notice.valid) completeReward(session, rainPacket);
            else {
              rewardUiBusy = false;
              tip("下次看完视频，就能领取这份奖励");
            }
          } else queryReward(session, notice, rainPacket);
        },
        event -> {
          if (isDestroyed() || !token.equals(api.token)) return;
          api.post(
              "/rewards/" + id + "/client-event",
              (JSONObject) event,
              ignored -> {},
              message -> {
                if (!isDestroyed() && token.equals(api.token)) tip("广告记录提交失败，请稍后重试。\n奖励编号：" + id);
              });
        },
        message -> {
          if (!isDestroyed() && token.equals(api.token)) {
            api.behavior.external(0);
            rewardError(message + "\n奖励编号：" + id);
          }
        });
  }

  void queryReward(JSONObject session, RewardedAdProvider.Notice notice, String rainPacket) {
    String token = api.token, id = session.optString("id");
    rewardUiBusy = true;
    if (rewardStatusDialog != null)
      rewardStatusDialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(false);
    api.get(
        "/rewards/" + id + "/status",
        value -> {
          if (isDestroyed() || !token.equals(api.token)) return;
          try {
            JSONObject status = (JSONObject) value;
            if (status.getInt("reward") != session.getInt("reward")) {
              closeRewardStatus();
              rewardError("奖励信息不一致，请联系客服。\n奖励编号：" + id);
              return;
            }
            switch (status.getString("status")) {
              case "verified":
              case "completed":
                closeRewardStatus();
                pendingReward = null;
                completeReward(session, rainPacket);
                break;
              case "pending":
                showRewardPending(session, notice, rainPacket, null);
                break;
              default:
                closeRewardStatus();
                rewardError("奖励状态异常，请联系客服。\n奖励编号：" + id);
            }
          } catch (JSONException error) {
            closeRewardStatus();
            rewardError(error.getMessage() + "\n奖励编号：" + id);
          }
        },
        message -> {
          if (!isDestroyed() && token.equals(api.token))
            showRewardPending(session, notice, rainPacket, "服务连接失败，请稍后再查询。");
        });
  }

  void showRewardPending(
      JSONObject session,
      RewardedAdProvider.Notice notice,
      String rainPacket,
      String requestError) {
    pendingReward = new PendingReward(session, notice, rainPacket, requestError);
    if (!foreground) return;
    String message =
        requestError != null
            ? requestError
            : notice.hasError()
                ? "本次广告奖励未通过确认，请稍后查询。"
                : notice.skipped && !notice.valid && !notice.complete
                    ? "本次领取尚未完成，可查询奖励状态。"
                    : "广告奖励待确认，请稍后再查询。";
    message += "\n奖励编号：" + session.optString("id");
    if (rewardStatusDialog != null) {
      rewardStatusDialog.setMessage(message);
      rewardStatusDialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(true);
      return;
    }
    AlertDialog dialog =
        new AlertDialog.Builder(this)
            .setTitle(notice.hasError() ? "奖励未确认" : "奖励待确认")
            .setMessage(message)
            .setPositiveButton("查询奖励", null)
            .setNegativeButton("稍后", null)
            .create();
    rewardStatusDialog = dialog;
    api.behavior.dialog(dialog, 4);
    dialog.setOnDismissListener(
        ignored -> {
          if (rewardStatusDialog == dialog) rewardStatusDialog = null;
          if (foreground) {
            pendingReward = null;
            rewardUiBusy = false;
          }
        });
    dialog.setOnShowListener(
        ignored ->
            dialog
                .getButton(AlertDialog.BUTTON_POSITIVE)
                .setOnClickListener(v -> queryReward(session, notice, rainPacket)));
    dialog.show();
  }

  private static final class PendingReward {
    final JSONObject session;
    final RewardedAdProvider.Notice notice;
    final String rainPacket, requestError;

    PendingReward(
        JSONObject session,
        RewardedAdProvider.Notice notice,
        String rainPacket,
        String requestError) {
      this.session = session;
      this.notice = notice;
      this.rainPacket = rainPacket;
      this.requestError = requestError;
    }
  }

  void closeRewardStatus() {
    if (rewardStatusDialog != null) {
      rewardStatusDialog.dismiss();
      rewardStatusDialog = null;
    }
  }

  void completeReward(JSONObject session, String rainPacket) {
    rewardUiBusy = true;
    String token = api.token, id = session.optString("id");
    api.post(
        "/rewards/" + id + "/complete",
        Api.object(),
        earned -> {
          if (isDestroyed() || !token.equals(api.token)) return;
          JSONObject result = (JSONObject) earned;
          if (!rainPacket.isEmpty()) rain.markClaimed(rainPacket);
          user = result.optJSONObject("user");
          render();
          if (foreground) {
            if (!result.optBoolean("already_completed")) celebrate(result.optInt("reward"));
            else {
              rewardUiBusy = false;
              tip("这份奖励已到账");
            }
          } else rewardUiBusy = false;
          api.get(
              "/tasks",
              updated -> {
                if (isDestroyed() || !token.equals(api.token)) return;
                updateTasks((JSONArray) updated);
                if (current.equals("hall")) render();
              },
              this::error);
        },
        message -> {
          if (!isDestroyed() && token.equals(api.token)) rewardError(message + "\n奖励编号：" + id);
        });
  }

  void celebrate(int amount) {
    rewardUiBusy = true;
    if (amount > 0) playCoinArrival();
    LinearLayout card = col();
    card.setPadding(dp(24), dp(24), dp(24), dp(24));
    card.setGravity(Gravity.CENTER_HORIZONTAL);
    card.setBackground(grad(0xfffff5df, WHITE, 30));
    TextView title = text("好心情，已到账", 24, INK, true);
    add(card, title, -2, -2);
    gap(card, 4);
    ImageView image = art("reward");
    add(card, image, 235, 210);
    TextView reward = text("+ " + amount, 44, RED, true);
    add(card, reward, -2, -2);
    gap(card, 8);
    add(card, label("金币已存入你的账户", 13, MUTED), -2, -2);
    gap(card, 25);
    Dialog dialog = sheet(card, 14);
    final boolean[] flying = {false};
    dialog.setOnDismissListener(
        ignored -> {
          if (!flying[0]) rewardUiBusy = false;
        });
    TextView receive =
        button(
            "开心收下",
            () -> {
              api.behavior.click(12, "");
              flying[0] = true;
              dialog.dismiss();
              flyCoins();
            });
    add(card, receive, -1, 52);
    dialog.show();
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(48), dp(360)), -2);
    card.setScaleX(.7f);
    card.setScaleY(.7f);
    card.setAlpha(0);
    card.animate()
        .scaleX(1)
        .scaleY(1)
        .alpha(1)
        .setDuration(500)
        .setInterpolator(new OvershootInterpolator(1.2f))
        .start();
  }

  void playCoinArrival() {
    int stream = coinSounds.play(coinArrivalSound, .7f, .7f, 1, 0, 1f);
    if (stream == 0) android.util.Log.e("CoinSound", "金币音效播放失败");
  }

  void rainCredited(JSONObject updated, int coins, boolean animate) {
    user = updated;
    if (current.equals("hall") && balance != null)
      balance.setText("● " + number(user.optInt("coins")));
    else if (page != null) render();
    if (animate && coins > 0) playCoinArrival();
  }

  void flyCoins() {
    float startX = stage.getWidth() / 2f,
        startY = stage.getHeight() * .55f,
        endX = stage.getWidth() - dp(65),
        endY = dp(40);
    for (int i = 0; i < 14; i++) {
      int index = i;
      TextView coin = text("¥", 19, 0xffd78b0a, true);
      coin.setGravity(Gravity.CENTER);
      GradientDrawable gold = grad(0xffffe98f, 0xffffb932, 30);
      gold.setStroke(dp(2), 0xffffeabc);
      coin.setBackground(gold);
      FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(dp(31), dp(31));
      stage.addView(coin, params);
      float sx = startX + ((i % 5) - 2) * dp(22), sy = startY + (i / 5) * dp(14);
      coin.setX(sx);
      coin.setY(sy);
      Path path = new Path();
      path.moveTo(sx, sy);
      path.quadTo(sx - dp(110) + i * dp(10), sy - dp(160), endX, endY);
      ObjectAnimator move = ObjectAnimator.ofFloat(coin, View.X, View.Y, path);
      move.setDuration(650 + i * 20);
      move.setStartDelay(i * 35L);
      move.setInterpolator(new AccelerateInterpolator());
      move.addListener(
          new AnimatorListenerAdapter() {
            public void onAnimationEnd(Animator a) {
              if (index == 13) {
                rewardUiBusy = false;
                updates.maybePresent();
              }
              stage.removeView(coin);
              if (balance != null) {
                balance
                    .animate()
                    .scaleX(1.13f)
                    .scaleY(1.13f)
                    .setDuration(90)
                    .withEndAction(
                        () -> balance.animate().scaleX(1).scaleY(1).setDuration(150).start())
                    .start();
              }
            }
          });
      move.start();
    }
  }

  void openCheckin() {
    api.get("/check-in/status", value -> showCheckinDialog((JSONObject) value), this::error);
  }

  void showCheckinDialog(JSONObject status) {
    LinearLayout card = col();
    card.setPadding(dp(22), dp(20), dp(22), dp(18));
    card.setBackground(bg(CREAM, 22));
    add(card, text("连续签到", 22, INK, true), -1, -2);
    gap(card, 5);
    add(card, label("连续 7 天，每天打开都能领取一份金币", 12, MUTED), -1, -2);
    gap(card, 14);
    JSONArray days = status.optJSONArray("days");
    if (days != null) {
      for (int i = 0; i < days.length(); i++) {
        JSONObject item = days.optJSONObject(i);
        if (item == null) continue;
        int dayNumber = item.optInt("day");
        String state = item.optString("status");
        LinearLayout row = row();
        row.setPadding(dp(12), dp(8), dp(8), dp(8));
        row.setBackground(bg(WHITE, 12));
        add(row, text("第" + dayNumber + "天", 14, INK, true), 60, 34);
        flex(row, label(item.optInt("reward") + " 金币", 12, MUTED));
        String action =
            state.equals("checked") || state.equals("makeup")
                ? "已完成"
                : state.equals("today") ? "签到" : state.equals("missed") ? "补签" : "待解锁";
        TextView actionView =
            text(action, 11, state.equals("today") || state.equals("missed") ? RED : MUTED, true);
        actionView.setGravity(Gravity.CENTER);
        actionView.setBackground(
            bg(state.equals("today") || state.equals("missed") ? 0xffffead8 : 0xfff3eee8, 12));
        row.addView(actionView, new LinearLayout.LayoutParams(dp(64), dp(32)));
        if (state.equals("today"))
          actionView.setOnClickListener(
              v -> {
                dialogCloseAndCheck(v);
              });
        else if (state.equals("missed")) actionView.setOnClickListener(v -> makeup(dayNumber));
        add(card, row, -1, 50);
        gap(card, 7);
      }
    }
    add(card, label("补签会按当天档位发放金币，签到到账后会播放金币动画。", 10, 0xffb39583), -1, -2);
    Dialog dialog = sheet(card, 3);
    dialog.show();
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(40), dp(390)), -2);
    checkinDialog = dialog;
  }

  Dialog checkinDialog;

  void dialogCloseAndCheck(View ignored) {
    if (checkinDialog != null) checkinDialog.dismiss();
    checkIn();
  }

  void makeup(int dayNumber) {
    if (checkinDialog != null) checkinDialog.dismiss();
    api.post(
        "/check-in/makeup",
        Api.object("day", dayNumber),
        value -> {
          JSONObject result = (JSONObject) value;
          user = result.optJSONObject("user");
          refreshCheckinStatus();
          render();
          celebrate(result.optInt("reward"));
        },
        this::error);
  }

  void checkIn() {
    api.behavior.click(2, "");
    if (updates.required()) {
      updates.maybePresent();
      return;
    }
    if (signBusy || user.optBoolean("checked_in_today")) return;
    signBusy = true;
    if (checkInButton != null) checkInButton.setBusy(true);
    api.post(
        "/check-in",
        Api.object(),
        d -> {
          signBusy = false;
          JSONObject result = (JSONObject) d;
          user = result.optJSONObject("user");
          refreshCheckinStatus();
          render();
          celebrate(result.optInt("reward"));
        },
        new Api.Failure() {
          @Override
          public void failed(String message) {
            signBusy = false;
            if (checkInButton != null) checkInButton.setBusy(false);
            error(message);
          }

          @Override
          public void failed(int status, String message) {
            failed(message);
            if (status == 409) refreshUser();
          }
        });
  }

  void rewardError(String message) {
    new AlertDialog.Builder(this)
        .setTitle("操作未完成")
        .setMessage(message)
        .setPositiveButton("知道了", null)
        .setOnDismissListener(ignored -> rewardUiBusy = false)
        .show();
  }

  void shop() {
    LinearLayout content = scrollPage();
    FrameLayout promo = new FrameLayout(this);
    promo.setBackground(grad(0xffffe5d3, 0xfffff0df, 23));
    LinearLayout c = col();
    c.setPadding(dp(17), dp(12), dp(100), dp(12));
    add(c, text("认真生活，聪明买", 18, INK, true), -1, -2);
    gap(c, 6);
    add(c, label("心动好物，把金币也带回家", 11, 0xffaf7a55), -1, -2);
    gap(c, 8);
    TextView small = text("每日精选  ·  好价发现", 10, RED, true);
    add(c, small, -1, -2);
    promo.addView(c, new FrameLayout.LayoutParams(-1, -1));
    if (!shopPromoImage.isEmpty()) {
      ProductImage promoImage =
          new ProductImage(this, shopPromoImage, "好物精选", ImageView.ScaleType.CENTER_CROP);
      FrameLayout.LayoutParams bp =
          new FrameLayout.LayoutParams(dp(100), dp(100), Gravity.RIGHT | Gravity.BOTTOM);
      promo.addView(promoImage, bp);
    }
    add(content, promo, -1, 100);
    gap(content, 14);
    if (!productLoadError.isEmpty()) {
      TextView failure = label("好物更新失败\n" + productLoadError + "\n点击重试", 12, RED);
      failure.setPadding(dp(12), dp(10), dp(12), dp(10));
      failure.setBackground(bg(WHITE, 12));
      failure.setOnClickListener(
          v -> {
            refreshProducts();
            render();
          });
      add(content, failure, -1, -2);
      gap(content, 12);
    }
    HorizontalScrollView filters = new HorizontalScrollView(this);
    filters.setHorizontalScrollBarEnabled(false);
    LinearLayout f = row();
    LinkedHashSet<String> categories = new LinkedHashSet<>();
    categories.add("全部");
    for (int i = 0; i < products.length(); i++) {
      JSONObject product = products.optJSONObject(i);
      if (product != null && !product.optString("category").isEmpty())
        categories.add(product.optString("category"));
    }
    if (!categories.contains(category)) category = "全部";
    for (String cat : categories) {
      TextView t = text(cat, 13, category.equals(cat) ? WHITE : MUTED, category.equals(cat));
      t.setGravity(Gravity.CENTER);
      t.setPadding(dp(17), dp(9), dp(17), dp(9));
      t.setBackground(bg(category.equals(cat) ? RED : WHITE, 18));
      LinearLayout.LayoutParams tp = lp(-2, 36);
      tp.rightMargin = dp(8);
      f.addView(t, tp);
      t.setOnClickListener(
          v -> {
            category = cat;
            render();
          });
    }
    filters.addView(f);
    add(content, filters, -1, 38);
    gap(content, 14);
    ArrayList<JSONObject> list = new ArrayList<>();
    for (int i = 0; i < products.length(); i++) {
      JSONObject p = products.optJSONObject(i);
      if (p != null && (category.equals("全部") || category.equals(p.optString("category"))))
        list.add(p);
    }
    for (int i = 0; i < list.size(); i += 2) {
      LinearLayout r = row();
      r.setGravity(Gravity.TOP);
      for (int j = i; j < i + 2; j++) {
        if (j >= list.size()) {
          r.addView(new View(this), new LinearLayout.LayoutParams(0, 1, 1));
          break;
        }
        JSONObject p = list.get(j);
        LinearLayout card = productCard(p);
        LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(0, -2, 1);
        if (j == i) cp.rightMargin = dp(10);
        r.addView(card, cp);
      }
      add(content, r, -1, -2);
      gap(content, 12);
    }
    if (list.isEmpty()) {
      TextView empty = label(productsLoading ? "正在寻找好物" : "暂无在售好物", 14, MUTED);
      empty.setGravity(Gravity.CENTER);
      add(content, empty, -1, 110);
      if (!productsLoading)
        add(
            content,
            button(
                "刷新看看",
                () -> {
                  refreshProducts();
                  render();
                }),
            -1,
            48);
    }
  }

  LinearLayout productCard(JSONObject p) {
    LinearLayout card = col();
    card.setBackground(bg(WHITE, 19));
    card.setClipToOutline(true);
    ProductImage photo =
        new ProductImage(
            this, p.optString("image"), p.optString("title"), ImageView.ScaleType.CENTER_CROP);
    add(card, photo, -1, 132);
    LinearLayout copy = col();
    copy.setPadding(dp(12), dp(12), dp(12), dp(14));
    TextView platform = text(p.optString("platform"), 10, RED, false);
    add(copy, platform, -1, -2);
    gap(copy, 7);
    TextView title = text(p.optString("title"), 13, INK, true);
    title.setMaxLines(2);
    title.setMinLines(2);
    title.setLineSpacing(dp(3), 1);
    add(copy, title, -1, -2);
    gap(copy, 11);
    LinearLayout price = row();
    flex(price, text("¥" + money(p.optDouble("price")), 20, RED, true));
    add(copy, price, -1, -2);
    gap(copy, 7);
    TextView coinReward = text(productReward(p), 11, 0xffb5753d, true);
    coinReward.setPadding(dp(6), dp(4), dp(6), dp(4));
    coinReward.setBackground(bg(0xfffff1dc, 6));
    add(copy, coinReward, -2, -2);
    add(card, copy, -1, -2);
    card.setOnClickListener(
        v -> {
          api.behavior.click(6, p.optString("id"));
          currentProduct(p, card, data -> productDetail((JSONObject) data));
        });
    return card;
  }

  String productReward(JSONObject product) {
    return "预计返 " + new DecimalFormat("#,###").format(product.optLong("reward_coins")) + " 金币";
  }

  void currentProduct(JSONObject product, View trigger, Api.Result action) {
    if (!trigger.isEnabled()) return;
    trigger.setEnabled(false);
    trigger.setAlpha(.65f);
    String session = api.token;
    api.get(
        "/products/" + Uri.encode(product.optString("id")),
        result -> {
          trigger.setEnabled(true);
          trigger.setAlpha(1);
          if (!session.equals(api.token) || user == null || isDestroyed() || !foreground) return;
          action.done(result);
        },
        message -> {
          trigger.setEnabled(true);
          trigger.setAlpha(1);
          if (!session.equals(api.token) || user == null || isDestroyed() || !foreground) return;
          android.util.Log.e(
              "Products", "GET /api/products/" + product.optString("id") + ": " + message);
          error(message);
          refreshProducts();
        });
  }

  void productDetail(JSONObject p) {
    api.productClick(p.optString("id"), "detail");
    api.behavior.business(7, p.optString("id"));
    Dialog dialog = new Dialog(this);
    LinearLayout card = col();
    card.setBackground(bg(CREAM, 25));
    card.setClipToOutline(true);
    LinearLayout heading = row();
    heading.setPadding(dp(20), 0, dp(18), 0);
    flex(heading, text("好物详情", 17, INK, true));
    LineIcon close = new LineIcon(this, "close", MUTED);
    close.setContentDescription("关闭商品详情");
    close.setOnClickListener(
        v -> {
          api.behavior.click(12, "");
          dialog.dismiss();
        });
    add(heading, close, 24, 24);
    add(card, heading, -1, 54);
    ScrollView scroll = new ScrollView(this);
    scroll.setVerticalScrollBarEnabled(false);
    LinearLayout content = col();
    scroll.addView(content);
    add(
        content,
        new ProductImage(
            this, p.optString("image"), p.optString("title"), ImageView.ScaleType.FIT_CENTER),
        -1,
        230);
    LinearLayout details = col();
    details.setPadding(dp(22), dp(18), dp(22), dp(18));
    add(details, label(p.optString("platform"), 11, RED), -1, -2);
    gap(details, 10);
    add(details, text(p.optString("title"), 19, INK, true), -1, -2);
    gap(details, 16);
    add(details, text("¥" + money(p.optDouble("price")), 29, RED, true), -1, -2);
    gap(details, 12);
    TextView coins = text(productReward(p), 14, 0xffb5753d, true);
    coins.setPadding(dp(12), dp(9), dp(12), dp(9));
    coins.setBackground(bg(0xffffecd2, 12));
    add(details, coins, -2, -2);
    add(content, details, -1, -2);
    card.addView(scroll, new LinearLayout.LayoutParams(-1, 0, 1));
    LinearLayout footer = col();
    footer.setPadding(dp(20), dp(12), dp(20), dp(16));
    TextView buy = button("去" + p.optString("platform", "购买平台") + "看看", () -> {});
    buy.setOnClickListener(
        v -> {
          api.behavior.click(7, p.optString("id"));
          currentProduct(
              p,
              buy,
              result -> {
                if (!dialog.isShowing()) return;
                JSONObject latest = (JSONObject) result;
                Uri target = Uri.parse(latest.optString("url"));
                if (!("https".equals(target.getScheme()) || "http".equals(target.getScheme()))
                    || target.getHost() == null) {
                  error("商品购买地址无效");
                  return;
                }
                try {
                  api.productClick(latest.optString("id"), "outbound");
                  startActivity(
                      new Intent(Intent.ACTION_VIEW, target)
                          .addCategory(Intent.CATEGORY_BROWSABLE));
                  api.behavior.business(8, latest.optString("id"));
                  dialog.dismiss();
                } catch (ActivityNotFoundException failure) {
                  error(failure.getMessage());
                }
              });
        });
    add(footer, buy, -1, 52);
    add(card, footer, -1, -2);
    dialog.setContentView(card);
    api.behavior.dialog(dialog, 6);
    Window window = dialog.getWindow();
    window.setBackgroundDrawableResource(android.R.color.transparent);
    window.setDimAmount(.5f);
    window.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
    dialog.show();
    window.setLayout(
        Math.min(getResources().getDisplayMetrics().widthPixels - dp(32), dp(400)),
        Math.min(getResources().getDisplayMetrics().heightPixels - dp(96), dp(630)));
  }

  void mine() {
    LinearLayout content = scrollPage();
    LinearLayout profile = row();
    ImageView avatar = art("mascot");
    avatar.setBackground(bg(0xffffe7d0, 20));
    add(profile, avatar, 46, 46);
    TextView nickname = text(user.optString("nickname", "小橙朋友"), 17, INK, true);
    nickname.setPadding(dp(12), 0, dp(8), 0);
    nickname.setSingleLine();
    nickname.setEllipsize(android.text.TextUtils.TruncateAt.END);
    flex(profile, nickname);
    String phone = user.optString("phone");
    add(
        profile,
        label(
            phone.length() == 11 ? phone.substring(0, 3) + " **** " + phone.substring(7) : phone,
            12,
            MUTED),
        -2,
        -2);
    add(content, profile, -1, 54);
    gap(content, 10);
    LinearLayout wallet = col();
    wallet.setPadding(dp(17), dp(16), dp(17), dp(16));
    wallet.setBackground(grad(0xffff7947, 0xffef432c, 26));
    LinearLayout top = row();
    flex(top, label("我的金币", 13, 0xffffdfc9));
    TextView record = label("收益明细  ›", 12, 0xffffdfc9);
    record.setOnClickListener(v -> ledger());
    add(top, record, -2, -2);
    add(wallet, top, -1, -2);
    gap(wallet, 6);
    add(wallet, text(number(user.optInt("coins")), 44, WHITE, true), -1, -2);
    gap(wallet, 4);
    add(wallet, label("约 ¥" + money(user.optDouble("cash_yuan")), 13, 0xffffe3c8), -1, -2);
    gap(wallet, 12);
    LinearLayout totals = row();
    flex(totals, label("今日 " + number(user.optInt("today_earned")) + " 金币", 12, 0xffffe3c8));
    flex(totals, label("累计 " + number(user.optInt("total_earned")) + " 金币", 12, 0xffffe3c8));
    add(wallet, totals, -1, -2);
    gap(wallet, 12);
    TextView withdraw = text("去提现", 16, RED, true);
    withdraw.setGravity(Gravity.CENTER);
    withdraw.setBackground(bg(0xfffff2dd, 16));
    withdraw.setOnClickListener(v -> withdraw());
    add(wallet, withdraw, -1, 40);
    add(content, wallet, -1, -2);
    gap(content, 12);
    LinearLayout invitation = row();
    invitation.setPadding(dp(17), dp(10), dp(15), dp(10));
    invitation.setBackground(grad(0xffffe2cb, 0xffffecda, 20));
    LinearLayout inviteCopy = col();
    add(inviteCopy, text("邀请好友", 17, INK, true), -1, -2);
    gap(inviteCopy, 4);
    add(inviteCopy, label("分享小欢喜，一起领福利", 12, 0xffb07c58), -1, -2);
    flex(invitation, inviteCopy);
    add(invitation, art("packet"), 48, 48);
    add(invitation, new LineIcon(this, "arrow", 0xffc48d68), 16, 16);
    invitation.setOnClickListener(v -> inviteFriends());
    add(content, invitation, -1, 68);
    gap(content, 12);
    LinearLayout actions = col();
    actions.setBackground(bg(WHITE, 22));
    menu(actions, "clock", "提现记录", this::withdrawals);
    line(actions);
    menu(actions, "coin", "金币明细", this::ledger);
    line(actions);
    boolean checked = user.optBoolean("checked_in_today");
    LinearLayout daily =
        menu(actions, checked ? "check" : "gift", checked ? "今日已签到" : "连续签到", this::openCheckin);
    add(content, actions, -1, -2);
    gap(content, 12);
    LinearLayout help = col();
    help.setBackground(bg(WHITE, 22));
    menu(
        help,
        "shield",
        "帮助与反馈",
        () ->
            new AlertDialog.Builder(this)
                .setTitle("很高兴遇见你")
                .setMessage("查看收益与提现记录，可以了解每一份奖励的去向。\n\n提现资格、审核与处理状态以页面实际显示为准。")
                .setPositiveButton("知道了", null)
                .show());
    add(content, help, -1, -2);
    gap(content, 23);
    TextView slogan = label("小小的收获，大大的开心", 11, 0xffbaa79b);
    slogan.setGravity(Gravity.CENTER);
    add(content, slogan, -1, 28);
  }

  LinearLayout menu(LinearLayout card, String icon, String title, Runnable action) {
    LinearLayout r = row();
    r.setPadding(dp(18), 0, dp(17), 0);
    LineIcon i = new LineIcon(this, icon, 0xffd28359);
    add(r, i, 21, 21);
    TextView t = label("   " + title, 14, INK);
    flex(r, t);
    add(r, new LineIcon(this, "arrow", 0xffcbb9ae), 15, 15);
    add(card, r, -1, 52);
    r.setOnClickListener(v -> action.run());
    return r;
  }

  void settings() {
    api.behavior.page(12);
    settingsReturn = current;
    current = "settings";
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    LinearLayout body = col();
    body.setPadding(dp(22), dp(18), dp(22), dp(28));
    scroll.addView(body);
    stage.removeAllViews();
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));

    LinearLayout top = row();
    LineIcon back = new LineIcon(this, "back", MUTED);
    back.setContentDescription("返回");
    back.setOnClickListener(v -> closeSettings());
    add(top, back, 28, 28);
    flex(top, text("账号设置", 22, INK, true));
    add(body, top, -1, 44);
    gap(body, 18);

    LinearLayout account = col();
    account.setBackground(bg(WHITE, 22));
    LinearLayout version = row();
    version.setPadding(dp(18), 0, dp(17), 0);
    flex(version, label("当前版本", 14, INK));
    add(version, label(BuildConfig.VERSION_NAME, 13, MUTED), -2, -2);
    add(account, version, -1, 52);
    line(account);
    menu(account, "clock", "检查更新", updates::manual);
    line(account);
    menu(account, "shield", "修改密码", () -> auth("reset"));
    line(account);
    menu(account, "shield", "隐私政策", () -> openPolicy("/privacy"));
    line(account);
    menu(account, "notice", "用户协议", () -> openPolicy("/terms"));
    add(body, account, -1, -2);
    gap(body, 14);

    LinearLayout security = col();
    security.setBackground(bg(WHITE, 22));
    menu(security, "clock", "注销账号", this::confirmDeleteAccount);
    line(security);
    menu(security, "back", "退出登录", this::exitAccount);
    add(body, security, -1, -2);
    gap(body, 24);
    TextView hint = label("账号注销需经过审核，审核通过后才会完成处理。", 11, MUTED);
    hint.setGravity(Gravity.CENTER);
    add(body, hint, -1, 28);
  }

  void closeSettings() {
    current = settingsReturn.isEmpty() ? "me" : settingsReturn;
    home();
  }

  void confirmDeleteAccount() {
    deletionReturn = current.equals("settings") ? settingsReturn : current;
    if (deletionReturn.isEmpty() || deletionReturn.equals("settings")) deletionReturn = "me";
    current = "deletion_confirm";
    deletionConfirmPage = true;
    deletionPage = false;
    endHallSession();
    page = null;
    stage.removeAllViews();
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    LinearLayout body = col();
    body.setPadding(dp(26), dp(18), dp(26), dp(28));
    scroll.addView(body);
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));

    LinearLayout top = row();
    LineIcon back = new LineIcon(this, "back", MUTED);
    back.setContentDescription("返回账号设置");
    back.setOnClickListener(v -> closeDeletionConfirm());
    add(top, back, 28, 28);
    flex(top, text("注销账号", 22, INK, true));
    add(body, top, -1, 44);
    gap(body, 22);

    LinearLayout card = col();
    card.setGravity(Gravity.CENTER_HORIZONTAL);
    card.setPadding(dp(24), dp(30), dp(24), dp(24));
    card.setBackground(grad(0xffffead8, 0xfffff4e7, 26));
    add(card, new LineIcon(this, "notice", RED), 50, 50);
    gap(card, 16);
    add(card, text("确认注销账号", 24, INK, true), -1, -2);
    gap(card, 12);
    TextView detail = label("提交后会进入审核流程，审核通过后账号资料不可恢复。", 14, MUTED);
    detail.setGravity(Gravity.CENTER);
    detail.setLineSpacing(dp(5), 1);
    add(card, detail, -1, -2);
    gap(card, 20);
    add(card, label("审核期间可以返回账号查看当前状态。", 12, 0xffb17a59), -1, -2);
    gap(card, 26);
    add(card, button("提交注销申请", this::deleteAccount), -1, 52);
    add(body, card, -1, -2);
    gap(body, 16);
    TextView backAccount = label("返回账号设置", 13, MUTED);
    backAccount.setGravity(Gravity.CENTER);
    backAccount.setOnClickListener(v -> closeDeletionConfirm());
    add(body, backAccount, -1, 36);
  }

  void closeDeletionConfirm() {
    deletionConfirmPage = false;
    deletionPage = false;
    current = deletionReturn.isEmpty() ? "me" : deletionReturn;
    settings();
  }

  void deleteAccount() {
    api.post(
        "/account/delete",
        Api.object("reason", ""),
        value -> showDeletionPage((JSONObject) value),
        this::error);
  }

  void inviteFriends() {
    api.behavior.click(9, "");
    api.get(
        "/invite/summary",
        result -> {
          JSONObject summary = (JSONObject) result;
          String code = summary.optString("invite_code");
          api.get(
              "/invite/qr/" + code,
              qr -> {
                try {
                  Bitmap poster = InvitePoster.render(this, summary, (JSONObject) qr);
                  cacheInvitePoster(poster, code);
                  invitationPreview(summary, poster);
                } catch (JSONException | IllegalArgumentException | IOException failure) {
                  error(failure.getMessage());
                }
              },
              this::error);
        },
        this::error);
  }

  void invitationPreview(JSONObject summary, Bitmap poster) {
    String code = summary.optString("invite_code"), url = summary.optString("invite_url");
    Dialog dialog = new Dialog(this);
    ScrollView scroll = new ScrollView(this);
    scroll.setBackground(bg(CREAM, 25));
    scroll.setClipToOutline(true);
    LinearLayout content = col();
    content.setPadding(dp(18), dp(16), dp(18), dp(16));
    scroll.addView(content);
    LinearLayout title = row();
    flex(title, text("邀请好友", 23, INK, true));
    LineIcon close = new LineIcon(this, "close", MUTED);
    close.setOnClickListener(v -> dialog.dismiss());
    add(title, close, 22, 22);
    add(content, title, -1, 30);
    gap(content, 12);
    LinearLayout totals = row();
    String[] labels = {"已邀请", "已注册", "待注册"};
    String[] fields = {"invited_count", "registered_count", "pending_count"};
    for (int i = 0; i < labels.length; i++) {
      LinearLayout item = col();
      item.setGravity(Gravity.CENTER);
      add(item, text(number(summary.optInt(fields[i])), 21, RED, true), -2, -2);
      gap(item, 7);
      add(item, label(labels[i], 11, MUTED), -2, -2);
      totals.addView(item, new LinearLayout.LayoutParams(0, -2, 1));
    }
    add(content, totals, -1, -2);
    gap(content, 16);
    ImageView preview = new ImageView(this);
    preview.setImageBitmap(poster);
    preview.setScaleType(ImageView.ScaleType.FIT_CENTER);
    add(content, preview, -1, 240);
    gap(content, 16);
    LinearLayout primary = row();
    TextView save = button("保存图片", () -> saveInvitePoster(poster, code));
    TextView share = button("分享海报", () -> shareInvitePoster(poster, code, url));
    LinearLayout.LayoutParams saveParams = new LinearLayout.LayoutParams(0, dp(49), 1);
    saveParams.rightMargin = dp(10);
    primary.addView(save, saveParams);
    primary.addView(share, new LinearLayout.LayoutParams(0, dp(49), 1));
    add(content, primary, -1, -2);
    gap(content, 10);
    LinearLayout copies = row();
    TextView link = text("复制链接", 13, RED, false);
    link.setGravity(Gravity.CENTER);
    link.setOnClickListener(v -> copyInvite("邀请链接", url));
    TextView inviteCode = text("复制邀请码", 13, RED, false);
    inviteCode.setGravity(Gravity.CENTER);
    inviteCode.setOnClickListener(v -> copyInvite("邀请码", code));
    copies.addView(link, new LinearLayout.LayoutParams(0, dp(38), 1));
    copies.addView(inviteCode, new LinearLayout.LayoutParams(0, dp(38), 1));
    add(content, copies, -1, -2);
    dialog.setContentView(scroll);
    api.behavior.dialog(dialog, 8);
    Window window = dialog.getWindow();
    window.setBackgroundDrawableResource(android.R.color.transparent);
    window.setDimAmount(.5f);
    window.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
    dialog.show();
    window.setLayout(
        Math.min(getResources().getDisplayMetrics().widthPixels - dp(32), dp(400)), -2);
  }

  void copyInvite(String label, String value) {
    ClipboardManager clipboard = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
    clipboard.setPrimaryClip(ClipData.newPlainText(label, value));
    tip("已复制" + label);
  }

  File cacheInvitePoster(Bitmap poster, String code) throws IOException {
    File directory = new File(getCacheDir(), "invite-posters");
    if (!directory.isDirectory() && !directory.mkdirs()) throw new IOException("海报目录创建失败");
    File file = new File(directory, "invite-" + code + ".png");
    try (OutputStream output = new FileOutputStream(file)) {
      if (!poster.compress(Bitmap.CompressFormat.PNG, 100, output))
        throw new IOException("海报图片保存失败");
    }
    return file;
  }

  void shareInvitePoster(Bitmap poster, String code, String url) {
    try {
      File file = cacheInvitePoster(poster, code);
      Uri uri = FileProvider.getUriForFile(this, getPackageName() + ".invite-files", file);
      Intent share = new Intent(Intent.ACTION_SEND);
      share.setType("image/png");
      share.putExtra(Intent.EXTRA_STREAM, uri);
      share.putExtra(Intent.EXTRA_TEXT, url);
      share.setClipData(ClipData.newRawUri("邀请海报", uri));
      share.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
      startActivity(Intent.createChooser(share, "邀请好友"));
    } catch (IOException | ActivityNotFoundException failure) {
      error(failure.getMessage());
    }
  }

  void saveInvitePoster(Bitmap poster, String code) {
    String filename = "小橙有喜-邀请-" + code + "-" + System.currentTimeMillis() + ".png";
    try {
      if (Build.VERSION.SDK_INT >= 29) {
        ContentValues values = new ContentValues();
        values.put(MediaStore.Images.Media.DISPLAY_NAME, filename);
        values.put(MediaStore.Images.Media.MIME_TYPE, "image/png");
        values.put(MediaStore.Images.Media.RELATIVE_PATH, Environment.DIRECTORY_PICTURES + "/小橙有喜");
        values.put(MediaStore.Images.Media.IS_PENDING, 1);
        Uri uri = getContentResolver().insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values);
        if (uri == null) throw new IOException("相册图片创建失败");
        try (OutputStream output = getContentResolver().openOutputStream(uri)) {
          if (!poster.compress(Bitmap.CompressFormat.PNG, 100, output))
            throw new IOException("海报图片保存失败");
        }
        values.clear();
        values.put(MediaStore.Images.Media.IS_PENDING, 0);
        getContentResolver().update(uri, values, null, null);
        tip("已保存到相册");
      } else {
        File directory = getExternalFilesDir(Environment.DIRECTORY_PICTURES);
        if (directory == null) throw new IOException("图片目录不可用");
        try (OutputStream output = new FileOutputStream(new File(directory, filename))) {
          if (!poster.compress(Bitmap.CompressFormat.PNG, 100, output))
            throw new IOException("海报图片保存失败");
        }
        tip("海报已保存，可直接分享");
      }
    } catch (IOException failure) {
      error(failure.getMessage());
    }
  }

  void ledger() {
    api.get(
        "/ledger",
        d -> {
          JSONArray items = (JSONArray) d;
          showList(
              "金币明细",
              items,
              (j) ->
                  j.optString("title")
                      + "\n"
                      + (j.optInt("coins") >= 0 ? "+" : "")
                      + j.optInt("coins")
                      + " 金币    "
                      + date(j.optString("created_at")));
        },
        this::error);
  }

  void withdrawals() {
    api.get(
        "/withdrawals",
        d -> {
          JSONArray items = (JSONArray) d;
          showList(
              "提现记录",
              items,
              (j) ->
                  "¥"
                      + money(j.optDouble("amount_yuan"))
                      + " · "
                      + status(j.optString("status"))
                      + "\n"
                      + date(j.optString("created_at")));
        },
        this::error);
  }

  interface RowText {
    String text(JSONObject j);
  }

  String status(String value) {
    switch (value) {
      case "pending":
        return "待处理";
      case "approved":
        return "已完成";
      case "rejected":
        return "已退回";
      default:
        return value;
    }
  }

  String date(String value) {
    SimpleDateFormat input = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.CHINA);
    input.setTimeZone(TimeZone.getTimeZone("UTC"));
    SimpleDateFormat output = new SimpleDateFormat("MM-dd HH:mm", Locale.CHINA);
    output.setTimeZone(TimeZone.getTimeZone("Asia/Shanghai"));
    try {
      return output.format(input.parse(value));
    } catch (ParseException e) {
      throw new IllegalArgumentException("时间格式错误：" + value, e);
    }
  }

  void showList(String title, JSONArray entries, RowText format) {
    LinearLayout outer = col();
    outer.setPadding(dp(22), dp(22), dp(22), dp(22));
    outer.setBackground(bg(CREAM, 24));
    add(outer, text(title, 22, INK, true), -1, -2);
    gap(outer, 16);
    ScrollView scroll = new ScrollView(this);
    LinearLayout rows = col();
    scroll.addView(rows);
    if (entries.length() == 0) {
      TextView empty = label("还没有记录，去发现今天的福利吧", 14, MUTED);
      empty.setPadding(0, dp(30), 0, dp(30));
      add(rows, empty, -1, -2);
    }
    for (int i = 0; i < entries.length(); i++) {
      JSONObject j = entries.optJSONObject(i);
      TextView item = label(format.text(j), 13, INK);
      item.setLineSpacing(dp(8), 1);
      item.setPadding(0, dp(14), 0, dp(14));
      add(rows, item, -1, -2);
      line(rows);
    }
    add(outer, scroll, -1, Math.min(340, 70 + entries.length() * 70));
    gap(outer, 16);
    Dialog dialog = sheet(outer);
    add(outer, button("完成", dialog::dismiss), -1, 49);
    dialog.show();
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(40), dp(390)), -2);
  }

  void withdraw() {
    api.behavior.click(8, "");
    if (updates.required()) {
      updates.maybePresent();
      return;
    }
    LinearLayout card = col();
    card.setPadding(dp(23), dp(23), dp(23), dp(23));
    card.setBackground(bg(CREAM, 26));
    add(card, text("把收获装进口袋", 23, INK, true), -1, -2);
    gap(card, 12);
    add(card, label("可提现 ¥" + money(user.optDouble("cash_yuan")), 14, MUTED), -1, -2);
    gap(card, 22);
    EditText amount =
        input(
            "提现金额，满 ¥" + money(user.optDouble("withdraw_min_yuan")) + " 可提",
            InputType.TYPE_CLASS_NUMBER | InputType.TYPE_NUMBER_FLAG_DECIMAL);
    add(card, amount, -1, 54);
    gap(card, 12);
    EditText account = input("收款账号", InputType.TYPE_CLASS_TEXT);
    add(card, account, -1, 54);
    gap(card, 15);
    TextView channel = label("收款方式：支付宝", 13, INK);
    add(card, channel, -1, 28);
    gap(card, 8);
    add(card, label("提现资格与处理状态以页面实际显示为准", 11, MUTED), -1, -2);
    gap(card, 20);
    Dialog dialog = sheet(card, 7);
    TextView submit = button("确认提现", () -> {});
    submit.setOnClickListener(
        v -> {
          double value;
          try {
            value = Double.parseDouble(amount.getText().toString());
          } catch (NumberFormatException e) {
            error("请输入提现金额");
            return;
          }
          submit.setEnabled(false);
          api.post(
              "/withdrawals",
              Api.object(
                  "amount_yuan",
                  value,
                  "channel",
                  "alipay",
                  "account",
                  account.getText().toString()),
              d -> {
                dialog.dismiss();
                JSONObject result = (JSONObject) d;
                if (result.has("user")) user = result.optJSONObject("user");
                refreshUser();
                tip("提现申请已提交");
              },
              m -> {
                submit.setEnabled(true);
                error(m);
              });
        });
    add(card, submit, -1, 52);
    gap(card, 12);
    TextView close = label("再攒一会儿", 12, MUTED);
    close.setGravity(Gravity.CENTER);
    close.setOnClickListener(v -> dialog.dismiss());
    add(card, close, -1, 30);
    dialog.show();
    dialog.getWindow().setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE);
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(40), dp(390)), -2);
  }

  @Override
  public boolean dispatchTouchEvent(MotionEvent event) {
    if (api != null) api.behavior.touch(event);
    return super.dispatchTouchEvent(event);
  }

  @Override
  public void onBackPressed() {
    if (updates.required()) {
      finish();
      return;
    }
    if (deletionConfirmPage) {
      closeDeletionConfirm();
    } else if (deletionPage) {
      enterAccountHome();
    } else if (current.equals("settings")) {
      closeSettings();
    } else if (welcomeShowing) {
      finish();
    } else if (user != null && user.optBoolean("invitation_required")) {
      exitAccount();
    } else if (user != null) {
      if (!current.equals("hall")) {
        current = "hall";
        home();
      } else super.onBackPressed();
    } else auth("login");
  }

  @Override
  protected void onResume() {
    super.onResume();
    foreground = true;
    api.behavior.resume();
    nativeFeeds.resume();
    if (current.equals("hall") && hallSurface != null && rainTime != null)
      rain.attach(hallSurface, rainTime);
    if (pendingReward != null) {
      PendingReward pending = pendingReward;
      showRewardPending(pending.session, pending.notice, pending.rainPacket, pending.requestError);
    }
    refreshRuntime();
    updates.resume();
  }

  @Override
  protected void onPause() {
    api.behavior.pause();
    foreground = false;
    runtimeEpoch++;
    runtimeLoading = false;
    handler.removeCallbacks(runtimeTick);
    updates.pause();
    closeRewardStatus();
    rain.detach();
    nativeFeeds.pause();
    super.onPause();
  }

  @Override
  protected void onDestroy() {
    api.behavior.destroy();
    updates.destroy();
    endHallSession();
    csjAds.dispose();
    coinSounds.release();
    super.onDestroy();
  }

  @Override
  protected void onSaveInstanceState(Bundle state) {
    super.onSaveInstanceState(state);
    updates.save(state);
  }

  @Override
  protected void onActivityResult(int requestCode, int resultCode, Intent data) {
    super.onActivityResult(requestCode, resultCode, data);
    updates.activityResult(requestCode, resultCode);
  }

  @Override
  public void onWindowFocusChanged(boolean focused) {
    super.onWindowFocusChanged(focused);
    if (focused && updates != null) updates.maybePresent();
  }

  boolean canPresentUpdate() {
    return servicesReady
        && foreground
        && !startupAdActive
        && !rewardUiBusy
        && !signBusy
        && (rain == null || !rain.busy())
        && getWindow().getDecorView().hasWindowFocus();
  }

  final Runnable runtimeTick = this::refreshRuntime;

  void refreshRuntime() {
    if (!servicesReady || !foreground || runtimeLoading) return;
    updates.maybePresent();
    api.behavior.refreshPolicy();
    runtimeLoading = true;
    int epoch = runtimeEpoch;
    handler.removeCallbacks(runtimeTick);
    handler.postDelayed(runtimeTick, 45000);
    api.get(
        "/app/runtime",
        result -> {
          if (epoch != runtimeEpoch || isDestroyed()) return;
          runtimeLoading = false;
          try {
            JSONObject runtime = (JSONObject) result;
            long revision = runtime.getLong("revision");
            if (revision != runtimeRevision) {
              hallNoticeText = runtime.getString("hall_notice").trim();
              runtimeRevision = revision;
              if (hallNotice != null) {
                hallNotice.setText(hallNoticeText);
                hallNotice.setVisibility(hallNoticeText.isEmpty() ? View.GONE : View.VISIBLE);
              }
            }
          } catch (JSONException error) {
            android.util.Log.e("AppRuntime", "GET /api/app/runtime: " + error.getMessage());
          }
        },
        message -> {
          if (epoch != runtimeEpoch || isDestroyed()) return;
          runtimeLoading = false;
          android.util.Log.e("AppRuntime", "GET /api/app/runtime: " + message);
        });
  }
}
