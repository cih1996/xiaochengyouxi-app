package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.Context;
import android.view.View;
import com.kwad.sdk.api.*;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.json.JSONObject;

/** 快手 (Kuaishou) 5.9.10.1: rewarded video and template feed. */
final class KsNetwork implements AdNetwork {
  private final List<Runnable> waiting = new ArrayList<>();
  private boolean starting, ready;

  public String id() {
    return "kuaishou";
  }

  public boolean ready() {
    return ready;
  }

  public void init(Context app, String appId, Runnable done, Failed failed) {
    if (ready) {
      done.run();
      return;
    }
    waiting.add(done);
    if (starting) return;
    starting = true;
    try {
      boolean ok =
          KsAdSDK.init(
              app,
              new SdkConfig.Builder()
                  .appId(appId)
                  .appName(app.getString(R.string.app_name))
                  .debug(false)
                  .showNotification(false)
                  .canReadMacAddress(false)
                  .canReadNearbyWifiList(false)
                  .canReadICCID(false)
                  .customController(
                      new KsCustomController() {
                        @Override
                        public boolean canReadLocation() {
                          return false;
                        }

                        @Override
                        public boolean canUsePhoneState() {
                          return false;
                        }

                        @Override
                        public boolean canUseMacAddress() {
                          return false;
                        }

                        @Override
                        public boolean canUseStoragePermission() {
                          return false;
                        }

                        @Override
                        public boolean canReadInstalledPackages() {
                          return false;
                        }
                      })
                  .setStartCallback(
                      new KsInitCallback() {
                        @Override
                        public void onSuccess() {
                          ready = true;
                          starting = false;
                          List<Runnable> callbacks = new ArrayList<>(waiting);
                          waiting.clear();
                          for (Runnable r : callbacks) r.run();
                        }

                        @Override
                        public void onFail(int code, String message) {
                          starting = false;
                          waiting.clear();
                          failed.failed(String.valueOf(code), message);
                        }
                      })
                  .build());
      if (!ok) {
        starting = false;
        waiting.clear();
        failed.failed("init", "init returned false");
        return;
      }
      KsAdSDK.setPersonalRecommend(false);
      KsAdSDK.start();
    } catch (RuntimeException | LinkageError error) {
      starting = false;
      waiting.clear();
      failed.failed("init", String.valueOf(error));
    }
  }

  private static long pos(String codeId) {
    try {
      return Long.parseLong(codeId);
    } catch (NumberFormatException e) {
      return 0;
    }
  }

  public void loadRewarded(Activity activity, String codeId, JSONObject session, Rewarded cb) {
    KsLoadManager manager = KsAdSDK.getLoadManager();
    if (manager == null || pos(codeId) == 0) {
      cb.failed("unavailable", "load manager unavailable");
      return;
    }
    Map<String, String> extra = new HashMap<>();
    extra.put("thirdUserId", session.optString("user_id"));
    extra.put("extraData", session.optString("media_extra"));
    KsScene scene =
        new KsScene.Builder(pos(codeId))
            .adNum(1)
            .screenOrientation(SdkConfig.SCREEN_ORIENTATION_PORTRAIT)
            .rewardCallbackExtraData(extra)
            .build();
    manager.loadRewardVideoAd(
        scene,
        new KsLoadManager.RewardVideoAdListener() {
          @Override
          public void onError(int code, String message) {
            cb.failed(String.valueOf(code), message);
          }

          @Override
          public void onRewardVideoResult(List<KsRewardVideoAd> list) {}

          @Override
          public void onRewardVideoAdLoad(List<KsRewardVideoAd> list) {
            if (list == null || list.isEmpty() || !list.get(0).isAdEnable()) {
              cb.failed("empty", "no ad");
              return;
            }
            KsRewardVideoAd ad = list.get(0);
            ad.setRewardAdInteractionListener(
                new KsRewardVideoAd.RewardAdInteractionListener() {
                  @Override
                  public void onAdClicked() {
                    cb.clicked();
                  }

                  @Override
                  public void onPageDismiss() {
                    cb.closed();
                  }

                  @Override
                  public void onVideoPlayError(int code, int extra) {
                    cb.playError(code + "/" + extra);
                  }

                  @Override
                  public void onVideoPlayEnd() {
                    cb.completed();
                  }

                  @Override
                  public void onVideoSkipToEnd(long duration) {
                    cb.skipped();
                  }

                  @Override
                  public void onVideoPlayStart() {
                    cb.shown();
                  }

                  @Override
                  public void onRewardVerify() {
                    cb.verified(true, null, null);
                  }

                  @Override
                  public void onRewardVerify(Map<String, Object> extras) {
                    Object fraud = extras == null ? null : extras.get(ApiConst.EXTRA_KEY_FRAUD);
                    Object code = extras == null ? null : extras.get(ApiConst.EXTRA_KEY_ERRORCODE);
                    boolean invalid =
                        Boolean.TRUE.equals(fraud)
                            || "true".equalsIgnoreCase(String.valueOf(fraud))
                            || "1".equals(String.valueOf(fraud))
                            || (code != null
                                && !"0".equals(String.valueOf(code))
                                && !"0.0".equals(String.valueOf(code)));
                    cb.verified(!invalid, null, invalid ? "fraud=" + fraud + " code=" + code : null);
                  }

                  @Override
                  public void onRewardStepVerify(int type, int status) {}

                  @Override
                  public void onExtraRewardVerify(int type) {}
                });
            cb.loaded(
                () ->
                    ad.showRewardVideoAd(
                        activity, new KsVideoPlayConfig.Builder().showLandscape(false).build()));
          }
        });
  }

  public void loadFeed(Activity activity, String codeId, int widthPx, Feed cb) {
    KsLoadManager manager = KsAdSDK.getLoadManager();
    if (manager == null || pos(codeId) == 0) {
      cb.failed("unavailable", "load manager unavailable");
      return;
    }
    manager.loadConfigFeedAd(
        new KsScene.Builder(pos(codeId)).adNum(1).width(widthPx).build(),
        new KsLoadManager.FeedAdListener() {
          @Override
          public void onError(int code, String message) {
            cb.failed(String.valueOf(code), message);
          }

          @Override
          public void onFeedAdLoad(List<KsFeedAd> list) {
            if (list == null || list.isEmpty()) {
              cb.failed("empty", "no ad");
              return;
            }
            KsFeedAd ad = list.get(0);
            ad.setVideoSoundEnable(false);
            ad.setAdInteractionListener(
                new KsFeedAd.AdInteractionListener() {
                  @Override
                  public void onAdClicked() {
                    cb.clicked();
                  }

                  @Override
                  public void onAdShow() {
                    cb.shown();
                  }

                  @Override
                  public void onDislikeClicked() {
                    cb.dismissed();
                  }

                  @Override
                  public void onDownloadTipsDialogShow() {}

                  @Override
                  public void onDownloadTipsDialogDismiss() {}
                });
            View view = ad.getFeedView(activity);
            if (view == null) cb.failed("render", "empty view");
            else cb.rendered(view, ad::destroy);
          }
        });
  }
}
