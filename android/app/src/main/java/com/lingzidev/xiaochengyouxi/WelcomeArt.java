package com.lingzidev.xiaochengyouxi;

import android.animation.Animator;
import android.animation.ObjectAnimator;
import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.*;
import android.graphics.drawable.GradientDrawable;
import android.os.Build;
import android.view.View;
import android.view.animation.AccelerateDecelerateInterpolator;
import android.widget.FrameLayout;
import android.widget.ImageView;
import java.util.ArrayList;

final class WelcomeArt extends FrameLayout {
  private final ImageView mascot, packet;
  private final View glow;
  private final ArrayList<Star> stars = new ArrayList<>();
  private final ArrayList<Animator> motion = new ArrayList<>();

  WelcomeArt(Context context) {
    super(context);
    setClipChildren(false);
    setClipToPadding(false);
    glow = new View(context);
    GradientDrawable halo =
        new GradientDrawable(
            GradientDrawable.Orientation.TL_BR, new int[] {0xffffe3c7, 0xfffff1df});
    halo.setShape(GradientDrawable.OVAL);
    glow.setBackground(halo);
    addView(glow);
    mascot = art("mascot");
    addView(mascot);
    packet = art("packet");
    addView(packet);
    for (int i = 0; i < 7; i++) {
      Star star = new Star(context);
      stars.add(star);
      addView(star);
    }
  }

  private ImageView art(String name) {
    ImageView image = new ImageView(getContext());
    image.setImageResource(
        getResources().getIdentifier(name, "drawable", getContext().getPackageName()));
    image.setScaleType(ImageView.ScaleType.FIT_CENTER);
    return image;
  }

  @Override
  protected void onLayout(boolean changed, int l, int t, int r, int b) {
    int w = getWidth(), h = getHeight();
    place(glow, w * .1f, h * .12f, w * .9f, h * .9f);
    place(mascot, w * .15f, h * .02f, w * .97f, h * .95f);
    place(packet, -w * .02f, h * .53f, w * .39f, h * 1.02f);
    float[][] points = {
      {.12f, .2f},
      {.85f, .13f},
      {.94f, .45f},
      {.17f, .47f},
      {.79f, .91f},
      {.47f, .07f},
      {.04f, .79f}
    };
    for (int i = 0; i < stars.size(); i++) {
      float size = dp(i % 2 == 0 ? 12 : 8), x = points[i][0] * w, y = points[i][1] * h;
      place(stars.get(i), x - size / 2, y - size / 2, x + size / 2, y + size / 2);
    }
  }

  private void place(View v, float l, float t, float r, float b) {
    v.layout((int) l, (int) t, (int) r, (int) b);
  }

  private float dp(float value) {
    return value * getResources().getDisplayMetrics().density;
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
    if (!motion.isEmpty() || Build.VERSION.SDK_INT >= 26 && !ValueAnimator.areAnimatorsEnabled())
      return;
    animate(mascot, "translationY", 0, -dp(9), 0, 3200, 0);
    animate(packet, "translationY", 0, dp(7), 0, 2800, 180);
    animate(packet, "rotation", -8, -2, -8, 3200, 180);
    for (int i = 0; i < stars.size(); i++) {
      Star star = stars.get(i);
      animate(star, "alpha", .25f, .9f, .25f, 2400 + i * 70, i * 180);
      animate(star, "scaleX", .7f, 1.15f, .7f, 2400 + i * 70, i * 180);
      animate(star, "scaleY", .7f, 1.15f, .7f, 2400 + i * 70, i * 180);
    }
  }

  private void animate(
      View view, String property, float start, float peak, float end, long duration, long delay) {
    ObjectAnimator animator = ObjectAnimator.ofFloat(view, property, start, peak, end);
    animator.setDuration(duration);
    animator.setStartDelay(delay);
    animator.setRepeatCount(ValueAnimator.INFINITE);
    animator.setInterpolator(new AccelerateDecelerateInterpolator());
    motion.add(animator);
    animator.start();
  }

  private void stopMotion() {
    for (Animator animator : motion) animator.cancel();
    motion.clear();
  }

  static final class Star extends View {
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Path path = new Path();

    Star(Context context) {
      super(context);
      paint.setColor(0xffeeb86d);
      setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO);
    }

    @Override
    protected void onSizeChanged(int w, int h, int oldw, int oldh) {
      path.reset();
      path.moveTo(w * .5f, 0);
      path.quadTo(w * .62f, h * .38f, w, h * .5f);
      path.quadTo(w * .62f, h * .62f, w * .5f, h);
      path.quadTo(w * .38f, h * .62f, 0, h * .5f);
      path.quadTo(w * .38f, h * .38f, w * .5f, 0);
      path.close();
    }

    @Override
    protected void onDraw(Canvas canvas) {
      canvas.drawPath(path, paint);
    }
  }
}
