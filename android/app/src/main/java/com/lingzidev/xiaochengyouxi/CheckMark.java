package com.lingzidev.xiaochengyouxi;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.PathMeasure;
import android.view.View;
import android.view.animation.OvershootInterpolator;

/** WeChat-Pay-style success mark: a green disc pops in, then a white tick draws itself. */
final class CheckMark extends View {
  static final int GREEN = 0xff07c160;
  private final Paint disc = new Paint(Paint.ANTI_ALIAS_FLAG), tick = new Paint(Paint.ANTI_ALIAS_FLAG);
  private final Path path = new Path(), part = new Path();
  private float scale = 0f, drawn = 0f;

  CheckMark(Context c) {
    super(c);
    disc.setColor(GREEN);
    tick.setColor(0xffffffff);
    tick.setStyle(Paint.Style.STROKE);
    tick.setStrokeCap(Paint.Cap.ROUND);
    tick.setStrokeJoin(Paint.Join.ROUND);
  }

  void play() {
    if (!ValueAnimator.areAnimatorsEnabled()) {
      scale = 1f;
      drawn = 1f;
      invalidate();
      return;
    }
    ValueAnimator pop = ValueAnimator.ofFloat(0f, 1f);
    pop.setDuration(420);
    pop.setInterpolator(new OvershootInterpolator(2f));
    pop.addUpdateListener(a -> {
      scale = (float) a.getAnimatedValue();
      invalidate();
    });
    ValueAnimator draw = ValueAnimator.ofFloat(0f, 1f);
    draw.setDuration(380);
    draw.setStartDelay(260);
    draw.addUpdateListener(a -> {
      drawn = (float) a.getAnimatedValue();
      invalidate();
    });
    pop.start();
    draw.start();
  }

  @Override
  protected void onDraw(Canvas canvas) {
    float w = getWidth(), h = getHeight(), r = Math.min(w, h) / 2f;
    canvas.drawCircle(w / 2, h / 2, r * scale, disc);
    if (drawn <= 0) return;
    tick.setStrokeWidth(r * .16f);
    path.reset();
    path.moveTo(w / 2 - r * .42f, h / 2 + r * .02f);
    path.lineTo(w / 2 - r * .1f, h / 2 + r * .32f);
    path.lineTo(w / 2 + r * .45f, h / 2 - r * .3f);
    PathMeasure m = new PathMeasure(path, false);
    part.reset();
    m.getSegment(0, m.getLength() * drawn, part, true);
    canvas.drawPath(part, tick);
  }
}
