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
import com.device.trust.secure.TouchAudit;
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
  int coinArrivalSound, packetOpenSound;
  final Handler handler = new Handler(Looper.getMainLooper());
  RewardedAdProvider rewarded;
  AdMediator ads;
  NativeFeedAds nativeFeeds;
  HallRainController rain;
  FrameLayout hallSurface;
  TextView rainTime;
  AlertDialog rewardStatusDialog;
  Dialog busyDialog;
  final Runnable busyTimeout = this::hideBusy;
  PendingReward pendingReward;
  AppUpdates updates;
  boolean servicesReady, rewardUiBusy;
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
  Runnable authBack;

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
    ads = new AdMediator(api);
    nativeFeeds = new NativeFeedAds(this, ads, this::removeHallItem);
    rain = new HallRainController(this, api);
    rewarded = BuildConfig.ADS_ENABLED ? ads : new RewardedAdProvider.Mock();
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
    packetOpenSound = coinSounds.load(this, R.raw.packet_open, 1);
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
    if (api.token.isEmpty()) auth("login");
    else loadAccount();
  }

  static final String PRIVACY_ACCEPTED = "privacy_accepted_v4";

  void confirmPrivacy() {
    // Bumped when the policy adds third parties (0.3.21: 快手/百度/优量汇 ad SDKs, 腾讯云短信),
    // so users who agreed to the older text are asked once more.
    if (api.preferences.getBoolean(PRIVACY_ACCEPTED, false)
        && api.preferences.getBoolean(DeviceTrustClient.CONSENT, false)) {
      afterPrivacy();
      return;
    }
    loading();
    AlertDialog consent =
        new AlertDialog.Builder(this)
            .setTitle("欢迎来到小橙有喜")
            .setMessage(
                "广西奇易电子商务有限公司为你提供服务。请阅读《隐私政策》和《用户协议》，点击“同意并继续”即表示你同意以下处理：\n\n"
                    + "1. 广告服务：App 集成穿山甲广告 SDK（含抖音 SDK）、快手联盟、百度移动广告 SDK（含百度 oCPX 转化追踪 SDK）、优量汇 Android SDK，会读取设备标识 OAID（匿名设备标识符），以及设备型号、系统版本、网络状态和广告展示点击记录，用于广告投放、效果统计和防作弊。已关闭个性化广告，不读取位置、电话状态、MAC 地址和应用列表。\n\n"
                    + "2. 账户安全：知衡 DeviceTrust SDK 会读取 ANDROID_ID、Widevine 设备标识的不可逆摘要（原值不上传）、指定风险工具是否安装、无障碍服务状态、本机调试端口，以及当前局域网的内网地址、网关和 DNS，并按核验需要检测同一网段内其他设备是否在线或开启无线调试，用于识别作弊和保护账户安全。\n\n"
                    + "3. 服务改进：登录后记录本应用内的页面访问、点击、停留与触点，用于账户安全和服务改进；不采集输入内容、密码短信、屏幕画面或其他应用的使用内容。\n\n"
                    + "以上 SDK 均在你同意后才会初始化。你可以在“我的 → 设置”中查看隐私政策和第三方 SDK 清单。")
            .setPositiveButton(
                "同意并继续",
                (dialog, which) -> {
                  api.preferences
                      .edit()
                      .putBoolean(PRIVACY_ACCEPTED, true)
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
            api.authConfig = ((JSONObject) result).optJSONObject("auth");
            api.adsConfig = ((JSONObject) result).optJSONObject("ads");
            activities.configure((JSONObject) result);
            api.configurationLoaded = true;
            api.deviceTrust.initialize(
                () -> {
                  servicesReady = true;
                  refreshRuntime();
                  updates.start(
                      () -> {
                        // No splash ad (store review): start the ad SDK quietly for rewarded
                        // videos and feed cards, and go straight into the app.
                        if (BuildConfig.ADS_ENABLED) startAdSdk();
                        launchApp();
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

  void startAdSdk() {
    // Starts every network the server enabled; which one serves a request is decided per request.
    ads.start(this, api.adsConfig);
  }

  void openPolicy(String path) {
    String url =
        path.equals("/privacy")
            ? BuildConfig.PRIVACY_URL
            : path.equals("/privacy#sdk")
                ? BuildConfig.PRIVACY_URL + "#sdk"
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

  // The login risk report runs in the background after sign-in; a review decision there is kept
  // server-side and enforced on rewards/withdrawals, so it must not interrupt the user.
  void loginReportFailed(String message) {
    android.util.Log.w("LoginReport", "login risk report not accepted: " + message);
  }

  void error(String message) {
    new AlertDialog.Builder(this)
        .setTitle("温馨提示")
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

  /** Small spinner over the page while a red packet or an ad video is being prepared. */
  void showBusy(String text) {
    hideBusy();
    LinearLayout box = col();
    box.setGravity(Gravity.CENTER);
    box.setPadding(dp(26), dp(22), dp(26), dp(20));
    box.setBackground(bg(0xFFFFFFFF, 18));
    ProgressBar spinner = new ProgressBar(this);
    add(box, spinner, 40, 40);
    gap(box, 12);
    add(box, label(text, 14, INK), -2, -2);
    Dialog d = new Dialog(this);
    d.requestWindowFeature(android.view.Window.FEATURE_NO_TITLE);
    d.setContentView(box);
    d.setCancelable(false);
    if (d.getWindow() != null) {
      d.getWindow().setBackgroundDrawable(new android.graphics.drawable.ColorDrawable(Color.TRANSPARENT));
      d.getWindow().setDimAmount(0.25f);
    }
    busyDialog = d;
    d.show();
    // Never leave the spinner up if something upstream forgets to call back.
    handler.removeCallbacks(busyTimeout);
    handler.postDelayed(busyTimeout, 30000);
  }

  void hideBusy() {
    handler.removeCallbacks(busyTimeout);
    if (busyDialog != null) {
      try {
        busyDialog.dismiss();
      } catch (RuntimeException ignored) {
      }
      busyDialog = null;
    }
  }

  void loadAccount() {
    loading();
    api.get(
        "/me",
        data -> {
          user = (JSONObject) data;
          enterAccount();
          api.reportLogin(this::loginReportFailed);
        },
        m -> {
          api.saveToken("");
          error(m);
          auth("login");
        });
  }

  void acceptAuth(Object data) {
    authBack = null;
    JSONObject j = (JSONObject) data;
    api.saveToken(j.optString("token"));
    user = j.optJSONObject("user");
    enterAccount();
    api.reportLogin(this::loginReportFailed);
  }

  void enterAccount() {
    checkDeletionStatus(this::enterAccountHome);
  }

  void enterAccountHome() {
    if (current.equals("deletion")) current = deletionReturn.isEmpty() ? "hall" : deletionReturn;
    deletionPage = false;
    activities.load();
    loadNewbie();
    loadWheel();
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
    if (state.equals("pending")) {
      add(card, button("撤回注销申请", this::confirmCancelDeletion), -1, 50);
      gap(card, 10);
      TextView refresh = text("刷新状态", 14, RED, true);
      refresh.setGravity(Gravity.CENTER);
      refresh.setOnClickListener(v -> refreshDeletionStatus());
      add(card, refresh, -1, 40);
    } else add(card, button("刷新状态", this::refreshDeletionStatus), -1, 50);
    add(body, card, -1, -2);
    gap(body, 16);
    TextView backAccount = label("返回账号", 13, MUTED);
    backAccount.setGravity(Gravity.CENTER);
    backAccount.setOnClickListener(v -> enterAccountHome());
    add(body, backAccount, -1, 36);
  }

  void confirmCancelDeletion() {
    new AlertDialog.Builder(this)
        .setTitle("撤回注销申请")
        .setMessage("撤回后账号恢复正常使用，金币和记录保持不变。如需注销可以以后重新提交。")
        .setPositiveButton(
            "确认撤回",
            (dialog, which) ->
                api.post(
                    "/account/deletion/cancel",
                    Api.object(),
                    value -> {
                      tip("已撤回注销申请");
                      deletionPage = false;
                      enterAccountHome();
                    },
                    this::error))
        .setNegativeButton("再想想", null)
        .show();
  }

  void finishDeletion() {
    deletionPage = false;
    deletionConfirmPage = false;
    activities.clear();
    endHallSession();
    api.saveToken("");
    user = null;
    newbie = null;
    wheelInfo = null;
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
          maybePromptCheckin();
        },
        message -> android.util.Log.e("CheckIn", "GET /api/check-in/status: " + message));
  }

  void invitationBinding() {
    authBack = null;
    endHallSession();
    page = null;
    stage.removeAllViews();
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    scroll.setVerticalScrollBarEnabled(false);
    LinearLayout body = col();
    body.setPadding(dp(24), dp(4), dp(24), dp(24));
    scroll.addView(body);
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));
    LinearLayout top = row();
    LineIcon back = new LineIcon(this, "back", INK);
    back.setContentDescription("退出并返回登录");
    back.setPadding(dp(10), dp(10), dp(10), dp(10));
    back.setOnClickListener(v -> exitAccount());
    LinearLayout.LayoutParams bp = lp(44, 44);
    bp.leftMargin = -dp(10);
    top.addView(back, bp);
    add(body, top, -1, 44);
    gap(body, 12);
    ImageView mascot = art("mascot");
    mascot.setBackground(bg(PALE, 18));
    mascot.setPadding(dp(4), dp(4), dp(4), dp(4));
    add(body, mascot, 56, 56);
    gap(body, 28);
    add(body, text("绑定邀请码", 26, INK, true), -1, -2);
    gap(body, 8);
    TextView subtitle = label("填写好友的邀请码，绑定后即可领取每日福利", 13, MUTED);
    subtitle.setLineSpacing(dp(3), 1);
    add(body, subtitle, -1, -2);
    gap(body, 24);
    // Visible-password variation makes Chinese IMEs open their Latin layout instead of pinyin.
    EditText code =
        input(
            "请输入邀请码",
            InputType.TYPE_CLASS_TEXT
                | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
                | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                | InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS);
    code.setFilters(
        new InputFilter[] {
          new InputFilter.AllCaps(),
          new InputFilter.LengthFilter(8),
          (source, start, end, dest, dstart, dend) ->
              source.subSequence(start, end).toString().replaceAll("[^A-Z0-9]", "")
        });
    code.setTextSize(18);
    code.setImeOptions(android.view.inputmethod.EditorInfo.IME_ACTION_DONE);
    LinearLayout codeBox = fieldBox();
    code.setBackground(null);
    flex(codeBox, code);
    focusRing(codeBox, code);
    add(body, codeBox, -1, 54);
    gap(body, 8);
    add(body, label("邀请码由 6 位字母和数字组成，可向邀请你的好友索取", 12, 0xffbaa494), -1, -2);
    gap(body, 24);
    TextView bind = button("确认绑定", () -> {});
    Runnable validate =
        () -> {
          // Typed characters get spacing for readability; the hint keeps normal spacing.
          code.setLetterSpacing(code.length() > 0 ? .12f : 0);
          bind.setAlpha(code.length() >= 6 ? 1 : .45f);
        };
    code.addTextChangedListener(
        new android.text.TextWatcher() {
          @Override
          public void beforeTextChanged(CharSequence s, int a, int b, int c) {}

          @Override
          public void onTextChanged(CharSequence s, int a, int b, int c) {}

          @Override
          public void afterTextChanged(android.text.Editable s) {
            validate.run();
          }
        });
    Runnable submit =
        () -> {
          if (code.length() < 6) {
            tip("请输入完整的邀请码");
            return;
          }
          hideKeyboard(bind);
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
                validate.run();
                error(message);
              });
        };
    bind.setOnClickListener(
        v -> {
          press(v);
          submit.run();
        });
    code.setOnEditorActionListener(
        (v, actionId, event) -> {
          if (actionId != android.view.inputmethod.EditorInfo.IME_ACTION_DONE) return false;
          submit.run();
          return true;
        });
    add(body, bind, -1, 52);
    gap(body, 18);
    TextView leave = label("暂不绑定，退出登录", 14, MUTED);
    leave.setGravity(Gravity.CENTER);
    leave.setPadding(0, dp(10), 0, dp(10));
    leave.setOnClickListener(v -> exitAccount());
    add(body, leave, -1, -2);
    validate.run();
  }

  void exitAccount() {
    api.post("/auth/logout", Api.object(), result -> {}, this::error);
    activities.clear();
    api.saveToken("");
    endHallSession();
    user = null;
    newbie = null;
    wheelInfo = null;
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
    handler.removeCallbacksAndMessages(SMS_COUNTDOWN);
    stage.removeAllViews();
    // "login" = account (phone or username) + password, the default; "login_code" = phone + SMS code.
    boolean codeLogin = mode.equals("login_code");
    boolean login = mode.equals("login") || codeLogin, reset = mode.equals("reset");
    boolean passwordLogin = login && !codeLogin;
    boolean usernameRegister =
        mode.equals("register")
            && api.authConfig != null
            && "username".equals(api.authConfig.optString("register_mode"));
    boolean usesPhone = !passwordLogin && !usernameRegister;
    // Password change is opened from settings while signed in; it returns there instead of login.
    boolean fromSettings = reset && user != null && !api.token.isEmpty();
    authBack =
        mode.equals("login")
            ? null
            : fromSettings
                ? () -> {
                  authBack = null;
                  api.behavior.sensitive(false);
                  current = settingsReturn;
                  settings();
                }
                : () -> auth("login");
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    scroll.setVerticalScrollBarEnabled(false);
    LinearLayout body = col();
    body.setPadding(dp(24), dp(4), dp(24), dp(24));
    scroll.addView(body);
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));

    // Sub-pages get a standard back affordance in the top-left; login is the root.
    LinearLayout top = row();
    if (!mode.equals("login")) {
      LineIcon back = new LineIcon(this, "back", INK);
      back.setContentDescription(fromSettings ? "返回设置" : "返回登录");
      back.setPadding(dp(10), dp(10), dp(10), dp(10));
      back.setOnClickListener(v -> authBack.run());
      LinearLayout.LayoutParams bp = lp(44, 44);
      bp.leftMargin = -dp(10);
      top.addView(back, bp);
    }
    add(body, top, -1, 44);
    gap(body, 12);

    LinearLayout brand = row();
    ImageView mascot = art("mascot");
    mascot.setBackground(bg(PALE, 18));
    mascot.setPadding(dp(4), dp(4), dp(4), dp(4));
    add(brand, mascot, 56, 56);
    LinearLayout name = col();
    name.setPadding(dp(12), 0, 0, 0);
    add(name, text("小橙有喜", 15, INK, true), -2, -2);
    gap(name, 4);
    add(name, label("自在生活 · 天天有礼", 12, MUTED), -2, -2);
    flex(brand, name);
    add(body, brand, -1, -2);
    gap(body, 28);
    add(
        body,
        text(
            passwordLogin
                ? "账号登录"
                : codeLogin ? "验证码登录" : fromSettings ? "修改密码" : reset ? "找回密码" : "注册新账号",
            26,
            INK,
            true),
        -1,
        -2);
    gap(body, 8);
    add(
        body,
        label(
            passwordLogin
                ? "手机号或用户名均可登录"
                : codeLogin
                    ? "未注册的手机号请先注册"
                    : reset
                        ? "验证手机号后设置新密码"
                        : usernameRegister ? "设置用户名和密码，马上开始领福利" : "注册后绑定好友邀请码，即可领取每日福利",
            13,
            MUTED),
        -1,
        -2);
    gap(body, 24);

    EditText account =
        input(
            passwordLogin ? "手机号 / 用户名" : "设置用户名（字母开头，4-20 位）",
            InputType.TYPE_CLASS_TEXT
                | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
                | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
    account.setFilters(new InputFilter[] {new InputFilter.LengthFilter(20)});
    account.setTypeface(Typeface.DEFAULT);
    account.setBackground(null);
    account.setPadding(dp(18), 0, dp(12), 0);
    account.setImeOptions(android.view.inputmethod.EditorInfo.IME_ACTION_NEXT);
    if (BuildConfig.DEBUG && passwordLogin) account.setText("13800138000");
    if (!usesPhone) {
      LinearLayout accountBox = fieldBox();
      flex(accountBox, account);
      focusRing(accountBox, account);
      add(body, accountBox, -1, 54);
    }

    EditText phone = input("请输入手机号", InputType.TYPE_CLASS_PHONE);
    phone.setFilters(new InputFilter[] {new InputFilter.LengthFilter(11)});
    phone.setBackground(null);
    phone.setPadding(dp(12), 0, dp(12), 0);
    phone.setImeOptions(android.view.inputmethod.EditorInfo.IME_ACTION_NEXT);
    if (BuildConfig.DEBUG && codeLogin) phone.setText("13800138000");
    if (fromSettings) phone.setText(user.optString("phone"));
    LinearLayout phoneBox = fieldBox();
    TextView prefix = text("+86", 15, INK, true);
    prefix.setPadding(dp(18), 0, dp(12), 0);
    add(phoneBox, prefix, -2, -2);
    View divider = new View(this);
    divider.setBackgroundColor(0xffeee2d8);
    add(phoneBox, divider, 1, 20);
    flex(phoneBox, phone);
    focusRing(phoneBox, phone);
    if (usesPhone) {
      add(body, phoneBox, -1, 54);
      gap(body, 12);
    }

    EditText code = input("请输入验证码", InputType.TYPE_CLASS_NUMBER);
    code.setFilters(new InputFilter[] {new InputFilter.LengthFilter(6)});
    code.setBackground(null);
    code.setImeOptions(
        reset || !login
            ? android.view.inputmethod.EditorInfo.IME_ACTION_NEXT
            : android.view.inputmethod.EditorInfo.IME_ACTION_DONE);
    LinearLayout codeBox = fieldBox();
    flex(codeBox, code);
    TextView send = text("获取验证码", 14, RED, true);
    send.setGravity(Gravity.CENTER);
    send.setPadding(dp(14), 0, dp(18), 0);
    add(codeBox, send, -2, -1);
    focusRing(codeBox, code);
    if (usesPhone) add(body, codeBox, -1, 54);

    final EditText password;
    if (!codeLogin) {
      if (usesPhone) gap(body, 12);
      else gap(body, 12);
      password =
          input(
              passwordLogin ? "请输入密码" : reset ? "设置新密码，至少 8 位" : "设置登录密码，至少 8 位",
              InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
      password.setImeOptions(android.view.inputmethod.EditorInfo.IME_ACTION_DONE);
      View box = passwordField(password);
      focusRing(box, password);
      add(body, box, -1, 54);
    } else {
      password = null;
    }
    gap(body, 16);

    CheckBox agree = new CheckBox(this);
    LinearLayout agreement = null;
    if (!reset) {
      agreement = row();
      agree.setButtonTintList(android.content.res.ColorStateList.valueOf(RED));
      agree.setContentDescription("同意用户协议和隐私政策");
      agree.setMinWidth(0);
      agree.setMinHeight(0);
      agree.setPadding(0, 0, 0, 0);
      LinearLayout.LayoutParams ap = lp(-2, -2);
      ap.leftMargin = -dp(5);
      ap.rightMargin = dp(2);
      agreement.addView(agree, ap);
      TextView terms = label("", 12, MUTED);
      terms.setLineSpacing(dp(3), 1);
      android.text.SpannableStringBuilder copy =
          new android.text.SpannableStringBuilder("我已阅读并同意");
      policyLink(copy, "《用户协议》", "/terms");
      copy.append("和");
      policyLink(copy, "《隐私政策》", "/privacy");
      terms.setText(copy);
      terms.setMovementMethod(android.text.method.LinkMovementMethod.getInstance());
      terms.setHighlightColor(0);
      terms.setOnClickListener(v -> agree.toggle());
      flex(agreement, terms);
      add(body, agreement, -1, -2);
      gap(body, 18);
    }

    final String[] captchaId = {""};
    final String[] captchaAnswer = {""};
    TextView submit = button(login ? "登录" : reset ? "保存新密码" : "注册并登录", () -> {});
    Runnable validate =
        () -> {
          boolean ready =
              (usesPhone
                      ? phone.length() == 11 && code.length() > 0
                      : account.length() >= (passwordLogin ? 1 : 4))
                  && (password == null || password.length() >= (passwordLogin ? 1 : 8));
          submit.setAlpha(ready ? 1 : .45f);
          submit.setTag(ready);
          boolean canSend = phone.length() == 11 && send.getTag() == null;
          send.setEnabled(canSend);
          send.setTextColor(canSend ? RED : 0xffc9b4a6);
        };
    android.text.TextWatcher watcher =
        new android.text.TextWatcher() {
          @Override
          public void beforeTextChanged(CharSequence s, int a, int b, int c) {}

          @Override
          public void onTextChanged(CharSequence s, int a, int b, int c) {}

          @Override
          public void afterTextChanged(android.text.Editable s) {
            validate.run();
          }
        };
    phone.addTextChangedListener(watcher);
    account.addTextChangedListener(watcher);
    code.addTextChangedListener(watcher);
    if (password != null) password.addTextChangedListener(watcher);

    send.setOnClickListener(
        v -> {
          if (send.getTag() != null) return;
          send.setTag("sending");
          validate.run();
          api.post(
              "/auth/sms",
              Api.object(
                  "phone",
                  phone.getText().toString(),
                  "purpose",
                  login ? "login" : reset ? "reset" : "register"),
              d -> {
                JSONObject r = (JSONObject) d;
                // Only the local test server returns the code; production delivers it by SMS.
                if (r.has("code")) code.setText(r.optString("code"));
                code.requestFocus();
                code.setSelection(code.length());
                tip("验证码已发送");
                smsCountdown(send, 60, validate);
              },
              message -> {
                send.setTag(null);
                validate.run();
                error(message);
              });
        });

    final Runnable[] submitRequest = new Runnable[1];
    submitRequest[0] =
        () -> {
          String path = login ? "/auth/login" : reset ? "/auth/reset-password" : "/auth/register";
          JSONObject request =
              passwordLogin
                  ? Api.object(
                      "phone", account.getText().toString().trim(),
                      "password", password.getText().toString())
                  : usernameRegister
                      ? Api.object(
                          "username", account.getText().toString().trim(),
                          "password", password.getText().toString())
                      : Api.object(
                          "phone",
                          phone.getText().toString(),
                          "code",
                          code.getText().toString(),
                          "password",
                          codeLogin ? "" : password.getText().toString());
          if (login && !captchaId[0].isEmpty()) {
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
                validate.run();
                if (reset) {
                  endHallSession();
                  api.saveToken("");
                  user = null;
    newbie = null;
    wheelInfo = null;
                  page = null;
                  tip("密码已更新，请重新登录");
                  auth("login");
                } else acceptAuth(d);
              },
              new Api.Failure() {
                @Override
                public void failed(String message) {
                  submit.setEnabled(true);
                  validate.run();
                  error(message);
                }

                @Override
                public void failed(int status, String message, String errorCode) {
                  if (login && "LOGIN_CAPTCHA_REQUIRED".equals(errorCode)) {
                    requestLoginCaptcha(
                        passwordLogin ? account : phone, submit, captchaId, captchaAnswer, submitRequest[0]);
                    return;
                  }
                  failed(message);
                }
              });
        };
    LinearLayout agreementRow = agreement;
    Runnable attempt =
        () -> {
          if (!Boolean.TRUE.equals(submit.getTag())) {
            if (usesPhone && phone.length() != 11) tip("请输入 11 位手机号");
            else if (usesPhone && code.length() == 0) tip("请输入验证码");
            else if (!usesPhone && account.length() == 0)
              tip(passwordLogin ? "请输入手机号或用户名" : "请设置用户名");
            else if (usernameRegister && account.length() < 4) tip("用户名至少 4 位");
            else tip(passwordLogin ? "请输入密码" : "密码至少需要 8 位");
            return;
          }
          if (agreementRow != null && !agree.isChecked()) {
            tip("请先阅读并勾选同意用户协议和隐私政策");
            agreementRow
                .animate()
                .translationX(dp(6))
                .setDuration(60)
                .withEndAction(
                    () ->
                        agreementRow
                            .animate()
                            .translationX(0)
                            .setInterpolator(new OvershootInterpolator(4))
                            .setDuration(240)
                            .start())
                .start();
            return;
          }
          hideKeyboard(submit);
          submit.setEnabled(false);
          submit.setAlpha(.65f);
          submitRequest[0].run();
        };
    submit.setOnClickListener(
        v -> {
          press(v);
          attempt.run();
        });
    (password != null ? password : code)
        .setOnEditorActionListener(
            (v, actionId, event) -> {
              if (actionId != android.view.inputmethod.EditorInfo.IME_ACTION_DONE) return false;
              attempt.run();
              return true;
            });
    add(body, submit, -1, 52);
    gap(body, 6);

    if (login) {
      LinearLayout methods = row();
      TextView toggle = text(passwordLogin ? "验证码登录" : "密码登录", 14, RED, true);
      toggle.setPadding(0, dp(10), dp(10), dp(10));
      toggle.setOnClickListener(v -> auth(passwordLogin ? "login_code" : "login"));
      add(methods, toggle, -2, -2);
      methods.addView(new View(this), new LinearLayout.LayoutParams(0, 1, 1));
      if (passwordLogin) {
        TextView forgot = label("忘记密码？", 14, MUTED);
        forgot.setPadding(dp(10), dp(10), 0, dp(10));
        forgot.setOnClickListener(v -> auth("reset"));
        add(methods, forgot, -2, -2);
      }
      add(body, methods, -1, -2);
    }
    gap(body, 10);

    LinearLayout switchRow = row();
    switchRow.setGravity(Gravity.CENTER);
    add(switchRow, label(login ? "还没有账号？" : "已有账号？", 14, MUTED), -2, -2);
    TextView switchMode = text(login ? "立即注册" : "去登录", 14, RED, true);
    switchMode.setPadding(dp(4), dp(10), dp(4), dp(10));
    switchMode.setOnClickListener(v -> auth(login ? "register" : "login"));
    add(switchRow, switchMode, -2, -2);
    if (!fromSettings) add(body, switchRow, -1, -2);
    validate.run();
  }

  static final Object SMS_COUNTDOWN = new Object();

  void smsCountdown(TextView send, int seconds, Runnable validate) {
    if (!send.isAttachedToWindow()) return;
    if (seconds <= 0) {
      send.setTag(null);
      send.setText("重新获取");
      validate.run();
      return;
    }
    send.setTag("countdown");
    send.setText(seconds + " 秒后重发");
    validate.run();
    handler.postAtTime(
        () -> smsCountdown(send, seconds - 1, validate),
        SMS_COUNTDOWN,
        android.os.SystemClock.uptimeMillis() + 1000);
  }

  void policyLink(android.text.SpannableStringBuilder text, String title, String path) {
    int start = text.length();
    text.append(title);
    text.setSpan(
        new android.text.style.ClickableSpan() {
          @Override
          public void onClick(View widget) {
            openPolicy(path);
          }

          @Override
          public void updateDrawState(android.text.TextPaint paint) {
            paint.setColor(0xffd9653f);
            paint.setUnderlineText(false);
          }
        },
        start,
        text.length(),
        android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
  }

  LinearLayout fieldBox() {
    LinearLayout box = row();
    box.setBackground(fieldBackground(false));
    return box;
  }

  GradientDrawable fieldBackground(boolean focused) {
    GradientDrawable d = bg(WHITE, 14);
    d.setStroke(dp(focused ? 1.5f : 1), focused ? 0xffff9a6b : 0xfff1e4da);
    return d;
  }

  void focusRing(View box, EditText field) {
    box.setBackground(fieldBackground(false));
    field.setOnFocusChangeListener((v, focused) -> box.setBackground(fieldBackground(focused)));
  }

  void hideKeyboard(View view) {
    android.view.inputmethod.InputMethodManager keyboard =
        (android.view.inputmethod.InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
    if (keyboard != null) keyboard.hideSoftInputFromWindow(view.getWindowToken(), 0);
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
    authBack = null;
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
          refreshHallMessages();
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
    connectHallStream();
    if (current.equals("hall")) render();
  }

  int hallRetry;
  final Runnable hallReconnect = this::ensureHallStream;

  void connectHallStream() {
    handler.removeCallbacks(hallReconnect);
    if (hallStream != null) hallStream.close();
    String token = api.token;
    hallStream =
        new HallStream(
            this,
            token,
            item -> {
              hallRetry = 0;
              appendHallItem(item);
            },
            message -> {
              // Keep what is already on screen and reconnect quietly with backoff.
              android.util.Log.w("HallStream", "disconnected: " + message);
              if (!token.equals(api.token)) return;
              hallRetry = Math.min(hallRetry + 1, 5);
              handler.postDelayed(hallReconnect, 1000L << hallRetry);
            });
  }

  // Restarts the hall stream if it died, without clearing the messages already shown.
  void ensureHallStream() {
    if (user == null
        || api.token.isEmpty()
        || user.optBoolean("invitation_required")
        || page == null
        || !foreground) return;
    if (hallStream != null && !hallStream.closed()) return;
    connectHallStream();
  }

  void endHallSession() {
    rewardUiBusy = false;
    closeRewardStatus();
    pendingReward = null;
    rain.reset();
    handler.removeCallbacks(hallReconnect);
    hallRetry = 0;
    if (hallStream != null) hallStream.close();
    hallStream = null;
    hallItems.clear();
    hallAnimated.clear();
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
    // A reconnected stream re-sends its greeting and first packet; skip what is already shown.
    String kind = item.optString("kind");
    for (JSONObject old : hallItems) {
      if (!old.optString("kind").equals(kind)) continue;
      if (kind.equals("text") && old.optString("text").equals(item.optString("text"))) return;
      if (kind.equals("red_packet")) {
        JSONObject a = old.optJSONObject("task"), b = item.optJSONObject("task");
        if (a != null && b != null && a.optString("id").equals(b.optString("id"))) return;
      }
    }
    hallItems.add(item);
    if (hallMessages == null || hallScroll == null) return;
    if (hallItems.size() == 1) {
      render();
      return;
    }
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
      // Balance: a flat coin and the number, no container.
      LinearLayout pill = row();
      pill.setPadding(dp(8), 0, 0, 0);
      add(pill, new CoinIcon(this), 20, 20);
      balance = text(number(user.optInt("coins")), 18, INK, true);
      balance.setPadding(dp(6), 0, 0, 0);
      balance.setFontFeatureSettings("tnum");
      add(pill, balance, -2, -2);
      pill.setContentDescription("我的金币 " + user.optInt("coins"));
      pill.setOnClickListener(
          v -> {
            current = "me";
            render();
          });
      add(header, pill, -2, 40);
    } else {
      LinearLayout settings = row();
      settings.setPadding(dp(12), 0, dp(4), 0);
      add(settings, label("设置", 15, 0xff7d665a), -2, -2);
      add(settings, new LineIcon(this, "arrow", 0xffcbb9ae), 14, 14);
      settings.setOnClickListener(v -> settings());
      add(header, settings, -2, 44);
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
    rainTime.setGravity(Gravity.CENTER_VERTICAL);
    add(rainStatus, rainTime, -2, -1);
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
    // During 新人礼包 the floating badge takes the mascot's corner.
    if (newbieOn() || wheelOn()) mascot.setVisibility(View.INVISIBLE);
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
    if (hallItems.isEmpty()) {
      chatText(hallMessages, "小橙正在赶来，马上就有惊喜…", true);
      ensureHallStream();
    }
    hallScroll = (ScrollView) content.getParent();
    ScrollView scroll = hallScroll;
    int savedScroll = hallScrollY;
    scroll.post(() -> scroll.scrollTo(0, savedScroll));
    addNewbieBadge();
    addWheelBadge();
    if (foreground) rain.attach(hallSurface, rainTime);
  }


  // ---- 新人礼包 (activity page web/public/activity/newbie.html; amounts decided by the server) ----
  static final String NEWBIE_PAGE = "/activity/newbie.html";
  static final String PENDING_INVITE = "pending_invite_code";
  JSONObject newbie;
  boolean newbieLoading;

  boolean newbieOn() {
    return newbie != null && newbie.optBoolean("enabled");
  }

  int newbieReady() {
    if (!newbieOn()) return 0;
    JSONObject w = newbie.optJSONObject("welcome");
    int n = w != null && "ready".equals(w.optString("status")) ? 1 : 0;
    JSONArray ready = newbie.optJSONArray("ready_invites");
    return n + (ready == null ? 0 : ready.length());
  }

  String newbieKey(JSONObject value) {
    if (value == null || !value.optBoolean("enabled")) return "off";
    JSONObject w = value.optJSONObject("welcome");
    JSONArray ready = value.optJSONArray("ready_invites");
    return value.optString("entry_label") + "|" + (w == null ? "-" : w.optString("status")) + "|" + (ready == null ? 0 : ready.length());
  }

  void loadNewbie() {
    if (user == null || user.optBoolean("invitation_required") || api.token.isEmpty() || newbieLoading) return;
    newbieLoading = true;
    String token = api.token;
    api.get(
        "/newbie",
        value -> {
          newbieLoading = false;
          if (!token.equals(api.token) || isDestroyed()) return;
          JSONObject next = (JSONObject) value;
          boolean changed = !newbieKey(newbie).equals(newbieKey(next));
          newbie = next;
          if (changed && current.equals("hall") && page != null && !rewardUiBusy) render();
          if (!acceptPendingInvite()) maybePopupNewbie();
        },
        message -> {
          newbieLoading = false;
          android.util.Log.w("Newbie", "GET /api/newbie: " + message);
        });
  }

  /** Once per round per account: the big packet pops up by itself while it is still unopened. */
  void maybePopupNewbie() {
    if (!newbieOn() || !newbie.optBoolean("auto_popup") || !foreground || rewardUiBusy) return;
    if (!current.equals("hall") || page == null || updates.required()) return;
    JSONObject w = newbie.optJSONObject("welcome");
    if (w == null || !"ready".equals(w.optString("status"))) return;
    String key = "newbie_popup_" + user.optLong("id") + "_" + newbie.optLong("round");
    if (api.preferences.getBoolean(key, false)) return;
    api.preferences.edit().putBoolean(key, true).apply();
    handler.postDelayed(this::openNewbie, 600);
  }

  void openNewbie() {
    openNewbie("");
  }

  /** query: "" or e.g. "claimed=1234" (back from a 新人礼包 video, to celebrate on the page). */
  void openNewbie(String query) {
    String url = WebActivity.resolve(NEWBIE_PAGE + (query.isEmpty() ? "" : "?" + query));
    if (url == null) return;
    api.behavior.click(0, "home:newbie");
    startActivityForResult(
        new Intent(this, WebActivity.class)
            .putExtra(WebActivity.EXTRA_URL, url)
            .putExtra(WebActivity.EXTRA_TITLE, newbie == null ? "新人礼包" : newbie.optString("title", "新人礼包"))
            .putExtra(WebActivity.EXTRA_IMMERSIVE, true),
        WebActivity.REQUEST);
    overridePendingTransition(android.R.anim.fade_in, 0);
  }

  /** An invite link opened the App (InviteLinkActivity stored the code): join this round. */
  boolean acceptPendingInvite() {
    String code = api.preferences.getString(PENDING_INVITE, "");
    if (code.isEmpty() || user == null || user.optBoolean("invitation_required") || !newbieOn()) return false;
    api.preferences.edit().remove(PENDING_INVITE).apply();
    if (code.equalsIgnoreCase(user.optString("invite_code"))) return false;
    api.post(
        "/newbie/accept",
        Api.object("invite_code", code),
        value -> {
          JSONObject result = (JSONObject) value;
          if (result.has("state")) newbie = result.optJSONObject("state");
          if (result.optBoolean("joined")) tip("已接受好友邀请，快去拆大红包");
          openNewbie();
        },
        message -> {
          tip(message);
          maybePopupNewbie();
        });
    return true;
  }

  // ---- 幸运转盘 (activity page web/public/activity/wheel.html) ----
  JSONObject wheelInfo;
  boolean wheelLoading;

  boolean wheelOn() {
    return wheelInfo != null && wheelInfo.optBoolean("enabled");
  }

  void loadWheel() {
    if (user == null || user.optBoolean("invitation_required") || api.token.isEmpty() || wheelLoading) return;
    wheelLoading = true;
    String token = api.token;
    api.get(
        "/wheel/badge",
        value -> {
          wheelLoading = false;
          if (!token.equals(api.token) || isDestroyed()) return;
          JSONObject next = (JSONObject) value;
          boolean changed = wheelInfo == null || !wheelInfo.toString().equals(next.toString());
          wheelInfo = next;
          if (changed && current.equals("hall") && page != null && !rewardUiBusy) render();
        },
        message -> wheelLoading = false);
  }

  void openWheel(String query) {
    String url = WebActivity.resolve("/activity/wheel.html" + (query.isEmpty() ? "" : "?" + query));
    if (url == null) return;
    api.behavior.click(0, "home:wheel");
    startActivityForResult(
        new Intent(this, WebActivity.class)
            .putExtra(WebActivity.EXTRA_URL, url)
            .putExtra(WebActivity.EXTRA_TITLE, "幸运转盘")
            .putExtra(WebActivity.EXTRA_IMMERSIVE, true),
        WebActivity.REQUEST);
    overridePendingTransition(android.R.anim.fade_in, 0);
  }

  void addWheelBadge() {
    if (!wheelOn() || hallSurface == null) return;
    NewbieBadge badge = new NewbieBadge(this, wheelInfo.optString("entry_label", "幸运转盘"), true);
    badge.setCount(wheelInfo.optInt("available"));
    FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(dp(92), dp(100), Gravity.TOP | Gravity.RIGHT);
    lp.topMargin = newbieOn() ? dp(10 + 100 + 6) : dp(10);
    lp.rightMargin = dp(16);
    hallSurface.addView(badge, lp);
    badge.setOnClickListener(
        v -> {
          press(v);
          openWheel("");
        });
    badge.post(badge::start);
  }

  void addNewbieBadge() {
    if (!newbieOn() || hallSurface == null) return;
    NewbieBadge badge = new NewbieBadge(this, newbie.optString("entry_label", "新人礼包"));
    badge.setCount(newbieReady());
    FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(dp(92), dp(100), Gravity.TOP | Gravity.RIGHT);
    lp.topMargin = dp(10);
    lp.rightMargin = dp(16);
    hallSurface.addView(badge, lp);
    badge.setOnClickListener(
        v -> {
          press(v);
          openNewbie();
        });
    badge.post(badge::start);
  }

  /** 新人礼包 video packet (amount already revealed on the page): standard rewarded-video flow
   * for task newbie:{id}; paid only after the ad network's verified callback. */
  /** "" or "newbie:{id}" / "wheel:{id}": the activity video in progress (to return to its page). */
  String newbieVideoPacket = "";

  void watchNewbieVideo(String packetId) {
    watchActivityVideo("newbie", packetId);
  }

  void watchActivityVideo(String kind, String packetId) {
    if (!packetId.matches("[0-9a-f-]{36}")) return;
    newbieVideoPacket = kind + ":" + packetId;
    rewardUiBusy = true;
    String token = api.token, task = kind + ":" + packetId;
    showBusy("视频加载中…");
    Api.Failure failed =
        m -> {
          hideBusy();
          rewardUiBusy = false;
          newbieVideoPacket = "";
          if (isDestroyed() || !token.equals(api.token)) return;
          tip(m);
          if (kind.equals("wheel")) openWheel("");
          else openNewbie();
        };
    api.quoteReward(
        task,
        quote ->
            api.post(
                "/rewards/start",
                Api.object("task_id", task, "quote_id", ((JSONObject) quote).optString("quote_id")),
                session -> {
                  if (isDestroyed() || !token.equals(api.token)) return;
                  showRewardAd((JSONObject) session, "");
                },
                failed),
        failed);
  }

  void openNative(String target) {
    if (user == null || page == null) return;
    if (target.startsWith("newbie-video:")) {
      watchActivityVideo("newbie", target.substring("newbie-video:".length()));
      return;
    }
    if (target.startsWith("wheel-video:")) {
      watchActivityVideo("wheel", target.substring("wheel-video:".length()));
      return;
    }
    switch (target) {
      case "checkin":
        openCheckin();
        break;
      case "invite":
        inviteFriends();
        break;
      case "newbie":
        openNewbie();
        break;
      case "wheel":
        openWheel("");
        break;
      case "activity":
        if (activities.config != null) openActivity();
        else tip("活动暂未开始");
        break;
      case "withdraw":
        withdraw();
        break;
      case "ledger":
        ledger();
        break;
      case "shop":
      case "me":
        current = target;
        if (target.equals("shop")) {
          refreshProducts();
          refreshProductPromo();
        }
        render();
        break;
      default:
        tip("请升级到最新版本后使用");
    }
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

  final Set<String> hallAnimated = new HashSet<>();
  boolean freshHallItem;

  // Rebuilds only the message list (no page rebuild, no replayed entrance animations).
  void refreshHallMessages() {
    if (!current.equals("hall")) return;
    if (hallMessages == null || hallScroll == null) {
      render();
      return;
    }
    hallMessages.removeAllViews();
    for (int i = 0; i < hallItems.size(); i++) hallItem(hallMessages, hallItems.get(i), i);
    if (hallItems.isEmpty()) chatText(hallMessages, "小橙正在赶来，马上就有惊喜…", true);
  }

  void hallItem(LinearLayout content, JSONObject item, int index) {
    freshHallItem = hallAnimated.add(item.optString("id"));
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
            nativeFeeds.card(item, widthDp),
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
    if (freshHallItem) {
      card.setAlpha(0);
      card.setTranslationY(dp(10));
      card.animate().alpha(1).translationY(0).setDuration(300).setStartDelay(70L).start();
    }
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
    withLocation(() -> openPacketNow(task));
  }

  void openPacketNow(JSONObject task) {
    newbieVideoPacket = "";
    rewardUiBusy = true;
    String token = api.token;
    showBusy("正在打开红包…");
    api.quoteReward(
        task.optString("id"),
        data -> {
          hideBusy();
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
            if (updated) refreshHallMessages();
            showPacket(quotedTask, quoteId, reward);
          } catch (JSONException error) {
            rewardError(error.getMessage());
          }
        },
        message -> {
          hideBusy();
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
          showBusy("视频加载中…");
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
                hideBusy();
                watch.setEnabled(true);
                error(m);
              });
        });
    dialog.show();
    playPacketOpen();
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(48), dp(360)), -2);
    shake.start();
  }

  void showRewardAd(JSONObject session, String rainPacket) {
    String token = api.token, id = session.optString("id");
    api.behavior.external(5);
    api.behavior.business(10, id);
    if (BuildConfig.ADS_ENABLED) {
      if (busyDialog == null) showBusy("视频加载中…");
      // The spinner stays until a network actually has a video ready to play.
      ads.onAdVisible = this::hideBusy;
    } else hideBusy();
    rewarded.show(
        this,
        session,
        notice -> {
          hideBusy();
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
          hideBusy();
          if (!isDestroyed() && token.equals(api.token)) {
            api.behavior.external(0);
            if (AdMediator.NO_AD.equals(message)) {
              // Plain "no ad right now" — identical whether no network filled or ads are withheld.
              rewardUiBusy = false;
              tip(AdMediator.NO_AD);
            } else rewardError(message + "\n奖励编号：" + id);
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
          if (!newbieVideoPacket.isEmpty()) {
            // Back to the activity page, which reveals what the video paid (blind box).
            String done = newbieVideoPacket;
            newbieVideoPacket = "";
            rewardUiBusy = false;
            if (foreground) {
              if (done.startsWith("wheel:")) openWheel("claimed=" + done.substring(6));
              else openNewbie("claimed=" + result.optInt("reward"));
            }
            return;
          }
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
                refreshHallMessages();
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

  /** Paper rip + chime when a packet opens. */
  void playPacketOpen() {
    coinSounds.play(packetOpenSound, .8f, .8f, 1, 0, 1f);
  }

  void playCoinArrival() {
    int stream = coinSounds.play(coinArrivalSound, .7f, .7f, 1, 0, 1f);
    if (stream == 0) android.util.Log.e("CoinSound", "金币音效播放失败");
  }

  void rainCredited(JSONObject updated, int coins, boolean animate) {
    user = updated;
    if (current.equals("hall") && balance != null)
      balance.setText(number(user.optInt("coins")));
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

  // ---- location (cell tower) permission: asked once, before the first check-in / packet /
  // withdrawal, with our own explanation first. Declining never blocks anything.
  static final String LOCATION_ASKED = "location_permission_asked_v1";
  static final int LOCATION_REQUEST = 4108;
  Runnable afterLocation;

  void withLocation(Runnable then) {
    if (DeviceTrustClient.locationGranted(this)
        || api.preferences.getBoolean(LOCATION_ASKED, false)
        || !api.preferences.getBoolean(DeviceTrustClient.CONSENT, false)) {
      then.run();
      return;
    }
    api.preferences.edit().putBoolean(LOCATION_ASKED, true).apply();
    AlertDialog ask =
        new AlertDialog.Builder(this)
            .setTitle("温馨提示")
            .setMessage(
                "为保障账户与提现安全，小橙有喜需要获取你的位置信息（含附近基站信息），仅用于识别异常设备与作弊环境，不会用于广告推送。\n\n拒绝授权不影响签到、领红包和提现。")
            .setNegativeButton("暂不授权", (d, w) -> then.run())
            .setPositiveButton(
                "去授权",
                (d, w) -> {
                  afterLocation = then;
                  requestPermissions(
                      new String[] {
                        android.Manifest.permission.ACCESS_FINE_LOCATION,
                        android.Manifest.permission.ACCESS_COARSE_LOCATION
                      },
                      LOCATION_REQUEST);
                })
            .setCancelable(false)
            .create();
    ask.show();
  }

  @Override
  public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] results) {
    super.onRequestPermissionsResult(requestCode, permissions, results);
    if (requestCode != LOCATION_REQUEST) return;
    Runnable then = afterLocation;
    afterLocation = null;
    if (DeviceTrustClient.locationGranted(this)) api.deviceTrust.refreshNow();
    if (then != null) then.run();
  }

  void openCheckin() {
    withLocation(this::openCheckinNow);
  }

  void openCheckinNow() {
    api.get("/check-in/status", value -> showCheckinDialog((JSONObject) value), this::error);
  }

  void showCheckinDialog(JSONObject status) {
    showCheckinDialog(status, false);
  }

  void showCheckinDialog(JSONObject status, boolean auto) {
    if (checkinDialog != null && checkinDialog.isShowing()) checkinDialog.dismiss();
    boolean done = user != null && user.optBoolean("checked_in_today");
    int todayReward = 0, total = 0;
    JSONArray days = status.optJSONArray("days");
    List<JSONObject> items = new ArrayList<>();
    if (days != null)
      for (int i = 0; i < days.length(); i++) {
        JSONObject item = days.optJSONObject(i);
        if (item == null) continue;
        items.add(item);
        total += item.optInt("reward");
        if (item.optString("status").equals("today")) todayReward = item.optInt("reward");
      }

    LinearLayout root = col();
    root.setGravity(Gravity.CENTER_HORIZONTAL);
    FrameLayout stack = new FrameLayout(this);
    root.addView(stack, new LinearLayout.LayoutParams(-1, -2));

    LinearLayout card = col();
    card.setBackground(bg(0xfffffaf5, 26));
    card.setClipToOutline(true);
    FrameLayout.LayoutParams cardLp = new FrameLayout.LayoutParams(-1, -2);
    cardLp.topMargin = dp(58);
    stack.addView(card, cardLp);

    // Header band
    LinearLayout band = col();
    band.setGravity(Gravity.CENTER_HORIZONTAL);
    band.setPadding(dp(18), dp(52), dp(18), dp(20));
    GradientDrawable bandBg =
        new GradientDrawable(
            GradientDrawable.Orientation.TL_BR, new int[] {0xffffa25a, 0xffff6a3d, 0xfff4492c});
    band.setBackground(bandBg);
    TextView title = text(done ? "今日已签到" : "每日签到", 26, WHITE, true);
    title.setGravity(Gravity.CENTER);
    title.setShadowLayer(dp(3), 0, dp(1), 0x33a02a10);
    add(band, title, -1, -2);
    gap(band, 6);
    int streak = status.optInt("streak");
    String streakText = "已连续签到 " + streak + " 天";
    android.text.SpannableString sub = new android.text.SpannableString(streakText + "  ·  7 天共 " + total + " 金币");
    int ns = streakText.indexOf(String.valueOf(streak));
    sub.setSpan(new android.text.style.ForegroundColorSpan(0xfffff1a8), ns, ns + String.valueOf(streak).length(), 0);
    sub.setSpan(new android.text.style.StyleSpan(android.graphics.Typeface.BOLD), ns, ns + String.valueOf(streak).length(), 0);
    TextView subView = label("", 13, 0xf2ffffff);
    subView.setText(sub);
    subView.setGravity(Gravity.CENTER);
    subView.setPadding(dp(14), dp(5), dp(14), dp(5));
    subView.setBackground(bg(0x26ffffff, 14));
    add(band, subView, -2, -2);
    add(card, band, -1, -2);

    // Day grid: 4 on top, 3 below with day 7 as the wide grand prize.
    LinearLayout body = col();
    body.setPadding(dp(14), dp(16), dp(14), dp(18));
    add(card, body, -1, -2);
    LinearLayout row1 = row(), row2 = row();
    for (int i = 0; i < items.size(); i++) {
      boolean last = i == items.size() - 1 && items.size() == 7;
      View cell = checkinCell(items.get(i), last);
      LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, dp(84), last ? 2.1f : 1);
      LinearLayout target = i < 4 ? row1 : row2;
      if (target.getChildCount() > 0) lp.leftMargin = dp(7);
      target.addView(cell, lp);
    }
    add(body, row1, -1, -2);
    gap(body, 7);
    add(body, row2, -1, -2);

    int makeups = status.optInt("makeup_available");
    if (makeups > 0) {
      gap(body, 12);
      TextView hint = label("有 " + makeups + " 天漏签，点击“补签”按当天档位补领", 11, 0xffb39583);
      hint.setGravity(Gravity.CENTER);
      add(body, hint, -1, -2);
    }
    gap(body, 16);
    TextView action;
    if (done) {
      action = text("今日已领取，明天再来", 16, 0xffb39583, true);
      action.setGravity(Gravity.CENTER);
      action.setBackground(bg(0xfff3ebe4, 26));
      action.setOnClickListener(v -> checkinDialog.dismiss());
    } else {
      action =
          button(
              todayReward > 0 ? "立即签到 · 领 " + todayReward + " 金币" : "立即签到",
              () -> dialogCloseAndCheck(null));
      action.setTextSize(17);
      action.setBackground(grad(0xffff8a3d, 0xfff2452b, 26));
      action.setElevation(dp(4));
    }
    add(body, action, -1, 54);
    if (auto && !done) {
      gap(body, 10);
      TextView later = label("关闭后可在大厅「签到领金币」再次打开", 11, 0xffc2ab9d);
      later.setGravity(Gravity.CENTER);
      add(body, later, -1, -2);
    }

    ImageView mascot = art("mascot");
    FrameLayout.LayoutParams mascotLp = new FrameLayout.LayoutParams(dp(112), dp(112));
    mascotLp.gravity = Gravity.TOP | Gravity.CENTER_HORIZONTAL;
    mascot.setElevation(dp(6));
    stack.addView(mascot, mascotLp);

    LineIcon close = new LineIcon(this, "close", WHITE);
    close.setPadding(dp(9), dp(9), dp(9), dp(9));
    GradientDrawable ring = bg(0x33000000, 99);
    ring.setStroke(dp(1.5f), 0xccffffff);
    close.setBackground(ring);
    close.setContentDescription("关闭");
    LinearLayout.LayoutParams closeLp = new LinearLayout.LayoutParams(dp(38), dp(38));
    closeLp.topMargin = dp(18);
    root.addView(close, closeLp);

    Dialog dialog = sheet(root, 3);
    close.setOnClickListener(v -> dialog.dismiss());
    dialog.show();
    dialog
        .getWindow()
        .setLayout(Math.min(getResources().getDisplayMetrics().widthPixels - dp(40), dp(380)), -2);
    checkinDialog = dialog;

    root.setScaleX(.75f);
    root.setScaleY(.75f);
    root.setAlpha(0);
    root.animate().scaleX(1).scaleY(1).alpha(1).setDuration(420)
        .setInterpolator(new OvershootInterpolator(1.3f)).start();
    mascot.setTranslationY(dp(14));
    mascot.animate().translationY(0).setStartDelay(180).setDuration(500)
        .setInterpolator(new OvershootInterpolator(2f)).start();
    if (!done) {
      android.animation.ObjectAnimator pulseX = android.animation.ObjectAnimator.ofFloat(action, "scaleX", 1f, 1.04f, 1f);
      android.animation.ObjectAnimator pulseY = android.animation.ObjectAnimator.ofFloat(action, "scaleY", 1f, 1.04f, 1f);
      for (android.animation.ObjectAnimator a : new android.animation.ObjectAnimator[] {pulseX, pulseY}) {
        a.setDuration(1100);
        a.setStartDelay(600);
        a.setRepeatCount(android.animation.ValueAnimator.INFINITE);
        a.start();
      }
      dialog.setOnDismissListener(ignored -> { pulseX.cancel(); pulseY.cancel(); });
    }
  }

  View checkinCell(JSONObject item, boolean grand) {
    int dayNumber = item.optInt("day");
    String state = item.optString("status");
    boolean got = state.equals("checked") || state.equals("makeup");
    boolean today = state.equals("today");
    boolean missed = state.equals("missed");
    LinearLayout cell = grand ? row() : col();
    cell.setGravity(Gravity.CENTER);
    GradientDrawable cellBg;
    if (today) cellBg = new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM, new int[] {0xffffb06b, 0xffff6a3d});
    else if (grand) cellBg = new GradientDrawable(GradientDrawable.Orientation.TL_BR, new int[] {0xfffff0d6, 0xffffdcc0});
    else cellBg = new GradientDrawable();
    if (!today && !grand) cellBg.setColor(got ? 0xfffff1e6 : missed ? 0xfff6f1ed : WHITE);
    cellBg.setCornerRadius(dp(14));
    if (!today) cellBg.setStroke(dp(1), grand ? 0xffffc79c : 0xfff6e3d6);
    cell.setBackground(cellBg);
    if (today) cell.setElevation(dp(3));
    int dayColor = today ? WHITE : missed ? 0xffb8a89d : MUTED;
    int rewardColor = today ? WHITE : got ? 0xffc9a48c : missed ? 0xffb8a89d : 0xffe0582f;
    if (grand) {
      cell.setPadding(dp(10), 0, dp(10), 0);
      ImageView gift = art("packet");
      if (got) gift.setAlpha(.5f);
      cell.addView(gift, new LinearLayout.LayoutParams(dp(54), dp(54)));
      LinearLayout txt = col();
      txt.setPadding(dp(6), 0, 0, 0);
      add(txt, text(today ? "今天 · 大礼" : "第7天 · 大礼", 12, today ? WHITE : 0xffc0602f, true), -2, -2);
      gap(txt, 3);
      add(txt, text(missed ? "补签" : got ? "已领取" : "+" + item.optInt("reward") + " 金币", 16, today ? WHITE : missed ? RED : got ? 0xffc9a48c : 0xffe0582f, true), -2, -2);
      cell.addView(txt, new LinearLayout.LayoutParams(-2, -2));
    } else {
      TextView d = text(today ? "今天" : "第" + dayNumber + "天", 11, dayColor, today);
      d.setGravity(Gravity.CENTER);
      add(cell, d, -1, -2);
      gap(cell, 5);
      FrameLayout icon = new FrameLayout(this);
      GradientDrawable disc = bg(today ? 0x33ffffff : got ? 0xffffe2cc : missed ? 0xffece4de : 0xffffefd9, 99);
      icon.setBackground(disc);
      LineIcon li = new LineIcon(this, got ? "check" : "coin", today ? WHITE : got ? 0xffe08a2c : missed ? 0xffc9b8ad : 0xffe08a2c);
      FrameLayout.LayoutParams ilp = new FrameLayout.LayoutParams(dp(18), dp(18));
      ilp.gravity = Gravity.CENTER;
      icon.addView(li, ilp);
      cell.addView(icon, new LinearLayout.LayoutParams(dp(28), dp(28)));
      gap(cell, 5);
      TextView r;
      if (missed) {
        r = text("补签", 11, WHITE, true);
        r.setBackground(bg(RED, 99));
        r.setPadding(dp(8), dp(1), dp(8), dp(1));
      } else r = text(got ? "已领" : "+" + item.optInt("reward"), 12, rewardColor, true);
      r.setGravity(Gravity.CENTER);
      add(cell, r, -2, -2);
    }
    if (missed) cell.setOnClickListener(v -> { press(v); makeup(dayNumber); });
    return cell;
  }

  // Opens the check-in popup once per user per Beijing day, after startup ads/updates are out of the way.
  void maybePromptCheckin() {
    if (user == null
        || checkinStatus == null
        || user.optBoolean("checked_in_today")
        || user.optBoolean("invitation_required")
        || page == null
        || updates.required()
        || !canPresentUpdate()
        || (checkinDialog != null && checkinDialog.isShowing())) return;
    java.text.SimpleDateFormat f = new java.text.SimpleDateFormat("yyyy-MM-dd", java.util.Locale.US);
    f.setTimeZone(java.util.TimeZone.getTimeZone("Asia/Shanghai"));
    String key = "checkin_prompt_" + user.optLong("id");
    String today = f.format(new java.util.Date());
    if (today.equals(api.preferences.getString(key, ""))) return;
    api.preferences.edit().putString(key, today).apply();
    showCheckinDialog(checkinStatus, true);
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
        .setTitle("温馨提示")
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
    c.setPadding(dp(17), dp(14), dp(shopPromoImage.isEmpty() ? 17 : 100), dp(14));
    add(c, text("认真生活，聪明买", 18, INK, true), -1, -2);
    gap(c, 6);
    add(c, label("心动好物，把金币也带回家", 11, 0xffaf7a55), -1, -2);
    gap(c, 8);
    TextView small = text("每日精选  ·  好价发现", 10, RED, true);
    add(c, small, -1, -2);
    promo.addView(c, new FrameLayout.LayoutParams(-1, -2));
    if (!shopPromoImage.isEmpty()) {
      ProductImage promoImage =
          new ProductImage(this, shopPromoImage, "好物精选", ImageView.ScaleType.CENTER_CROP);
      FrameLayout.LayoutParams bp =
          new FrameLayout.LayoutParams(dp(100), dp(100), Gravity.RIGHT | Gravity.BOTTOM);
      promo.addView(promoImage, bp);
    }
    add(content, promo, -1, shopPromoImage.isEmpty() ? -2 : 100);
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
    TextView platform = text(p.optString("platform"), 11, MUTED, false);
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
    LinearLayout identity = col();
    identity.setPadding(dp(12), 0, dp(8), 0);
    TextView nickname = text(user.optString("nickname", "小橙朋友"), 17, INK, true);
    nickname.setSingleLine();
    nickname.setEllipsize(android.text.TextUtils.TruncateAt.END);
    add(identity, nickname, -1, -2);
    gap(identity, 5);
    String phone = user.optString("phone");
    add(
        identity,
        label(
            phone.length() == 11
                ? phone.substring(0, 3) + "****" + phone.substring(7)
                : phone.isEmpty() ? "用户名：" + user.optString("username") : phone,
            12,
            MUTED),
        -1,
        -2);
    flex(profile, identity);
    add(content, profile, -1, 58);
    gap(content, 12);
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
    menu(actions, "shield", "收款账户", this::payoutAccounts);
    line(actions);
    menu(actions, "coin", "金币明细", this::ledger);
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
    authBack = null;
    api.behavior.page(12);
    settingsReturn = current;
    current = "settings";
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    scroll.setVerticalScrollBarEnabled(false);
    LinearLayout body = col();
    body.setPadding(dp(20), dp(4), dp(20), dp(28));
    scroll.addView(body);
    stage.removeAllViews();
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));

    LinearLayout top = row();
    LineIcon back = new LineIcon(this, "back", INK);
    back.setContentDescription("返回");
    back.setPadding(dp(10), dp(10), dp(10), dp(10));
    back.setOnClickListener(v -> closeSettings());
    LinearLayout.LayoutParams bp = lp(44, 44);
    bp.leftMargin = -dp(10);
    top.addView(back, bp);
    flex(top, text("设置", 20, INK, true));
    add(body, top, -1, 56);
    gap(body, 8);

    settingsGroup(body, "账号与安全");
    LinearLayout account = col();
    account.setBackground(bg(WHITE, 18));
    settingRow(account, "修改密码", "", () -> auth("reset"));
    line(account);
    settingRow(account, "注销账号", "需审核", this::confirmDeleteAccount);
    add(body, account, -1, -2);
    gap(body, 20);

    settingsGroup(body, "关于");
    LinearLayout about = col();
    about.setBackground(bg(WHITE, 18));
    settingRow(about, "检查更新", "当前 " + BuildConfig.VERSION_NAME, updates::manual);
    line(about);
    settingRow(about, "隐私政策", "", () -> openPolicy("/privacy"));
    line(about);
    settingRow(about, "第三方 SDK 清单", "", () -> openPolicy("/privacy#sdk"));
    line(about);
    settingRow(about, "用户协议", "", () -> openPolicy("/terms"));
    add(body, about, -1, -2);
    gap(body, 28);

    TextView logout = text("退出登录", 16, RED, true);
    logout.setGravity(Gravity.CENTER);
    logout.setBackground(bg(WHITE, 18));
    logout.setOnClickListener(v -> exitAccount());
    add(body, logout, -1, 52);
  }

  void settingsGroup(LinearLayout body, String title) {
    TextView label = label(title, 13, MUTED);
    label.setPadding(dp(4), 0, 0, dp(8));
    add(body, label, -1, -2);
  }

  void settingRow(LinearLayout card, String title, String value, Runnable action) {
    LinearLayout r = row();
    r.setPadding(dp(18), 0, dp(14), 0);
    flex(r, label(title, 15, INK));
    if (!value.isEmpty()) {
      TextView v = label(value, 13, MUTED);
      v.setPadding(0, 0, dp(6), 0);
      add(r, v, -2, -2);
    }
    add(r, new LineIcon(this, "arrow", 0xffcbb9ae), 15, 15);
    r.setOnClickListener(v -> action.run());
    add(card, r, -1, 54);
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
          // While 新人礼包 runs, the poster advertises the invited friend's packet.
          if (newbieOn()) {
            try {
              summary.put("promo_max", newbie.optInt("invited_max", newbie.optInt("welcome_max")));
            } catch (JSONException ignored) {
            }
          }
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

  /** 邀请好友: the poster, the invite code (tap to copy) and two ways to send it. */
  void invitationPreview(JSONObject summary, Bitmap poster) {
    String code = summary.optString("invite_code"), url = summary.optString("invite_url");
    Dialog dialog = new Dialog(this);
    LinearLayout content = col();
    content.setBackground(bg(CREAM, 25));
    content.setClipToOutline(true);
    content.setPadding(dp(18), dp(16), dp(18), dp(18));
    LinearLayout title = row();
    flex(title, text("邀请好友", 21, INK, true));
    LineIcon close = new LineIcon(this, "close", MUTED);
    close.setContentDescription("关闭");
    close.setOnClickListener(v -> dialog.dismiss());
    add(title, close, 22, 22);
    add(content, title, -1, 30);
    gap(content, 12);
    ImageView preview = new ImageView(this);
    preview.setImageBitmap(poster);
    preview.setScaleType(ImageView.ScaleType.FIT_CENTER);
    preview.setAdjustViewBounds(true);
    int screenH = getResources().getDisplayMetrics().heightPixels;
    add(content, preview, -1, Math.max(260, Math.min(430, (int) (screenH / getResources().getDisplayMetrics().density) - 300)));
    gap(content, 12);
    TextView codeLine = label("邀请码 " + code + "  ·  点击复制", 13, MUTED);
    codeLine.setGravity(Gravity.CENTER);
    codeLine.setOnClickListener(v -> copyInvite("邀请码", code));
    add(content, codeLine, -1, 30);
    gap(content, 10);
    LinearLayout actions = row();
    TextView wechat = text("微信好友", 16, WHITE, true);
    wechat.setGravity(Gravity.CENTER);
    wechat.setBackground(grad(0xff2fd05a, 0xff14a83c, 18));
    wechat.setOnClickListener(
        v -> {
          press(v);
          PosterShare.share(this, "wechat", poster, url);
        });
    TextView moments = text("朋友圈", 16, WHITE, true);
    moments.setGravity(Gravity.CENTER);
    moments.setBackground(grad(0xffff8a3d, 0xfff0551f, 18));
    moments.setOnClickListener(
        v -> {
          press(v);
          PosterShare.share(this, "moments", poster, url);
        });
    LinearLayout.LayoutParams wp = new LinearLayout.LayoutParams(0, dp(50), 1);
    wp.rightMargin = dp(10);
    actions.addView(wechat, wp);
    actions.addView(moments, new LinearLayout.LayoutParams(0, dp(50), 1));
    add(content, actions, -1, -2);
    dialog.setContentView(content);
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

  /** 金币明细: balance summary, 全部/收入/支出 filter, entries grouped by day. */
  void ledger() {
    api.get("/ledger", d -> ledgerPage((JSONArray) d, "all"), this::error);
  }

  void ledgerPage(JSONArray items, String filter) {
    LinearLayout body = subPage("ledger", "金币明细", CREAM);

    // summary
    LinearLayout card = col();
    card.setBackground(grad(0xffff8a4c, RED, 22));
    card.setPadding(dp(20), dp(18), dp(20), dp(16));
    LinearLayout top = row();
    LinearLayout left = col();
    add(left, label("当前金币", 13, 0xccffffff), -2, -2);
    gap(left, 6);
    LinearLayout amountRow = row();
    add(amountRow, new CoinIcon(this), 28, 28);
    TextView amount = text(" " + number(user.optInt("coins")), 32, WHITE, true);
    amountRow.addView(amount);
    add(left, amountRow, -2, -2);
    gap(left, 4);
    add(left, label("约 ¥" + money(user.optInt("coins") / 1000.0), 12, 0xddffffff), -2, -2);
    flex(top, left);
    TextView go = text("去提现", 13, RED, true);
    go.setGravity(Gravity.CENTER);
    go.setPadding(dp(16), dp(8), dp(16), dp(8));
    go.setBackground(bg(WHITE, 18));
    go.setOnClickListener(v -> {
      press(v);
      withdraw();
    });
    top.addView(go);
    add(card, top, -1, -2);
    gap(card, 14);
    View divider = new View(this);
    divider.setBackgroundColor(0x33ffffff);
    add(card, divider, -1, 1);
    gap(card, 12);
    LinearLayout stats = row();
    flex(stats, statCell("今日收入", user.optInt("today_earned")));
    flex(stats, statCell("累计收入", user.optInt("total_earned")));
    add(card, stats, -1, -2);
    add(body, card, -1, -2);
    gap(body, 16);

    // filter
    LinearLayout tabs = row();
    tabs.setPadding(dp(4), dp(4), dp(4), dp(4));
    tabs.setBackground(bg(0xfff3e6da, 16));
    String[][] options = {{"all", "全部"}, {"in", "收入"}, {"out", "支出"}};
    for (String[] o : options) {
      boolean on = o[0].equals(filter);
      TextView t = text(o[1], 14, on ? RED : 0xff9c8576, on);
      t.setGravity(Gravity.CENTER);
      if (on) t.setBackground(bg(WHITE, 13));
      t.setOnClickListener(v -> ledgerPage(items, o[0]));
      tabs.addView(t, new LinearLayout.LayoutParams(0, dp(34), 1));
    }
    add(body, tabs, -1, -2);
    gap(body, 14);

    // grouped list
    SimpleDateFormat in = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.CHINA);
    in.setTimeZone(TimeZone.getTimeZone("UTC"));
    SimpleDateFormat dayKey = new SimpleDateFormat("yyyy-MM-dd", Locale.CHINA);
    SimpleDateFormat dayLabel = new SimpleDateFormat("M月d日 EEEE", Locale.CHINA);
    SimpleDateFormat clock = new SimpleDateFormat("HH:mm", Locale.CHINA);
    TimeZone bj = TimeZone.getTimeZone("Asia/Shanghai");
    dayKey.setTimeZone(bj);
    dayLabel.setTimeZone(bj);
    clock.setTimeZone(bj);
    String today = dayKey.format(new java.util.Date());
    String yesterday = dayKey.format(new java.util.Date(System.currentTimeMillis() - 86_400_000L));
    String currentDay = "";
    LinearLayout group = null;
    TextView groupSum = null;
    long sum = 0;
    int shown = 0;
    for (int i = 0; i < items.length(); i++) {
      JSONObject j = items.optJSONObject(i);
      int coins = j.optInt("coins");
      if (filter.equals("in") && coins < 0 || filter.equals("out") && coins >= 0) continue;
      java.util.Date at;
      try {
        at = in.parse(j.optString("created_at"));
      } catch (ParseException e) {
        continue;
      }
      String key = dayKey.format(at);
      if (!key.equals(currentDay)) {
        if (groupSum != null) groupSum.setText(sumText(sum));
        currentDay = key;
        sum = 0;
        LinearLayout head = row();
        head.setPadding(dp(4), 0, dp(4), dp(8));
        String name = key.equals(today) ? "今天" : key.equals(yesterday) ? "昨天" : dayLabel.format(at);
        flex(head, text(name, 14, INK, true));
        groupSum = label("", 12, MUTED);
        head.addView(groupSum);
        if (shown > 0) gap(body, 12);
        add(body, head, -1, -2);
        group = col();
        group.setBackground(bg(WHITE, 18));
        group.setPadding(dp(14), dp(4), dp(14), dp(4));
        add(body, group, -1, -2);
      } else if (group != null) {
        View sep = new View(this);
        sep.setBackgroundColor(0xfff6ede4);
        LinearLayout.LayoutParams sp = lp(-1, 1);
        sp.leftMargin = dp(50);
        group.addView(sep, sp);
      }
      sum += coins;
      group.addView(ledgerRow(j, clock.format(at)));
      shown++;
    }
    if (groupSum != null) groupSum.setText(sumText(sum));
    if (shown == 0) {
      LinearLayout empty = col();
      empty.setGravity(Gravity.CENTER_HORIZONTAL);
      empty.setPadding(dp(20), dp(40), dp(20), dp(40));
      empty.setBackground(bg(WHITE, 18));
      add(empty, new CoinIcon(this), 48, 48);
      gap(empty, 12);
      add(empty, text(filter.equals("out") ? "还没有支出记录" : "还没有金币记录", 15, INK, true), -2, -2);
      gap(empty, 6);
      add(empty, label("拆红包、签到、看视频都能得金币", 13, MUTED), -2, -2);
      add(body, empty, -1, -2);
    } else {
      TextView foot = label("仅展示最近 200 条", 12, 0xffc4b2a6);
      foot.setGravity(Gravity.CENTER);
      foot.setPadding(0, dp(14), 0, 0);
      add(body, foot, -1, -2);
    }
  }

  String sumText(long sum) {
    return sum >= 0 ? "收入 " + number((int) sum) : "支出 " + number((int) -sum);
  }

  LinearLayout statCell(String name, int value) {
    LinearLayout c = col();
    add(c, text(number(value), 18, WHITE, true), -2, -2);
    gap(c, 3);
    add(c, label(name, 12, 0xccffffff), -2, -2);
    return c;
  }

  View ledgerRow(JSONObject j, String time) {
    String kind = j.optString("kind", "other");
    int coins = j.optInt("coins");
    String icon;
    int tint;
    switch (kind) {
      case "withdraw":
        icon = "clock";
        tint = 0xff8c7b70;
        break;
      case "refund":
        icon = "back";
        tint = 0xff2f8a4c;
        break;
      case "checkin":
        icon = "check";
        tint = 0xffff8a3d;
        break;
      case "rain":
      case "newbie":
        icon = "gift";
        tint = 0xffe8262b;
        break;
      case "wheel":
        icon = "wheel";
        tint = 0xff8e44ad;
        break;
      case "video":
        icon = "gift";
        tint = 0xfff65331;
        break;
      case "invite":
        icon = "me";
        tint = 0xff13a36f;
        break;
      default:
        icon = "coin";
        tint = 0xffe0a020;
    }
    LinearLayout r = row();
    r.setPadding(0, dp(12), 0, dp(12));
    FrameLayout disc = new FrameLayout(this);
    disc.setBackground(bg((tint & 0x00ffffff) | 0x1f000000, 18));
    disc.addView(new LineIcon(this, icon, tint), new FrameLayout.LayoutParams(dp(18), dp(18), Gravity.CENTER));
    add(r, disc, 36, 36);
    LinearLayout mid = col();
    mid.setPadding(dp(12), 0, dp(8), 0);
    TextView title = text(j.optString("title"), 15, INK, false);
    title.setSingleLine(true);
    title.setEllipsize(android.text.TextUtils.TruncateAt.END);
    add(mid, title, -1, -2);
    gap(mid, 4);
    add(mid, label(time, 12, 0xffbba596), -2, -2);
    flex(r, mid);
    TextView amount = text((coins >= 0 ? "+" : "") + number(coins), 17, coins >= 0 ? 0xffe8452b : INK, true);
    r.addView(amount);
    return r;
  }

  String withdrawalsReturn = "me";

  /** Full-page withdrawal history: summary card plus one card per request. */
  void withdrawals() {
    authBack = null;
    api.behavior.page(7);
    if (!current.equals("withdrawals")) withdrawalsReturn = current;
    current = "withdrawals";
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    scroll.setVerticalScrollBarEnabled(false);
    LinearLayout body = col();
    body.setPadding(dp(18), dp(4), dp(18), dp(28));
    scroll.addView(body);
    stage.removeAllViews();
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));

    LinearLayout top = row();
    LineIcon back = new LineIcon(this, "back", INK);
    back.setContentDescription("返回");
    back.setPadding(dp(10), dp(10), dp(10), dp(10));
    back.setOnClickListener(v -> closeWithdrawals());
    LinearLayout.LayoutParams bp = lp(44, 44);
    bp.leftMargin = -dp(10);
    top.addView(back, bp);
    flex(top, text("提现记录", 20, INK, true));
    add(body, top, -1, 56);
    gap(body, 6);

    LinearLayout list = col();
    TextView loadingText = label("正在加载…", 14, MUTED);
    loadingText.setGravity(Gravity.CENTER);
    loadingText.setPadding(0, dp(40), 0, dp(40));
    add(list, loadingText, -1, -2);
    LinearLayout summary = col();
    add(body, summary, -1, -2);
    add(body, list, -1, -2);

    api.get(
        "/withdrawals",
        d -> {
          if (!current.equals("withdrawals")) return;
          JSONArray items = (JSONArray) d;
          double done = 0, pending = 0;
          for (int i = 0; i < items.length(); i++) {
            JSONObject j = items.optJSONObject(i);
            String state = withdrawalState(j);
            if (state.equals("done")) done += j.optDouble("amount_yuan");
            else if (state.equals("pending")) pending += j.optDouble("amount_yuan");
          }
          summary.removeAllViews();
          withdrawalSummary(summary, done, pending, items.length());
          list.removeAllViews();
          if (items.length() == 0) {
            LinearLayout empty = col();
            empty.setGravity(Gravity.CENTER_HORIZONTAL);
            empty.setPadding(dp(20), dp(36), dp(20), dp(36));
            empty.setBackground(bg(WHITE, 18));
            LineIcon icon = new LineIcon(this, "clock", 0xffd9c2b2);
            add(empty, icon, 44, 44);
            gap(empty, 14);
            add(empty, text("还没有提现记录", 16, INK, true), -2, -2);
            gap(empty, 6);
            add(empty, label("攒够金币就能提现到支付宝", 13, MUTED), -2, -2);
            add(list, empty, -1, -2);
            return;
          }
          add(list, text("全部记录", 14, MUTED, false), -2, -2);
          gap(list, 10);
          for (int i = 0; i < items.length(); i++) {
            add(list, withdrawalCard(items.optJSONObject(i)), -1, -2);
            gap(list, 10);
          }
          TextView foot = label("仅展示最近 200 条记录", 12, 0xffc4b2a6);
          foot.setGravity(Gravity.CENTER);
          foot.setPadding(0, dp(6), 0, 0);
          add(list, foot, -1, -2);
        },
        m -> {
          if (!current.equals("withdrawals")) return;
          list.removeAllViews();
          TextView failed = label("加载失败，点此重试", 14, RED);
          failed.setGravity(Gravity.CENTER);
          failed.setPadding(0, dp(40), 0, dp(40));
          failed.setOnClickListener(v -> withdrawals());
          add(list, failed, -1, -2);
        });
  }

  void closeWithdrawals() {
    current = withdrawalsReturn.isEmpty() || withdrawalsReturn.equals("withdrawals") ? "me" : withdrawalsReturn;
    home();
  }

  /** Server sends state (pending/done/returned); older servers only send status text. */
  String withdrawalState(JSONObject j) {
    String state = j.optString("state");
    if (!state.isEmpty()) return state;
    String raw = j.optString("status");
    if (raw.endsWith("已完成") || raw.equals("approved") || raw.equals("已到账")) return "done";
    if (raw.endsWith("已退回") || raw.equals("rejected")) return "returned";
    return "pending";
  }

  void withdrawalSummary(LinearLayout parent, double done, double pending, int count) {
    LinearLayout card = col();
    card.setBackground(grad(0xffff8a4c, RED, 22));
    card.setPadding(dp(20), dp(18), dp(20), dp(18));
    add(card, label("累计到账（元）", 13, 0xccffffff), -2, -2);
    gap(card, 8);
    add(card, text(money(done), 32, WHITE, true), -2, -2);
    gap(card, 14);
    LinearLayout stats = row();
    flex(stats, label("审核中 ¥" + money(pending) + "   ·   共 " + count + " 笔", 13, 0xe6ffffff));
    TextView go = text("去提现", 13, RED, true);
    go.setGravity(Gravity.CENTER);
    go.setPadding(dp(14), dp(7), dp(14), dp(7));
    go.setBackground(bg(WHITE, 16));
    go.setOnClickListener(
        v -> {
          press(v);
          withdraw();
        });
    stats.addView(go);
    add(card, stats, -1, -2);
    add(parent, card, -1, -2);
    gap(parent, 18);
  }

  View withdrawalCard(JSONObject j) {
    String state = withdrawalState(j);
    int tint = state.equals("done") ? 0xff2f8a4c : state.equals("returned") ? 0xff9a8a80 : 0xffe8742f;
    int chip = state.equals("done") ? 0xffeaf7ee : state.equals("returned") ? 0xfff2eeeb : 0xfffff1e6;
    String statusText =
        j.has("state") ? j.optString("status") : state.equals("done") ? "已到账" : state.equals("returned") ? "已退回" : "审核中";

    LinearLayout card = col();
    card.setBackground(bg(WHITE, 18));
    card.setPadding(dp(16), dp(14), dp(16), dp(14));

    LinearLayout head = row();
    TextView badge = text("支", 15, WHITE, true);
    badge.setGravity(Gravity.CENTER);
    badge.setBackground(bg(0xff1677ff, 12));
    head.addView(badge, lp(38, 38));
    LinearLayout mid = col();
    mid.setPadding(dp(12), 0, dp(8), 0);
    String channel = j.optString("channel_text", "支付宝");
    add(mid, text("提现到" + (channel.isEmpty() ? "支付宝" : channel), 15, INK, true), -2, -2);
    gap(mid, 5);
    String account = j.optString("account");
    add(mid, label((account.isEmpty() ? "" : account + "  ·  ") + date(j.optString("created_at")), 12, MUTED), -2, -2);
    flex(head, mid);
    LinearLayout right = col();
    right.setGravity(Gravity.END);
    add(right, text("¥" + money(j.optDouble("amount_yuan")), 18, INK, true), -2, -2);
    gap(right, 6);
    TextView status = text(statusText, 12, tint, true);
    status.setPadding(dp(9), dp(3), dp(9), dp(3));
    status.setBackground(bg(chip, 10));
    add(right, status, -2, -2);
    head.addView(right);
    add(card, head, -1, -2);

    String note = j.optString("note");
    int coins = j.optInt("coins");
    String id = j.optString("id");
    if (!note.isEmpty() || coins > 0 || !id.isEmpty()) {
      gap(card, 12);
      View divider = new View(this);
      divider.setBackgroundColor(0xfff5ece4);
      add(card, divider, -1, 1);
      gap(card, 10);
      LinearLayout foot = row();
      flex(foot, label(note.isEmpty() ? (coins > 0 ? "消耗 " + coins + " 金币" : "") : note, 12, state.equals("returned") ? 0xff9a8a80 : MUTED));
      if (!id.isEmpty()) foot.addView(label("单号 " + id.substring(0, Math.min(8, id.length())), 11, 0xffc4b2a6));
      add(card, foot, -1, -2);
    }
    return card;
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
    withLocation(
        () -> api.get("/withdraw/options", value -> showWithdraw((JSONObject) value), this::error));
  }

  // ---- 提现 / 收款账户 (WeChat-wallet look: grey page, white cards, green actions) ----
  static final int WX_GREEN = 0xff07c160, WX_BG = 0xffededed, ALIPAY_BLUE = 0xff1677ff;
  static final int PICK_QR = 4110;
  String subReturn = "me";
  JSONObject payoutAccount;
  Runnable afterPayoutBound;

  /** Full page with a back arrow and a title; returns the scrollable body. */
  LinearLayout subPage(String key, String title, int background) {
    authBack = null;
    if (!current.equals("withdraw") && !current.equals("payout") && !current.equals("withdraw_done") && !current.equals("ledger")) subReturn = current;
    current = key;
    ScrollView scroll = new ScrollView(this);
    scroll.setFillViewport(true);
    scroll.setVerticalScrollBarEnabled(false);
    scroll.setBackgroundColor(background);
    LinearLayout body = col();
    body.setPadding(dp(14), dp(4), dp(14), dp(28));
    scroll.addView(body);
    stage.removeAllViews();
    stage.addView(scroll, new FrameLayout.LayoutParams(-1, -1));
    LinearLayout top = row();
    LineIcon back = new LineIcon(this, "back", INK);
    back.setContentDescription("返回");
    back.setPadding(dp(10), dp(10), dp(10), dp(10));
    back.setOnClickListener(v -> closeSubPage());
    LinearLayout.LayoutParams bp = lp(44, 44);
    bp.leftMargin = -dp(6);
    top.addView(back, bp);
    flex(top, text(title, 18, INK, true));
    add(body, top, -1, 52);
    gap(body, 6);
    return body;
  }

  void closeSubPage() {
    if (current.equals("payout") && afterPayoutBound != null) {
      afterPayoutBound = null;
      withdraw();
      return;
    }
    current = subReturn.isEmpty() ? "me" : subReturn;
    home();
  }

  View brandIcon(boolean wechat, int size) {
    TextView t = text(wechat ? "微" : "支", size / 2, WHITE, true);
    t.setGravity(Gravity.CENTER);
    t.setBackground(bg(wechat ? WX_GREEN : ALIPAY_BLUE, size / 4));
    return t;
  }

  LinearLayout whiteCard(LinearLayout parent) {
    LinearLayout card = col();
    card.setBackground(bg(WHITE, 12));
    card.setPadding(dp(16), dp(14), dp(16), dp(14));
    add(parent, card, -1, -2);
    gap(parent, 12);
    return card;
  }

  TextView greenButton(String s, Runnable run) {
    TextView b = text(s, 17, WHITE, true);
    b.setGravity(Gravity.CENTER);
    b.setBackground(bg(WX_GREEN, 8));
    b.setOnClickListener(v -> {
      press(v);
      run.run();
    });
    return b;
  }

  // ---------------- 收款账户 ----------------
  void payoutAccounts() {
    api.get("/payout-account", v -> {
      payoutAccount = (JSONObject) v;
      payoutPage();
    }, this::error);
  }

  void payoutPage() {
    LinearLayout body = subPage("payout", "收款账户", WX_BG);
    add(body, label("绑定一次，以后提现直接选择，无需每次填写。", 12, 0xff888888), -1, -2);
    gap(body, 10);
    JSONObject wx = payoutAccount == null ? null : payoutAccount.optJSONObject("wechat");
    JSONObject ali = payoutAccount == null ? null : payoutAccount.optJSONObject("alipay");

    // 微信收款码
    LinearLayout wcard = whiteCard(body);
    LinearLayout wh = row();
    add(wh, brandIcon(true, 36), 36, 36);
    LinearLayout wt = col();
    wt.setPadding(dp(12), 0, 0, 0);
    add(wt, text("微信收款码", 16, INK, true), -2, -2);
    gap(wt, 4);
    add(wt, label(wx == null ? "选择微信“收款码”图片，提现打到你的微信" : "已绑定" + (wx.optString("name").isEmpty() ? "" : " · " + wx.optString("name")), 12, wx == null ? 0xff888888 : WX_GREEN), -2, -2);
    flex(wh, wt);
    add(wcard, wh, -1, -2);
    gap(wcard, 12);
    EditText wname = input("真实姓名（选填，便于核对）", InputType.TYPE_CLASS_TEXT);
    wname.setBackground(bg(0xfff6f6f6, 8));
    add(wcard, wname, -1, 46);
    gap(wcard, 8);
    add(wcard, label("在微信「我 → 服务 → 收付款 → 二维码收款」保存收款码图片后选择。图片只在手机上识别，仅上传收款码内容。", 11, 0xffaaaaaa), -1, -2);
    gap(wcard, 12);
    add(wcard, greenButton(wx == null ? "选择收款码图片" : "更换收款码", () -> {
      pendingWechatName = wname.getText().toString().trim();
      Intent pick = new Intent(Intent.ACTION_GET_CONTENT).setType("image/*").addCategory(Intent.CATEGORY_OPENABLE);
      try {
        startActivityForResult(Intent.createChooser(pick, "选择收款码图片"), PICK_QR);
      } catch (ActivityNotFoundException e) {
        tip("没有可用的相册应用");
      }
    }), -1, 46);

    // 支付宝
    LinearLayout acard = whiteCard(body);
    LinearLayout ah = row();
    add(ah, brandIcon(false, 36), 36, 36);
    LinearLayout at = col();
    at.setPadding(dp(12), 0, 0, 0);
    add(at, text("支付宝", 16, INK, true), -2, -2);
    gap(at, 4);
    add(at, label(ali == null ? "填写支付宝账号和实名认证的姓名" : "已绑定 " + ali.optString("account") + " · " + ali.optString("name"), 12, ali == null ? 0xff888888 : ALIPAY_BLUE), -2, -2);
    flex(ah, at);
    add(acard, ah, -1, -2);
    gap(acard, 12);
    EditText account = input("支付宝账号（手机号或邮箱）", InputType.TYPE_CLASS_TEXT);
    account.setBackground(bg(0xfff6f6f6, 8));
    add(acard, account, -1, 46);
    gap(acard, 8);
    EditText name = input("真实姓名（须与支付宝实名一致）", InputType.TYPE_CLASS_TEXT);
    name.setBackground(bg(0xfff6f6f6, 8));
    add(acard, name, -1, 46);
    gap(acard, 12);
    TextView save = text(ali == null ? "绑定支付宝" : "修改支付宝", 16, WHITE, true);
    save.setGravity(Gravity.CENTER);
    save.setBackground(bg(ALIPAY_BLUE, 8));
    save.setOnClickListener(v -> {
      press(v);
      String a = account.getText().toString().trim(), n = name.getText().toString().trim();
      if (a.isEmpty() || n.isEmpty()) {
        tip("请填写支付宝账号和真实姓名");
        return;
      }
      save.setEnabled(false);
      api.request("PUT", "/payout-account/alipay", Api.object("account", a, "real_name", n), d -> {
        payoutAccount = (JSONObject) d;
        tip("支付宝已绑定");
        afterBind();
      }, m -> {
        save.setEnabled(true);
        error(m);
      });
    });
    add(acard, save, -1, 46);
    add(body, label("收款信息仅用于提现打款，提交后的提现申请不受之后修改影响。", 11, 0xffaaaaaa), -1, -2);
  }

  String pendingWechatName = "";

  /** Decodes the 收款码 on the phone; only its content (wxp://…) is uploaded, never the image. */
  void uploadWechatQr(Uri uri) {
    showBusy("正在识别收款码…");
    new Thread(() -> {
      String code;
      try {
        code = decodeQr(uri);
      } catch (IOException | RuntimeException e) {
        code = null;
      }
      String found = code;
      runOnUiThread(() -> {
        if (found == null) {
          hideBusy();
          error("没有识别到二维码，请选择清晰的微信收款码图片");
          return;
        }
        if (!found.matches("wxp://[A-Za-z0-9_-]{12,120}")) {
          hideBusy();
          error("这不是微信收款码。请在微信“收付款 → 二维码收款”中保存收款码后重新选择");
          return;
        }
        showBusy("正在绑定…");
        api.post("/payout-account/wechat", Api.object("code", found, "real_name", pendingWechatName), d -> {
          hideBusy();
          payoutAccount = (JSONObject) d;
          tip("微信收款码已绑定");
          afterBind();
        }, m -> {
          hideBusy();
          error(m);
        });
      });
    }, "payout-qr").start();
  }

  String decodeQr(Uri uri) throws IOException {
    BitmapFactory.Options bounds = new BitmapFactory.Options();
    bounds.inJustDecodeBounds = true;
    try (java.io.InputStream in = getContentResolver().openInputStream(uri)) {
      BitmapFactory.decodeStream(in, null, bounds);
    }
    int sample = 1;
    while (Math.max(bounds.outWidth, bounds.outHeight) / sample > 2400) sample *= 2;
    BitmapFactory.Options opts = new BitmapFactory.Options();
    opts.inSampleSize = sample;
    Bitmap bmp;
    try (java.io.InputStream in = getContentResolver().openInputStream(uri)) {
      bmp = BitmapFactory.decodeStream(in, null, opts);
    }
    if (bmp == null) return null;
    java.util.Map<com.google.zxing.DecodeHintType, Object> hints = new java.util.EnumMap<>(com.google.zxing.DecodeHintType.class);
    hints.put(com.google.zxing.DecodeHintType.TRY_HARDER, Boolean.TRUE);
    hints.put(com.google.zxing.DecodeHintType.CHARACTER_SET, "UTF-8");
    // Full image first, then smaller copies (very large screenshots can defeat the detector).
    for (int edge : new int[] {0, 1200, 800}) {
      Bitmap b = bmp;
      if (edge > 0 && Math.max(bmp.getWidth(), bmp.getHeight()) > edge) {
        float k = (float) edge / Math.max(bmp.getWidth(), bmp.getHeight());
        b = Bitmap.createScaledBitmap(bmp, Math.round(bmp.getWidth() * k), Math.round(bmp.getHeight() * k), true);
      }
      int w = b.getWidth(), h = b.getHeight();
      int[] px = new int[w * h];
      b.getPixels(px, 0, w, 0, 0, w, h);
      com.google.zxing.LuminanceSource src = new com.google.zxing.RGBLuminanceSource(w, h, px);
      for (com.google.zxing.Binarizer bin : new com.google.zxing.Binarizer[] {new com.google.zxing.common.HybridBinarizer(src), new com.google.zxing.common.GlobalHistogramBinarizer(src)}) {
        try {
          return new com.google.zxing.qrcode.QRCodeReader().decode(new com.google.zxing.BinaryBitmap(bin), hints).getText().trim();
        } catch (com.google.zxing.ReaderException ignored) {
        }
      }
    }
    return null;
  }

  void afterBind() {
    Runnable next = afterPayoutBound;
    afterPayoutBound = null;
    if (next != null) next.run();
    else payoutPage();
  }

  // ---------------- 提现 ----------------
  void showWithdraw(JSONObject options) {
    api.get("/payout-account", v -> {
      payoutAccount = (JSONObject) v;
      withdrawPage(options);
    }, this::error);
  }

  void withdrawPage(JSONObject options) {
    LinearLayout body = subPage("withdraw", "提现", WX_BG);
    JSONObject wx = payoutAccount == null ? null : payoutAccount.optJSONObject("wechat");
    JSONObject ali = payoutAccount == null ? null : payoutAccount.optJSONObject("alipay");
    final String[] channel = {wx != null ? "wechat" : ali != null ? "alipay" : ""};

    // 提现到
    LinearLayout to = whiteCard(body);
    add(to, label("提现到", 13, 0xff888888), -1, -2);
    gap(to, 10);
    LinearLayout methods = col();
    add(to, methods, -1, -2);
    Runnable[] paintMethods = new Runnable[1];
    paintMethods[0] = () -> {
      methods.removeAllViews();
      for (int i = 0; i < 2; i++) {
        boolean wechat = i == 0;
        JSONObject acc = wechat ? wx : ali;
        LinearLayout r = row();
        r.setPadding(0, dp(8), 0, dp(8));
        add(r, brandIcon(wechat, 32), 32, 32);
        LinearLayout t = col();
        t.setPadding(dp(12), 0, 0, 0);
        add(t, text(wechat ? "微信" : "支付宝", 15, INK, true), -2, -2);
        gap(t, 3);
        String sub = acc == null ? "未绑定，点击去绑定" : wechat ? "收款码" + (acc.optString("name").isEmpty() ? "" : " · " + acc.optString("name")) : acc.optString("account") + " · " + acc.optString("name");
        add(t, label(sub, 12, acc == null ? 0xffff7a45 : 0xff888888), -2, -2);
        flex(r, t);
        String key = wechat ? "wechat" : "alipay";
        boolean on = key.equals(channel[0]);
        TextView radio = text(on ? "✓" : "", 13, WHITE, true);
        radio.setGravity(Gravity.CENTER);
        GradientDrawable rd = bg(on ? WX_GREEN : WHITE, 11);
        if (!on) rd.setStroke(dp(1.5f), 0xffcccccc);
        radio.setBackground(rd);
        add(r, radio, 22, 22);
        r.setOnClickListener(v -> {
          if (acc == null) {
            afterPayoutBound = () -> withdraw();
            payoutAccounts();
            return;
          }
          channel[0] = key;
          paintMethods[0].run();
        });
        add(methods, r, -1, -2);
        if (i == 0) line(methods);
      }
    };
    paintMethods[0].run();

    // 金额
    LinearLayout amountCard = whiteCard(body);
    add(amountCard, label("提现金额", 13, 0xff888888), -1, -2);
    gap(amountCard, 6);
    LinearLayout bal = row();
    add(bal, text("¥", 22, INK, true), -2, -2);
    TextView cash = text(" " + money(options.optDouble("cash_yuan")), 34, INK, true);
    bal.addView(cash);
    add(amountCard, bal, -1, -2);
    gap(amountCard, 2);
    add(amountCard, label("当前可提现余额", 12, 0xffaaaaaa), -1, -2);
    gap(amountCard, 14);
    JSONArray tiers = options.optJSONArray("tiers");
    final JSONObject[] selected = {null};
    final ArrayList<View> chips = new ArrayList<>();
    TextView submit = greenButton("提现", () -> {});
    Runnable paint = () -> {
      for (View chip : chips) {
        JSONObject tier = (JSONObject) chip.getTag();
        boolean on = tier == selected[0];
        GradientDrawable d = bg(on ? 0xffe8f8ef : 0xfff7f7f7, 8);
        d.setStroke(dp(on ? 1.5f : 1), on ? WX_GREEN : 0xffeeeeee);
        chip.setBackground(d);
      }
      boolean ready = selected[0] != null && !channel[0].isEmpty();
      submit.setAlpha(ready ? 1f : .45f);
      submit.setText(selected[0] == null ? "提现" : "提现 " + selected[0].optString("label"));
    };
    LinearLayout r = null;
    int count = tiers == null ? 0 : tiers.length();
    for (int i = 0; i < count; i++) {
      JSONObject tier = tiers.optJSONObject(i);
      if (i % 3 == 0) {
        if (r != null) gap(amountCard, 10);
        r = row();
        add(amountCard, r, -1, -2);
      }
      boolean available = tier.optBoolean("available");
      LinearLayout chip = col();
      chip.setGravity(Gravity.CENTER);
      chip.setPadding(dp(4), dp(12), dp(4), dp(10));
      chip.setTag(tier);
      TextView amount = text(tier.optString("label"), 17, available ? INK : 0xffbbbbbb, true);
      amount.setGravity(Gravity.CENTER);
      add(chip, amount, -1, -2);
      gap(chip, 3);
      TextView note = label(available ? tier.optString("limit") : tier.optString("reason"), 10, available ? 0xffaaaaaa : 0xffff7a45);
      note.setGravity(Gravity.CENTER);
      note.setSingleLine(true);
      add(chip, note, -1, -2);
      chip.setOnClickListener(v -> {
        if (!available) {
          tip(tier.optString("reason") + " · " + tier.optString("limit"));
          return;
        }
        selected[0] = tier;
        paint.run();
      });
      chips.add(chip);
      LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, -2, 1);
      if (i % 3 != 0) lp.leftMargin = dp(10);
      r.addView(chip, lp);
      if (selected[0] == null && available) selected[0] = tier;
    }
    if (r != null) for (int i = count % 3 == 0 ? 3 : count % 3; i < 3; i++) {
      LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, 1, 1);
      lp.leftMargin = dp(10);
      r.addView(new View(this), lp);
    }
    if (count == 0) add(amountCard, label("提现暂未开放", 13, 0xff888888), -1, -2);
    gap(body, 10);
    add(body, submit, -1, 50);
    gap(body, 12);
    TextView manage = label("管理收款账户", 13, 0xff576b95);
    manage.setGravity(Gravity.CENTER);
    manage.setOnClickListener(v -> {
      afterPayoutBound = () -> withdraw();
      payoutAccounts();
    });
    add(body, manage, -1, 36);
    add(body, label("提现申请提交后需人工审核，预计 1–3 个工作日到账。", 11, 0xffaaaaaa), -1, -2);
    paint.run();
    submit.setOnClickListener(v -> {
      if (channel[0].isEmpty()) {
        tip("请先绑定收款账户");
        afterPayoutBound = () -> withdraw();
        payoutAccounts();
        return;
      }
      if (selected[0] == null) {
        tip(count == 0 ? "提现暂未开放" : "余额不足或次数已用完");
        return;
      }
      press(v);
      submit.setEnabled(false);
      showBusy("正在提交…");
      JSONObject tier = selected[0];
      String ch = channel[0];
      api.post("/withdrawals", Api.object("amount_yuan", tier.optDouble("amount_yuan"), "channel", ch), d -> {
        hideBusy();
        JSONObject result = (JSONObject) d;
        if (result.has("user")) user = result.optJSONObject("user");
        refreshUser();
        withdrawDone(result, ch);
      }, m -> {
        hideBusy();
        submit.setEnabled(true);
        error(m);
      });
    });
  }

  /** WeChat-Pay-style result: green tick, amount, details, 完成. */
  void withdrawDone(JSONObject result, String channel) {
    LinearLayout body = subPage("withdraw_done", "", WHITE);
    body.setGravity(Gravity.CENTER_HORIZONTAL);
    gap(body, 36);
    CheckMark mark = new CheckMark(this);
    add(body, mark, 72, 72);
    mark.post(mark::play);
    gap(body, 18);
    TextView title = text("提现申请已提交", 18, INK, true);
    add(body, title, -2, -2);
    gap(body, 16);
    LinearLayout amt = row();
    amt.setGravity(Gravity.CENTER);
    add(amt, text("¥", 24, INK, true), -2, -2);
    amt.addView(text(" " + money(result.optDouble("amount_yuan")), 40, INK, true));
    add(body, amt, -1, -2);
    gap(body, 30);
    LinearLayout details = col();
    details.setPadding(dp(10), 0, dp(10), 0);
    String[][] rows = {
      {"提现方式", channel.equals("wechat") ? "微信" : "支付宝"},
      {"收款账户", result.optString("account")},
      {"申请时间", date(result.optString("created_at"))},
      {"预计到账", "1–3 个工作日"},
      {"当前状态", "审核中"},
    };
    for (String[] rr : rows) {
      LinearLayout line = row();
      line.setPadding(0, dp(9), 0, dp(9));
      flex(line, label(rr[0], 14, 0xff888888));
      TextView v = label(rr[1], 14, INK);
      v.setGravity(Gravity.END);
      line.addView(v);
      add(details, line, -1, -2);
    }
    add(body, details, -1, -2);
    gap(body, 40);
    TextView done = text("完成", 17, WX_GREEN, true);
    done.setGravity(Gravity.CENTER);
    done.setBackground(bg(0xfff2f2f2, 8));
    done.setOnClickListener(v -> {
      press(v);
      current = subReturn.isEmpty() ? "me" : subReturn;
      home();
    });
    LinearLayout.LayoutParams dlp = new LinearLayout.LayoutParams(dp(184), dp(48));
    body.addView(done, dlp);
    gap(body, 14);
    TextView records = label("查看提现记录", 13, 0xff576b95);
    records.setGravity(Gravity.CENTER);
    records.setOnClickListener(v -> withdrawals());
    add(body, records, -2, 32);
    playCoinArrival();
  }

  @Override
  public boolean dispatchTouchEvent(MotionEvent event) {
    TouchAudit.record(event);
    if (api != null) api.behavior.touch(event);
    return super.dispatchTouchEvent(event);
  }

  @Override
  public void onBackPressed() {
    if (updates.required()) {
      finish();
      return;
    }
    if (authBack != null) {
      authBack.run();
    } else if (deletionConfirmPage) {
      closeDeletionConfirm();
    } else if (deletionPage) {
      enterAccountHome();
    } else if (current.equals("settings")) {
      closeSettings();
    } else if (current.equals("withdrawals")) {
      closeWithdrawals();
    } else if (current.equals("withdraw") || current.equals("payout") || current.equals("withdraw_done") || current.equals("ledger")) {
      closeSubPage();
    } else if (welcomeShowing) {
      finish();
    } else if (user != null && user.optBoolean("invitation_required")) {
      exitAccount();
    } else if (user != null) {
      if (!current.equals("hall")) {
        current = "hall";
        home();
      } else super.onBackPressed();
    } else super.onBackPressed();
  }

  @Override
  protected void onNewIntent(Intent intent) {
    super.onNewIntent(intent);
    setIntent(intent);
    // InviteLinkActivity brought us forward with a fresh invite code.
    if (user != null) loadNewbie();
  }

  @Override
  protected void onResume() {
    super.onResume();
    foreground = true;
    api.behavior.resume();
    nativeFeeds.resume();
    if (current.equals("hall") && hallSurface != null && rainTime != null)
      rain.attach(hallSurface, rainTime);
    ensureHallStream();
    if (pendingReward != null) {
      PendingReward pending = pendingReward;
      showRewardPending(pending.session, pending.notice, pending.rainPacket, pending.requestError);
    }
    refreshRuntime();
    updates.resume();
    if (servicesReady) api.deviceTrust.refreshIfStale();
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
    hideBusy();
    ads.flush();
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
    if (requestCode == PICK_QR) {
      if (resultCode == RESULT_OK && data != null && data.getData() != null) uploadWechatQr(data.getData());
      return;
    }
    if (requestCode == WebActivity.REQUEST) {
      loadNewbie();
      loadWheel();
      String target = data == null ? null : data.getStringExtra(WebActivity.RESULT_OPEN);
      if (resultCode == RESULT_OK && target != null) handler.post(() -> openNative(target));
      return;
    }
    updates.activityResult(requestCode, resultCode);
  }

  @Override
  public void onWindowFocusChanged(boolean focused) {
    super.onWindowFocusChanged(focused);
    if (focused && updates != null) {
      updates.maybePresent();
      handler.postDelayed(this::maybePromptCheckin, 400);
    }
  }

  // Edge-to-edge (Android 15): the system bars show the stage background, so tint that during rain.
  void rainChrome(boolean raining) {
    stage.setBackgroundColor(raining ? 0xffb3261b : CREAM);
    getWindow()
        .getDecorView()
        .setSystemUiVisibility(raining ? 0 : View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR | View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
  }

  boolean canPresentUpdate() {
    return servicesReady
        && foreground
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
    loadNewbie();
    loadWheel();
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
