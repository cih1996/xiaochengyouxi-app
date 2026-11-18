package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.Context;
import android.view.View;
import com.baidu.mobads.sdk.api.*;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.json.JSONObject;

/** 百度百青藤 9.460: rewarded video and template (express) feed. */
final class BdNetwork implements AdNetwork {
  private final List<Runnable> waiting = new ArrayList<>();
  private boolean starting, ready;

  public String id() {
    return "baidu";
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
      MobadsPermissionSettings.setPermissionLocation(false);
      MobadsPermissionSettings.setPermissionStorage(false);
      MobadsPermissionSettings.setPermissionAppUpdate(false);
      MobadsPermissionSettings.setPermissionRunningApp(false);
      MobadsPermissionSettings.setPermissionDeviceInfo(false);
      MobadsPermissionSettings.setLimitPersonalAds(true);
      new BDAdConfig.Builder()
          .setAppsid(appId)
          .setAppName(app.getString(R.string.app_name))
          .setDebug(false)
          .setBDAdInitListener(
              new BDAdConfig.BDAdInitListener() {
                @Override
                public void success() {
                  ready = true;
                  starting = false;
                  List<Runnable> callbacks = new ArrayList<>(waiting);
                  waiting.clear();
                  for (Runnable r : callbacks) r.run();
                }

                @Override
                public void fail() {
                  starting = false;
                  waiting.clear();
                  failed.failed("init", "baidu init failed");
                }
              })
          .build(app)
          .init();
    } catch (RuntimeException | LinkageError error) {
      starting = false;
      waiting.clear();
      failed.failed("init", String.valueOf(error));
    }
  }

  public void loadRewarded(Activity activity, String codeId, JSONObject session, Rewarded cb) {
    final RewardVideoAd[] holder = new RewardVideoAd[1];
    final boolean[] state = new boolean[2]; // [offered, shown]
    RewardVideoAd ad =
        new RewardVideoAd(
            activity,
            codeId,
            new RewardVideoAd.RewardVideoAdListener() {
              @Override
              public void onAdShow() {
                state[1] = true;
                cb.shown();
              }

              @Override
              public void onAdClick() {
                cb.clicked();
              }

              @Override
              public void onAdClose(float progress) {
                cb.closed();
              }

              @Override
              public void onAdFailed(String reason) {
                if (state[1]) cb.playError(reason);
                else if (!state[0]) cb.failed("failed", reason);
              }

              @Override
              public void onVideoDownloadSuccess() {
                if (state[0]) return;
                state[0] = true;
                cb.loaded(() -> holder[0].show());
              }

              @Override
              public void onVideoDownloadFailed() {
                if (!state[0]) cb.failed("cache", "video cache failed");
              }

              @Override
              public void playCompletion() {
                cb.completed();
              }

              @Override
              public void onAdLoaded() {}

              @Override
              public void onAdSkip(float progress) {
                cb.skipped();
              }

              @Override
              public void onRewardVerify(boolean valid, Map<String, Object> info) {
                cb.verified(valid, null, null);
              }
            });
    holder[0] = ad;
    ad.setUserId(session.optString("user_id"));
    ad.setExtraInfo(session.optString("media_extra"));
    ad.setShowDialogOnSkip(true);
    ad.load();
  }

  public void loadFeed(Activity activity, String codeId, int widthPx, Feed cb) {
    new BaiduNativeManager(activity, codeId)
        .loadExpressAd(
            new RequestParameters.Builder().setWidth(widthPx).build(),
            new BaiduNativeManager.ExpressAdListener() {
              @Override
              public void onNativeLoad(List<ExpressResponse> list) {
                if (list == null || list.isEmpty()) {
                  cb.failed("empty", "no ad");
                  return;
                }
                ExpressResponse ad = list.get(0);
                ad.bindInteractionActivity(activity);
                ad.setInteractionListener(
                    new ExpressResponse.ExpressInteractionListener() {
                      @Override
                      public void onAdClick() {
                        cb.clicked();
                      }

                      @Override
                      public void onAdExposed() {
                        cb.shown();
                      }

                      @Override
                      public void onAdRenderFail(View view, String message, int code) {
                        ad.destroy();
                        cb.failed(String.valueOf(code), message);
                      }

                      @Override
                      public void onAdRenderSuccess(View view, float width, float height) {
                        View express = ad.getExpressAdView();
                        cb.rendered(express != null ? express : view, ad::destroy);
                      }

                      @Override
                      public void onAdUnionClick() {}
                    });
                ad.setAdDislikeListener(
                    new ExpressResponse.ExpressDislikeListener() {
                      @Override
                      public void onDislikeWindowShow() {}

                      @Override
                      public void onDislikeItemClick(String reason) {
                        cb.dismissed();
                      }

                      @Override
                      public void onDislikeWindowClose() {}
                    });
                ad.render();
              }

              @Override
              public void onNativeFail(int code, String message, ExpressResponse response) {
                cb.failed(String.valueOf(code), message);
              }

              @Override
              public void onNoAd(int code, String message, ExpressResponse response) {
                cb.failed(String.valueOf(code), message);
              }

              @Override
              public void onVideoDownloadSuccess() {}

              @Override
              public void onVideoDownloadFailed() {}

              @Override
              public void onLpClosed() {}
            });
  }
}
