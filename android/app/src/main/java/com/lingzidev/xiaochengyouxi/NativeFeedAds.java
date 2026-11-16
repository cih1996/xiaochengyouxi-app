package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.graphics.drawable.GradientDrawable;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import com.bytedance.sdk.openadsdk.*;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;

final class NativeFeedAds {
  private final Activity activity;
  private final Api.Failure failure;
  private final Consumer<String> removed;
  private final Map<String, Entry> ads = new LinkedHashMap<>();
  private boolean foreground;

  NativeFeedAds(Activity activity, Api.Failure failure, Consumer<String> removed) {
    this.activity = activity;
    this.failure = failure;
    this.removed = removed;
  }

  View card(String id, String codeId, float widthDp) {
    Entry entry = ads.get(id);
    if (entry == null) {
      entry = new Entry(id, codeId, widthDp);
      ads.put(id, entry);
    }
    if (entry.card.getParent() instanceof ViewGroup) {
      ((ViewGroup) entry.card.getParent()).removeView(entry.card);
    }
    Entry selected = entry;
    entry.card.post(() -> activate(selected));
    return entry.card;
  }

  void resume() {
    foreground = true;
    for (Entry entry : new ArrayList<>(ads.values())) activate(entry);
  }

  void pause() {
    foreground = false;
  }

  void destroyAll() {
    for (Entry entry : ads.values()) entry.destroy();
    ads.clear();
  }

  private void activate(Entry entry) {
    if (!foreground || entry.removed || !entry.card.isAttachedToWindow()) return;
    if (entry.ad != null && !entry.rendering) {
      entry.rendering = true;
      entry.ad.render();
    } else if (!entry.attempted) {
      entry.attempted = true;
      if (!TTAdSdk.isSdkReady()) {
        reject(entry, "穿山甲 SDK 尚未初始化成功");
        return;
      }
      TTAdSdk.getAdManager()
          .createAdNative(activity)
          .loadNativeExpressAd(
              new AdSlot.Builder()
                  .setCodeId(entry.codeId)
                  .setAdCount(1)
                  .setAdLoadType(TTAdLoadType.LOAD)
                  .setExpressViewAcceptedSize(entry.widthDp, 0)
                  .build(),
              new TTAdNative.NativeExpressAdListener() {
                @Override
                public void onError(int code, String message) {
                  ui(() -> reject(entry, "穿山甲信息流加载（" + code + "）：" + message));
                }

                @Override
                public void onNativeExpressAdLoad(List<TTNativeExpressAd> loaded) {
                  activity.runOnUiThread(
                      () -> {
                        if (entry.removed || activity.isDestroyed()) {
                          for (TTNativeExpressAd ad : loaded) ad.destroy();
                          return;
                        }
                        if (loaded.isEmpty()) {
                          reject(entry, "穿山甲信息流返回空广告列表");
                          return;
                        }
                        entry.ad = loaded.get(0);
                        for (int i = 1; i < loaded.size(); i++) loaded.get(i).destroy();
                        entry.ad.setExpressInteractionListener(
                            new TTNativeExpressAd.ExpressAdInteractionListener() {
                              @Override
                              public void onAdClicked(View view, int type) {}

                              @Override
                              public void onAdShow(View view, int type) {}

                              @Override
                              public void onRenderFail(View view, String message, int code) {
                                ui(() -> reject(entry, "穿山甲信息流渲染（" + code + "）：" + message));
                              }

                              @Override
                              public void onRenderSuccess(View view, float width, float height) {
                                ui(
                                    () -> {
                                      if (entry.removed) return;
                                      if (view.getParent() instanceof ViewGroup)
                                        ((ViewGroup) view.getParent()).removeView(view);
                                      entry.host.removeAllViews();
                                      entry.host.addView(
                                          view, new FrameLayout.LayoutParams(-1, -2));
                                      entry.card.setVisibility(View.VISIBLE);
                                    });
                              }
                            });
                        entry.ad.setDislikeCallback(
                            activity,
                            new TTAdDislike.DislikeInteractionCallback() {
                              @Override
                              public void onShow() {}

                              @Override
                              public void onCancel() {}

                              @Override
                              public void onSelected(int position, String value, boolean enforce) {
                                ui(
                                    () -> {
                                      if (entry.removed) return;
                                      entry.destroy();
                                      ads.remove(entry.id);
                                      removed.accept(entry.id);
                                    });
                              }
                            });
                        activate(entry);
                      });
                }
              });
    }
  }

  private void reject(Entry entry, String message) {
    if (entry.removed) return;
    entry.destroy();
    ads.remove(entry.id);
    removed.accept(entry.id);
    failure.failed(message);
  }

  private void ui(Runnable action) {
    activity.runOnUiThread(
        () -> {
          if (!activity.isDestroyed()) action.run();
        });
  }

  private int dp(float value) {
    return Math.round(value * activity.getResources().getDisplayMetrics().density);
  }

  private final class Entry {
    final String id, codeId;
    final float widthDp;
    final LinearLayout card;
    final FrameLayout host;
    TTNativeExpressAd ad;
    boolean attempted, rendering, removed;

    Entry(String id, String codeId, float widthDp) {
      this.id = id;
      this.codeId = codeId;
      this.widthDp = widthDp;
      card = new LinearLayout(activity);
      card.setOrientation(LinearLayout.VERTICAL);
      card.setPadding(dp(12), dp(12), dp(12), dp(12));
      GradientDrawable background = new GradientDrawable();
      background.setColor(0xffffffff);
      background.setCornerRadius(dp(20));
      card.setBackground(background);
      card.setVisibility(View.GONE);
      TextView title = new TextView(activity);
      title.setText("为你精选 · 广告");
      title.setTextSize(11);
      title.setTextColor(0xffa38c7e);
      title.setPadding(0, 0, 0, dp(8));
      card.addView(title, new LinearLayout.LayoutParams(-1, -2));
      host = new FrameLayout(activity);
      card.addView(host, new LinearLayout.LayoutParams(-1, -2));
    }

    void destroy() {
      removed = true;
      if (ad != null) ad.destroy();
      ad = null;
      host.removeAllViews();
      card.setVisibility(View.GONE);
      if (card.getParent() instanceof ViewGroup) ((ViewGroup) card.getParent()).removeView(card);
    }
  }
}
