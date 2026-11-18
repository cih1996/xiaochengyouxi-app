package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.graphics.drawable.GradientDrawable;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Consumer;
import org.json.JSONArray;
import org.json.JSONObject;

/**
 * Hall feed cards. Each hall item carries the server's plan ("ads"); the mediator tries the
 * networks in order. A card that gets no ad quietly disappears — no error is shown to the user.
 */
final class NativeFeedAds {
  private final Activity activity;
  private final AdMediator mediator;
  private final Consumer<String> removed;
  private final Map<String, Entry> ads = new LinkedHashMap<>();
  private boolean foreground;

  NativeFeedAds(Activity activity, AdMediator mediator, Consumer<String> removed) {
    this.activity = activity;
    this.mediator = mediator;
    this.removed = removed;
  }

  View card(JSONObject item, float widthDp) {
    String id = item.optString("id");
    Entry entry = ads.get(id);
    if (entry == null) {
      JSONArray plan = item.optJSONArray("ads");
      if (plan == null && !item.optString("ad_code_id").isEmpty()) {
        plan = new JSONArray();
        plan.put(
            Api.object(
                "network", "csj", "app_id", BuildConfig.CSJ_APP_ID, "code_id", item.optString("ad_code_id")));
      }
      entry = new Entry(id, plan == null ? new JSONArray() : plan, widthDp);
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
    mediator.flush();
  }

  void destroyAll() {
    for (Entry entry : ads.values()) entry.destroy();
    ads.clear();
  }

  private void activate(Entry entry) {
    if (!foreground || entry.removed || entry.attempted || !entry.card.isAttachedToWindow()) return;
    entry.attempted = true;
    int widthPx = Math.max(dp(200), dp(entry.widthDp) - dp(24));
    mediator.loadFeed(
        activity,
        entry.plan,
        widthPx,
        new AdMediator.FeedResult() {
          @Override
          public void rendered(View view, Runnable destroy) {
            if (entry.removed) {
              destroy.run();
              return;
            }
            entry.destroyAd = destroy;
            if (view.getParent() instanceof ViewGroup) ((ViewGroup) view.getParent()).removeView(view);
            entry.host.removeAllViews();
            entry.host.addView(view, new FrameLayout.LayoutParams(-1, -2));
            entry.card.setVisibility(View.VISIBLE);
          }

          @Override
          public void empty() {
            drop(entry);
          }

          @Override
          public void dismissed() {
            drop(entry);
          }
        });
  }

  private void drop(Entry entry) {
    if (entry.removed) return;
    entry.destroy();
    ads.remove(entry.id);
    removed.accept(entry.id);
  }

  private int dp(float value) {
    return Math.round(value * activity.getResources().getDisplayMetrics().density);
  }

  private final class Entry {
    final String id;
    final JSONArray plan;
    final float widthDp;
    final LinearLayout card;
    final FrameLayout host;
    Runnable destroyAd;
    boolean attempted, removed;

    Entry(String id, JSONArray plan, float widthDp) {
      this.id = id;
      this.plan = plan;
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
      if (destroyAd != null) {
        try {
          destroyAd.run();
        } catch (RuntimeException ignored) {
        }
      }
      destroyAd = null;
      host.removeAllViews();
      card.setVisibility(View.GONE);
      if (card.getParent() instanceof ViewGroup) ((ViewGroup) card.getParent()).removeView(card);
    }
  }
}
