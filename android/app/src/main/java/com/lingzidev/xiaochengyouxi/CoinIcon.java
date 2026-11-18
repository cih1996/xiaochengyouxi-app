package com.lingzidev.xiaochengyouxi;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RadialGradient;
import android.graphics.RectF;
import android.graphics.Shader;
import android.graphics.Typeface;
import android.view.View;

/** A drawn gold coin: rim, inner groove, embossed ¥ and a soft highlight. */
final class CoinIcon extends View {
  private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG), t = new Paint(Paint.ANTI_ALIAS_FLAG);
  private final RectF arc = new RectF();

  CoinIcon(Context c) {
    super(c);
    t.setTypeface(Typeface.DEFAULT_BOLD);
    t.setTextAlign(Paint.Align.CENTER);
  }

  @Override
  protected void onDraw(Canvas canvas) {
    float cx = getWidth() / 2f, cy = getHeight() / 2f, r = Math.min(cx, cy);
    // edge (slightly darker, a hint of thickness at the bottom)
    p.setShader(null);
    p.setColor(0xffc77a00);
    canvas.drawCircle(cx, cy + r * .06f, r * .97f, p);
    // face
    p.setColor(0xff000000);
    p.setShader(new RadialGradient(cx - r * .35f, cy - r * .4f, r * 1.4f, new int[] {0xfffff3b8, 0xffffc93c, 0xffeb9a00}, new float[] {0f, .45f, 1f}, Shader.TileMode.CLAMP));
    canvas.drawCircle(cx, cy, r * .92f, p);
    p.setShader(null);
    // inner groove
    p.setStyle(Paint.Style.STROKE);
    p.setStrokeWidth(r * .09f);
    p.setColor(0x66b06a00);
    canvas.drawCircle(cx, cy, r * .66f, p);
    // highlight sweep on the upper left
    p.setColor(0x99ffffff);
    p.setStrokeWidth(r * .1f);
    p.setStrokeCap(Paint.Cap.ROUND);
    arc.set(cx - r * .78f, cy - r * .78f, cx + r * .78f, cy + r * .78f);
    canvas.drawArc(arc, 200, 60, false, p);
    p.setStyle(Paint.Style.FILL);
    // embossed ¥
    t.setTextSize(r * .95f);
    Paint.FontMetrics m = t.getFontMetrics();
    float base = cy - (m.ascent + m.descent) / 2;
    t.setColor(0x80ffffff);
    canvas.drawText("¥", cx, base + r * .05f, t);
    t.setColor(0xffa45a00);
    canvas.drawText("¥", cx, base, t);
  }
}
