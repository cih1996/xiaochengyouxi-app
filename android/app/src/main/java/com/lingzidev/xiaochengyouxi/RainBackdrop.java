package com.lingzidev.xiaochengyouxi;

import android.content.Context;
import android.graphics.*;
import android.os.SystemClock;
import android.view.View;
import java.util.Random;

/** Full-screen animated backdrop for the red-packet rain: glow, rotating rays, confetti, twinkles. Decorative only. */
final class RainBackdrop extends View {
  private static final int CONFETTI = 46, STARS = 26;
  private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
  private final Path ray = new Path();
  private final float density;
  private final float[][] confetti = new float[CONFETTI][7]; // x, y, speed, size, phase, spin, colorIndex
  private final float[][] stars = new float[STARS][4]; // x, y, size, phase
  private final int[] colors = {0xffffd166, 0xffffb43c, 0xffff7a45, 0xffffe9a8, 0xfff2493a, 0xffffffff};
  private final long born = SystemClock.uptimeMillis();
  private long last;
  private boolean animate = true;
  private Shader background;
  private RadialGradient glow;
  private int shaderW, shaderH;

  RainBackdrop(Context context) {
    super(context);
    density = context.getResources().getDisplayMetrics().density;
    Random r = new Random();
    for (float[] c : confetti) {
      c[0] = r.nextFloat();
      c[1] = r.nextFloat();
      c[2] = .05f + r.nextFloat() * .09f;
      c[3] = (3.5f + r.nextFloat() * 4.5f) * density;
      c[4] = r.nextFloat() * 6.28f;
      c[5] = (r.nextFloat() - .5f) * 9f;
      c[6] = r.nextInt(colors.length);
    }
    for (float[] s : stars) {
      s[0] = r.nextFloat();
      s[1] = r.nextFloat() * .75f;
      s[2] = (1.5f + r.nextFloat() * 2.5f) * density;
      s[3] = r.nextFloat() * 6.28f;
    }
  }

  void setAnimated(boolean value) {
    animate = value;
    invalidate();
  }

  @Override
  protected void onDraw(Canvas canvas) {
    int w = getWidth(), h = getHeight();
    if (w == 0 || h == 0) return;
    if (w != shaderW || h != shaderH) {
      shaderW = w;
      shaderH = h;
      background =
          new LinearGradient(0, 0, 0, h, new int[] {0xffb3261b, 0xfa8a1a12, 0xf54a0c08}, new float[] {0, .45f, 1}, Shader.TileMode.CLAMP);
      glow = new RadialGradient(w / 2f, h * .1f, w * .85f, new int[] {0x66ffcf6e, 0x22ff8a3d, 0x00ff8a3d}, new float[] {0, .5f, 1}, Shader.TileMode.CLAMP);
    }
    long now = SystemClock.uptimeMillis();
    float t = (now - born) / 1000f;
    float dt = last == 0 ? 0 : Math.min(.05f, (now - last) / 1000f);
    last = now;

    paint.setAlpha(255);
    paint.setColor(0xffffffff);
    paint.setShader(background);
    canvas.drawRect(0, 0, w, h, paint);
    paint.setShader(glow);
    canvas.drawRect(0, 0, w, h, paint);
    paint.setShader(null);

    // Slowly rotating light rays from the top centre.
    canvas.save();
    float cx = w / 2f, cy = h * .1f, len = h * 1.1f;
    canvas.rotate(t * 6f, cx, cy);
    paint.setColor(0x14fff0c0);
    for (int i = 0; i < 14; i++) {
      double a = i * Math.PI * 2 / 14, b = a + .11;
      ray.reset();
      ray.moveTo(cx, cy);
      ray.lineTo(cx + (float) Math.cos(a) * len, cy + (float) Math.sin(a) * len);
      ray.lineTo(cx + (float) Math.cos(b) * len, cy + (float) Math.sin(b) * len);
      ray.close();
      canvas.drawPath(ray, paint);
    }
    canvas.restore();

    // Twinkling stars.
    for (float[] s : stars) {
      float a = .35f + .65f * (float) Math.abs(Math.sin(t * 1.6f + s[3]));
      paint.setColor(Color.argb((int) (a * 230), 255, 236, 180));
      float x = s[0] * w, y = s[1] * h, r = s[2] * (.6f + a * .6f);
      canvas.drawCircle(x, y, r * .45f, paint);
      paint.setStrokeWidth(Math.max(1, r * .28f));
      canvas.drawLine(x - r * 1.6f, y, x + r * 1.6f, y, paint);
      canvas.drawLine(x, y - r * 1.6f, x, y + r * 1.6f, paint);
    }

    // Falling, spinning confetti.
    for (float[] c : confetti) {
      c[1] += c[2] * dt;
      if (c[1] > 1.05f) {
        c[1] = -.05f;
        c[0] = (float) Math.random();
      }
      float x = c[0] * w + (float) Math.sin(t * 1.3f + c[4]) * 14 * density, y = c[1] * h;
      float flip = (float) Math.cos(t * 3f + c[4]);
      canvas.save();
      canvas.translate(x, y);
      canvas.rotate(t * c[5] * 20f + c[4] * 57f);
      paint.setColor(colors[(int) c[6]]);
      paint.setAlpha(210);
      canvas.drawRoundRect(-c[3] / 2, -c[3] * .3f * Math.abs(flip) - 1, c[3] / 2, c[3] * .3f * Math.abs(flip) + 1, 2, 2, paint);
      canvas.restore();
    }
    if (animate) postInvalidateOnAnimation();
  }
}
