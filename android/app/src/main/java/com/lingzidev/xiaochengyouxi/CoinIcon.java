package com.lingzidev.xiaochengyouxi;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Typeface;
import android.view.View;

/** Flat gold coin: solid disc, a lighter inner ring and a white ¥. No gradients or bevels. */
final class CoinIcon extends View {
  private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG), t = new Paint(Paint.ANTI_ALIAS_FLAG);

  CoinIcon(Context c) {
    super(c);
    t.setTypeface(Typeface.DEFAULT_BOLD);
    t.setTextAlign(Paint.Align.CENTER);
    t.setColor(0xffffffff);
  }

  @Override
  protected void onDraw(Canvas canvas) {
    float cx = getWidth() / 2f, cy = getHeight() / 2f, r = Math.min(cx, cy);
    p.setStyle(Paint.Style.FILL);
    p.setColor(0xffffb21a);
    canvas.drawCircle(cx, cy, r, p);
    p.setStyle(Paint.Style.STROKE);
    p.setStrokeWidth(r * .12f);
    p.setColor(0xffffd36b);
    canvas.drawCircle(cx, cy, r * .72f, p);
    t.setTextSize(r * 1.0f);
    Paint.FontMetrics m = t.getFontMetrics();
    canvas.drawText("¥", cx, cy - (m.ascent + m.descent) / 2, t);
  }
}
