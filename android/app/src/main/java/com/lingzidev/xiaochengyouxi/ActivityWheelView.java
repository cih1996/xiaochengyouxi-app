package com.lingzidev.xiaochengyouxi;

import android.animation.ObjectAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.Typeface;
import android.view.View;
import android.view.animation.DecelerateInterpolator;

/** Lightweight in-app wheel visual; the server still decides the result. */
final class ActivityWheelView extends View {
  private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
  private final String[] labels = {"金币", "惊喜", "再来", "好运", "奖励", "加倍"};
  private float rotation;

  ActivityWheelView(Context context) {
    super(context);
    setContentDescription("活动转盘");
    paint.setTypeface(Typeface.create(Typeface.DEFAULT, Typeface.BOLD));
  }

  @Override
  protected void onDraw(Canvas canvas) {
    super.onDraw(canvas);
    float cx = getWidth() / 2f, cy = getHeight() / 2f, radius = Math.min(cx, cy) - 10;
    canvas.save();
    canvas.rotate(rotation, cx, cy);
    int[] colors = {0xffff9d5c, 0xfff25d3f, 0xffffd080, 0xfff47b58, 0xffffb66a, 0xffe85a43};
    for (int i = 0; i < labels.length; i++) {
      paint.setColor(colors[i]);
      Path sector = new Path();
      sector.moveTo(cx, cy);
      sector.arcTo(cx - radius, cy - radius, cx + radius, cy + radius, i * 60 - 90, 60, true);
      sector.close();
      canvas.drawPath(sector, paint);
      paint.setColor(0xfffffbef);
      paint.setTextSize(Math.max(12, radius * .12f));
      float textWidth = paint.measureText(labels[i]);
      double angle = Math.toRadians(i * 60 - 60);
      float tx = cx + (float) Math.cos(angle) * radius * .58f - textWidth / 2;
      float ty = cy + (float) Math.sin(angle) * radius * .58f + paint.getTextSize() / 3;
      canvas.drawText(labels[i], tx, ty, paint);
    }
    paint.setColor(0xfffff1ca);
    canvas.drawCircle(cx, cy, radius * .22f, paint);
    paint.setColor(0xffe96b43);
    paint.setTextSize(radius * .11f);
    String center = "转盘";
    canvas.drawText(
        center, cx - paint.measureText(center) / 2, cy + paint.getTextSize() / 3, paint);
    canvas.restore();
    paint.setColor(0xffd94d35);
    Path pointer = new Path();
    pointer.moveTo(cx, 4);
    pointer.lineTo(cx - 12, 25);
    pointer.lineTo(cx + 12, 25);
    pointer.close();
    canvas.drawPath(pointer, paint);
  }

  void spin() {
    ObjectAnimator animator =
        ObjectAnimator.ofFloat(this, "wheelRotation", rotation, rotation + 1440f + 180f);
    animator.setDuration(1200);
    animator.setInterpolator(new DecelerateInterpolator());
    animator.start();
  }

  public float getWheelRotation() {
    return rotation;
  }

  public void setWheelRotation(float value) {
    rotation = value % 360f;
    invalidate();
  }
}
