package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.Context;
import android.os.Bundle;
import android.view.View;
import com.bytedance.sdk.openadsdk.*;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.json.JSONObject;

/** 穿山甲 (Pangle / CSJ) 7.8.1.4: rewarded video and template feed. */
final class CsjNetwork implements AdNetwork {
  private final List<Runnable> waiting = new ArrayList<>();
  private boolean starting;

  public String id() {
    return "csj";
  }

  public boolean ready() {
    return TTAdSdk.isSdkReady();
  }

  public void init(Context app, String appId, Runnable ready, Failed failed) {
    if (TTAdSdk.isSdkReady()) {
      ready.run();
      return;
    }
    waiting.add(ready);
    if (starting) return;
    starting = true;
    TTAdSdk.init(
        app,
        new TTAdConfig.Builder()
            .appId(appId)
            .appName(app.getString(R.string.app_name))
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
            starting = false;
            List<Runnable> callbacks = new ArrayList<>(waiting);
            waiting.clear();
            for (Runnable r : callbacks) r.run();
          }

          @Override
          public void fail(int code, String message) {
            starting = false;
            waiting.clear();
            failed.failed(String.valueOf(code), message);
          }
        });
  }

  public void loadRewarded(Activity activity, String codeId, JSONObject session, Rewarded cb) {
    AdSlot slot =
        new AdSlot.Builder()
            .setCodeId(codeId)
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
              boolean offered;

              @Override
              public void onError(int code, String message) {
                cb.failed(String.valueOf(code), message);
              }

              @Override
              public void onRewardVideoAdLoad(TTRewardVideoAd ad) {
                loaded = ad;
              }

              @Override
              public void onRewardVideoCached() {
                if (loaded != null) offer(loaded);
              }

              @Override
              public void onRewardVideoCached(TTRewardVideoAd ad) {
                offer(ad);
              }

              void offer(TTRewardVideoAd ad) {
                if (offered) return;
                offered = true;
                ad.setRewardAdInteractionListener(
                    new TTRewardVideoAd.RewardAdInteractionListener() {
                      @Override
                      public void onAdShow() {
                        cb.shown();
                      }

                      @Override
                      public void onAdVideoBarClick() {
                        cb.clicked();
                      }

                      @Override
                      public void onAdClose() {
                        ad.setRewardAdInteractionListener(null);
                        cb.closed();
                      }

                      @Override
                      public void onVideoComplete() {
                        cb.completed();
                      }

                      @Override
                      public void onVideoError() {
                        cb.playError("视频播放失败");
                      }

                      @Override
                      public void onSkippedVideo() {
                        cb.skipped();
                      }

                      @Override
                      public void onRewardVerify(
                          boolean valid, int amount, String name, int code, String message) {}

                      @Override
                      public void onRewardArrived(boolean valid, int type, Bundle extra) {
                        Integer code =
                            extra != null
                                    && extra.containsKey(TTRewardVideoAd.REWARD_EXTRA_KEY_ERROR_CODE)
                                ? extra.getInt(TTRewardVideoAd.REWARD_EXTRA_KEY_ERROR_CODE)
                                : null;
                        String message =
                            extra == null
                                ? null
                                : extra.getString(TTRewardVideoAd.REWARD_EXTRA_KEY_ERROR_MSG);
                        cb.verified(valid, code, message);
                      }
                    });
                cb.loaded(() -> ad.showRewardVideoAd(activity));
              }
            });
  }

  public void loadFeed(Activity activity, String codeId, int widthPx, Feed cb) {
    float widthDp = widthPx / activity.getResources().getDisplayMetrics().density;
    TTAdSdk.getAdManager()
        .createAdNative(activity)
        .loadNativeExpressAd(
            new AdSlot.Builder()
                .setCodeId(codeId)
                .setAdCount(1)
                .setAdLoadType(TTAdLoadType.LOAD)
                .setExpressViewAcceptedSize(widthDp, 0)
                .build(),
            new TTAdNative.NativeExpressAdListener() {
              @Override
              public void onError(int code, String message) {
                cb.failed(String.valueOf(code), message);
              }

              @Override
              public void onNativeExpressAdLoad(List<TTNativeExpressAd> list) {
                if (list == null || list.isEmpty()) {
                  cb.failed("empty", "no ad");
                  return;
                }
                TTNativeExpressAd ad = list.get(0);
                for (int i = 1; i < list.size(); i++) list.get(i).destroy();
                ad.setExpressInteractionListener(
                    new TTNativeExpressAd.ExpressAdInteractionListener() {
                      @Override
                      public void onAdClicked(View view, int type) {
                        cb.clicked();
                      }

                      @Override
                      public void onAdShow(View view, int type) {
                        cb.shown();
                      }

                      @Override
                      public void onRenderFail(View view, String message, int code) {
                        ad.destroy();
                        cb.failed(String.valueOf(code), message);
                      }

                      @Override
                      public void onRenderSuccess(View view, float width, float height) {
                        cb.rendered(view, ad::destroy);
                      }
                    });
                ad.setDislikeCallback(
                    activity,
                    new TTAdDislike.DislikeInteractionCallback() {
                      @Override
                      public void onShow() {}

                      @Override
                      public void onCancel() {}

                      @Override
                      public void onSelected(int position, String value, boolean enforce) {
                        cb.dismissed();
                      }
                    });
                ad.render();
              }
            });
  }
}
