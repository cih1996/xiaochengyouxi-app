package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.Context;
import com.qq.e.ads.nativ.ADSize;
import com.qq.e.ads.nativ.NativeExpressAD;
import com.qq.e.ads.nativ.NativeExpressADView;
import com.qq.e.ads.rewardvideo.RewardVideoAD;
import com.qq.e.ads.rewardvideo.RewardVideoADListener;
import com.qq.e.ads.rewardvideo.ServerSideVerificationOptions;
import com.qq.e.comm.managers.GDTAdSdk;
import com.qq.e.comm.managers.setting.GlobalSetting;
import com.qq.e.comm.util.AdError;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import org.json.JSONObject;

/** 腾讯优量汇 4.640.1510: rewarded video and template (express) feed. */
final class GdtNetwork implements AdNetwork {
  private final List<Runnable> waiting = new ArrayList<>();
  private boolean starting, ready;

  public String id() {
    return "gdt";
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
      GlobalSetting.setPersonalizedState(1); // 1 = no personalised ads
      GlobalSetting.setAgreeReadDeviceId(false);
      GlobalSetting.setEnableCollectAppInstallStatus(false);
      // The app now holds location permission (for DeviceTrust only); keep 优量汇 off it.
      java.util.Map<String, Boolean> privacy = new java.util.HashMap<>();
      privacy.put("cell_id", false);
      privacy.put("mac_address", false);
      privacy.put("android_id", false);
      privacy.put("device_id", false);
      GlobalSetting.setAgreeReadPrivacyInfo(privacy);
      GDTAdSdk.initWithoutStart(app, appId);
      GDTAdSdk.start(
          new GDTAdSdk.OnStartListener() {
            @Override
            public void onStartSuccess() {
              ready = true;
              starting = false;
              List<Runnable> callbacks = new ArrayList<>(waiting);
              waiting.clear();
              for (Runnable r : callbacks) r.run();
            }

            @Override
            public void onStartFailed(Exception error) {
              starting = false;
              waiting.clear();
              failed.failed("init", String.valueOf(error));
            }
          });
    } catch (RuntimeException | LinkageError error) {
      starting = false;
      waiting.clear();
      failed.failed("init", String.valueOf(error));
    }
  }

  public void loadRewarded(Activity activity, String codeId, JSONObject session, Rewarded cb) {
    final RewardVideoAD[] holder = new RewardVideoAD[1];
    final boolean[] state = new boolean[2]; // [offered, shown]
    RewardVideoAD ad =
        new RewardVideoAD(
            activity,
            codeId,
            new RewardVideoADListener() {
              @Override
              public void onADLoad() {}

              @Override
              public void onVideoCached() {
                if (state[0]) return;
                state[0] = true;
                cb.loaded(() -> holder[0].showAD(activity));
              }

              @Override
              public void onADShow() {
                state[1] = true;
              }

              @Override
              public void onADExpose() {
                cb.shown();
              }

              @Override
              public void onReward(Map<String, Object> map) {
                cb.verified(true, null, null);
              }

              @Override
              public void onADClick() {
                cb.clicked();
              }

              @Override
              public void onVideoComplete() {
                cb.completed();
              }

              @Override
              public void onADClose() {
                cb.closed();
              }

              @Override
              public void onError(AdError error) {
                String code = error == null ? "error" : String.valueOf(error.getErrorCode());
                String message = error == null ? "" : error.getErrorMsg();
                if (state[1]) cb.playError(code + " " + message);
                else if (!state[0]) cb.failed(code, message);
              }
            },
            true);
    holder[0] = ad;
    ad.setServerSideVerificationOptions(
        new ServerSideVerificationOptions.Builder()
            .setUserId(session.optString("user_id"))
            .setCustomData(session.optString("media_extra"))
            .build());
    ad.loadAD();
  }

  public void loadFeed(Activity activity, String codeId, int widthPx, Feed cb) {
    new NativeExpressAD(
            activity,
            new ADSize(ADSize.FULL_WIDTH, ADSize.AUTO_HEIGHT),
            codeId,
            new NativeExpressAD.NativeExpressADListener() {
              @Override
              public void onNoAD(AdError error) {
                cb.failed(
                    error == null ? "error" : String.valueOf(error.getErrorCode()),
                    error == null ? "" : error.getErrorMsg());
              }

              @Override
              public void onADLoaded(List<NativeExpressADView> list) {
                if (list == null || list.isEmpty()) {
                  cb.failed("empty", "no ad");
                  return;
                }
                for (int i = 1; i < list.size(); i++) list.get(i).destroy();
                list.get(0).render();
              }

              @Override
              public void onRenderFail(NativeExpressADView view) {
                view.destroy();
                cb.failed("render", "render failed");
              }

              @Override
              public void onRenderSuccess(NativeExpressADView view) {
                cb.rendered(view, view::destroy);
              }

              @Override
              public void onADExposure(NativeExpressADView view) {
                cb.shown();
              }

              @Override
              public void onADClicked(NativeExpressADView view) {
                cb.clicked();
              }

              @Override
              public void onADClosed(NativeExpressADView view) {
                cb.dismissed();
              }

              @Override
              public void onADLeftApplication(NativeExpressADView view) {}
            })
        .loadAD(1);
  }
}
