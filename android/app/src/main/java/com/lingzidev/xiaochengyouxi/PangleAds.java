package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import com.bytedance.sdk.openadsdk.*;
import java.util.HashMap;
import java.util.Map;
import org.json.JSONObject;

final class PangleAds implements RewardedAdProvider {
  private CSJSplashAd splash;
  private TTRewardVideoAd activeReward;

  void initialize(Activity activity, Runnable ready, Api.Failure failure) {
    if (TTAdSdk.isSdkReady()) {
      ready.run();
      return;
    }
    TTAdSdk.init(
        activity.getApplicationContext(),
        new TTAdConfig.Builder()
            .appId(BuildConfig.CSJ_APP_ID)
            .appName(activity.getString(R.string.app_name))
            .debug(false)
            .useMediation(false)
            .supportMultiProcess(false)
            .allowShowNotify(false)
            .data("[{\"name\":\"personal_ads_type\",\"value\":\"0\"}]")
            .customController(
                new TTCustomController() {
                  @Override
                  public boolean isCanUseLocation() {
                    return false;
                  }

                  @Override
                  public boolean alist() {
                    return false;
                  }

                  @Override
                  public boolean isCanUsePhoneState() {
                    return false;
                  }

                  @Override
                  public boolean isCanUseWifiState() {
                    return false;
                  }

                  @Override
                  public boolean isCanUseWriteExternal() {
                    return false;
                  }

                  @Override
                  public boolean isCanUseAndroidId() {
                    return false;
                  }

                  @Override
                  public boolean isCanUsePermissionRecordAudio() {
                    return false;
                  }

                  @Override
                  public boolean isCanUseMessage() {
                    return false;
                  }

                  @Override
                  public Map<String, Object> userPrivacyConfig() {
                    Map<String, Object> config = new HashMap<>();
                    config.put("motion_info", "0");
                    return config;
                  }
                })
            .build());
    TTAdSdk.start(
        new TTAdSdk.Callback() {
          @Override
          public void success() {
            ui(activity, ready);
          }

          @Override
          public void fail(int code, String message) {
            ui(activity, () -> failure.failed(error("初始化", code, message)));
          }
        });
  }

  void showSplash(Activity activity, FrameLayout host, Runnable closed, Api.Failure failure) {
    host.post(
        () -> {
          float density = activity.getResources().getDisplayMetrics().density;
          AdSlot slot =
              new AdSlot.Builder()
                  .setCodeId(BuildConfig.CSJ_SPLASH_SLOT)
                  .setExpressViewAcceptedSize(host.getWidth() / density, host.getHeight() / density)
                  .setAdLoadType(TTAdLoadType.LOAD)
                  .build();
          TTAdSdk.getAdManager()
              .createAdNative(activity)
              .loadSplashAd(
                  slot,
                  new TTAdNative.CSJSplashAdListener() {
                    @Override
                    public void onSplashLoadSuccess(CSJSplashAd ad) {}

                    @Override
                    public void onSplashLoadFail(CSJAdError error) {
                      ui(
                          activity,
                          () -> failure.failed(error("开屏加载", error.getCode(), error.getMsg())));
                    }

                    @Override
                    public void onSplashRenderSuccess(CSJSplashAd ad) {
                      ui(
                          activity,
                          () -> {
                            splash = ad;
                            ad.setSplashAdListener(
                                new CSJSplashAd.SplashAdListener() {
                                  @Override
                                  public void onSplashAdShow(CSJSplashAd value) {}

                                  @Override
                                  public void onSplashAdClick(CSJSplashAd value) {}

                                  @Override
                                  public void onSplashAdClose(CSJSplashAd value, int type) {
                                    ui(
                                        activity,
                                        () -> {
                                          value.setSplashAdListener(null);
                                          host.removeAllViews();
                                          splash = null;
                                          closed.run();
                                        });
                                  }
                                });
                            ad.showSplashView(host);
                          });
                    }

                    @Override
                    public void onSplashRenderFail(CSJSplashAd ad, CSJAdError error) {
                      ui(
                          activity,
                          () -> failure.failed(error("开屏渲染", error.getCode(), error.getMsg())));
                    }
                  },
                  5000);
        });
  }

  @Override
  public void show(
      Activity activity,
      JSONObject session,
      Closed closed,
      Api.Result clientEvent,
      Api.Failure failure) {
    if (!TTAdSdk.isSdkReady()) {
      failure.failed("穿山甲 SDK 尚未初始化成功");
      return;
    }
    AdSlot slot =
        new AdSlot.Builder()
            .setCodeId(session.optString("ad_code_id"))
            .setOrientation(TTAdConstant.VERTICAL)
            .setAdLoadType(TTAdLoadType.LOAD)
            .setAdCount(1)
            .setRewardName("金币")
            .setRewardAmount(session.optInt("reward"))
            .setUserID(session.optString("user_id"))
            .setMediaExtra(session.optString("media_extra"))
            .build();
    TTAdSdk.getAdManager()
        .createAdNative(activity)
        .loadRewardVideoAd(
            slot,
            new TTAdNative.RewardVideoAdListener() {
              TTRewardVideoAd loaded;
              boolean shown;

              @Override
              public void onError(int code, String message) {
                ui(activity, () -> failure.failed(error("激励视频加载", code, message)));
              }

              @Override
              public void onRewardVideoAdLoad(TTRewardVideoAd ad) {
                loaded = ad;
              }

              @Override
              public void onRewardVideoCached() {
                if (loaded != null) showLoaded(loaded);
              }

              @Override
              public void onRewardVideoCached(TTRewardVideoAd ad) {
                showLoaded(ad);
              }

              void showLoaded(TTRewardVideoAd ad) {
                ui(
                    activity,
                    () -> {
                      if (shown) return;
                      shown = true;
                      activeReward = ad;
                      ad.setRewardAdInteractionListener(
                          new TTRewardVideoAd.RewardAdInteractionListener() {
                            final Notice notice = new Notice();
                            boolean finished;

                            @Override
                            public void onAdShow() {}

                            @Override
                            public void onAdVideoBarClick() {}

                            @Override
                            public void onAdClose() {
                              ui(
                                  activity,
                                  () -> {
                                    if (finished) return;
                                    finished = true;
                                    clientEvent.done(Api.object("event", "ad_closed"));
                                    ad.setRewardAdInteractionListener(null);
                                    activeReward = null;
                                    closed.done(notice);
                                  });
                            }

                            @Override
                            public void onVideoComplete() {
                              ui(activity, () -> notice.complete = true);
                            }

                            @Override
                            public void onVideoError() {
                              ui(
                                  activity,
                                  () -> {
                                    notice.playbackError = true;
                                    clientEvent.done(
                                        Api.object(
                                            "event", "video_error", "error_message", "视频播放失败"));
                                  });
                            }

                            @Override
                            public void onSkippedVideo() {
                              ui(activity, () -> notice.skipped = true);
                            }

                            @Override
                            public void onRewardVerify(
                                boolean isValid,
                                int amount,
                                String name,
                                int code,
                                String message) {
                              ui(
                                  activity,
                                  () ->
                                      clientEvent.done(
                                          rewardEvent(
                                              "reward_verify",
                                              isValid,
                                              null,
                                              null,
                                              code,
                                              message)));
                            }

                            @Override
                            public void onRewardArrived(boolean isValid, int type, Bundle extra) {
                              ui(
                                  activity,
                                  () -> {
                                    Integer code =
                                        extra != null
                                                && extra.containsKey(
                                                    TTRewardVideoAd.REWARD_EXTRA_KEY_ERROR_CODE)
                                            ? extra.getInt(
                                                TTRewardVideoAd.REWARD_EXTRA_KEY_ERROR_CODE)
                                            : null;
                                    String message =
                                        extra == null
                                            ? null
                                            : extra.getString(
                                                TTRewardVideoAd.REWARD_EXTRA_KEY_ERROR_MSG);
                                    Boolean serverVerify =
                                        extra != null
                                                && extra.containsKey(
                                                    TTRewardVideoAd
                                                        .REWARD_EXTRA_KEY_IS_SERVER_VERIFY)
                                            ? extra.getBoolean(
                                                TTRewardVideoAd.REWARD_EXTRA_KEY_IS_SERVER_VERIFY)
                                            : null;
                                    notice.arrived(isValid, code, message);
                                    clientEvent.done(
                                        rewardEvent(
                                            "reward_arrived",
                                            isValid,
                                            type,
                                            serverVerify,
                                            code,
                                            message));
                                  });
                            }
                          });
                      ad.showRewardVideoAd(activity);
                    });
              }
            });
  }

  void dispose() {
    if (splash != null) {
      splash.setSplashAdListener(null);
      View view = splash.getSplashView();
      if (view != null && view.getParent() instanceof ViewGroup)
        ((ViewGroup) view.getParent()).removeView(view);
      splash = null;
    }
    if (activeReward != null) {
      activeReward.setRewardAdInteractionListener(null);
      activeReward = null;
    }
  }

  private static String error(String stage, int code, String message) {
    return "穿山甲"
        + stage
        + "（"
        + code
        + "）"
        + (message == null || message.isEmpty() ? "" : "：" + message);
  }

  private static JSONObject rewardEvent(
      String event,
      boolean valid,
      Integer rewardType,
      Boolean serverVerify,
      Integer errorCode,
      String errorMessage) {
    JSONObject value = Api.object("event", event, "valid", valid);
    try {
      if (rewardType != null) value.put("reward_type", rewardType);
      if (serverVerify != null) value.put("is_server_verify", serverVerify);
      if (errorCode != null) value.put("error_code", errorCode);
      if (errorMessage != null)
        value.put("error_message", errorMessage.substring(0, Math.min(500, errorMessage.length())));
    } catch (org.json.JSONException error) {
      throw new IllegalArgumentException(error);
    }
    return value;
  }

  private static void ui(Activity activity, Runnable action) {
    activity.runOnUiThread(
        () -> {
          if (!activity.isDestroyed()) action.run();
        });
  }
}
