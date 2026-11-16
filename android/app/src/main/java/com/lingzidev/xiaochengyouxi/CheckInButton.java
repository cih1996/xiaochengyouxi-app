package com.lingzidev.xiaochengyouxi;

import android.animation.*;
import android.content.Context;
import android.graphics.drawable.GradientDrawable;
import android.os.Build;
import android.view.Gravity;
import android.view.View;
import android.view.animation.AccelerateDecelerateInterpolator;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.util.ArrayList;

final class CheckInButton extends FrameLayout {
  private final boolean checked;
  private final LineIcon coin;
  private final TextView title;
  private final WelcomeArt.Star glint;
  private final ArrayList<Animator> motion = new ArrayList<>();
  private boolean busy;

  CheckInButton(Context context, boolean checked) {
    super(context);
    this.checked = checked;
    setMinimumHeight(dp(33));
    GradientDrawable background =
        new GradientDrawable(
            GradientDrawable.Orientation.LEFT_RIGHT,
            checked ? new int[] {0xffffedd9, 0xffffedd9} : new int[] {0xffffd57c, 0xffffa650});
    background.setCornerRadius(dp(18));
    setBackground(background);
    LinearLayout row = new LinearLayout(context);
    row.setGravity(Gravity.CENTER);
    row.setPadding(dp(10), 0, dp(13), 0);
    coin = new LineIcon(context, checked ? "check" : "coin", checked ? 0xffb99575 : 0xffa34a22);
    row.addView(coin, new LinearLayout.LayoutParams(dp(17), dp(17)));
    title = new TextView(context);
    title.setText(checked ? "今日已签到" : "签到领金币");
    title.setTextSize(12);
    title.setTypeface(null, 1);
    title.setTextColor(checked ? 0xffb99575 : 0xff933f1c);
    title.setPadding(dp(6), 0, 0, 0);
    title.setIncludeFontPadding(false);
    row.addView(title, new LinearLayout.LayoutParams(-2, -2));
    addView(row, new FrameLayout.LayoutParams(-2, -1, Gravity.CENTER));
    glint = new WelcomeArt.Star(context);
    FrameLayout.LayoutParams gp =
        new FrameLayout.LayoutParams(dp(7), dp(7), Gravity.TOP | Gravity.RIGHT);
    gp.topMargin = dp(3);
    gp.rightMargin = dp(6);
    addView(glint, gp);
    glint.setVisibility(checked ? View.GONE : View.VISIBLE);
    setEnabled(!checked);
    setContentDescription(checked ? "今日已签到" : "签到领取金币");
  }

  void setBusy(boolean value) {
    busy = value;
    title.setText(busy ? "领取中…" : checked ? "今日已签到" : "签到领金币");
    setEnabled(!checked && !busy);
    if (value) stopMotion();
    else if (isShown()) startMotion();
  }

  @Override
  public void onVisibilityAggregated(boolean visible) {
    super.onVisibilityAggregated(visible);
    if (visible) startMotion();
    else stopMotion();
  }

  @Override
  protected void onDetachedFromWindow() {
    stopMotion();
    super.onDetachedFromWindow();
  }

  private void startMotion() {
    if (checked
        || busy
        || !motion.isEmpty()
        || Build.VERSION.SDK_INT >= 26 && !ValueAnimator.areAnimatorsEnabled()) return;
    ObjectAnimator bounce =
        ObjectAnimator.ofFloat(coin, View.TRANSLATION_Y, 0, -dp(4), 0, -dp(2), 0, 0, 0, 0);
    bounce.setDuration(3200);
    bounce.setRepeatCount(ValueAnimator.INFINITE);
    bounce.setInterpolator(new AccelerateDecelerateInterpolator());
    ObjectAnimator shine = ObjectAnimator.ofFloat(glint, View.ALPHA, .15f, 1, .15f, .15f);
    shine.setDuration(3200);
    shine.setRepeatCount(ValueAnimator.INFINITE);
    motion.add(bounce);
    motion.add(shine);
    for (Animator animator : motion) animator.start();
  }

  private void stopMotion() {
    for (Animator animator : motion) animator.cancel();
    motion.clear();
    coin.setTranslationY(0);
  }

  private int dp(float value) {
    return Math.round(value * getResources().getDisplayMetrics().density);
  }
}
