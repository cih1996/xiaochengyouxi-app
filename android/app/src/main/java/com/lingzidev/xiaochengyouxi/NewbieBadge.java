package com.lingzidev.xiaochengyouxi;

import android.animation.Animator;
import android.animation.ObjectAnimator;
import android.animation.PropertyValuesHolder;
import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.LinearGradient;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.RadialGradient;
import android.graphics.RectF;
import android.graphics.Shader;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.animation.LinearInterpolator;
import android.view.animation.OvershootInterpolator;
import android.widget.FrameLayout;
import android.widget.TextView;
import java.util.ArrayList;

/**
 * Hall floating badge for 新人礼包: rotating golden rays, a red envelope that shakes every couple
 * of seconds, a gold 開 coin, twinkling sparkles, a pulsing label and a count of packets to open.
 * Everything is drawn in code so it stays sharp at any density.
 */
final class NewbieBadge extends FrameLayout {
  private final ArrayList<Animator> animators = new ArrayList<>();
  private final TextView label, count;
  private final View rays, envelope;
  private final boolean wheel;

  NewbieBadge(Context context, String text) {
    this(context, text, false);
  }

  /** wheel=true: a spinning mini lucky wheel instead of the red envelope (幸运转盘). */
  NewbieBadge(Context context, String text, boolean wheel) {
    super(context);
    this.wheel = wheel;
    setClipChildren(false);
    setClipToPadding(false);
    setContentDescription(text);

    rays = new Rays(context);
    addView(rays, new LayoutParams(dp(96), dp(96), Gravity.TOP | Gravity.CENTER_HORIZONTAL));
    LayoutParams rp = (LayoutParams) rays.getLayoutParams();
    rp.topMargin = -dp(14);

    envelope = wheel ? new MiniWheel(context) : new Envelope(context);
    LayoutParams ep =
        wheel
            ? new LayoutParams(dp(62), dp(62), Gravity.TOP | Gravity.CENTER_HORIZONTAL)
            : new LayoutParams(dp(50), dp(62), Gravity.TOP | Gravity.CENTER_HORIZONTAL);
    ep.topMargin = dp(4);
    addView(envelope, ep);
    if (wheel) {
      View pointer = new Pointer(context);
      LayoutParams pp = new LayoutParams(dp(16), dp(20), Gravity.TOP | Gravity.CENTER_HORIZONTAL);
      pp.topMargin = 0;
      addView(pointer, pp);
    }

    label = new TextView(context);
    label.setText(text);
    label.setTextSize(TypedValue.COMPLEX_UNIT_SP, 12);
    label.setTypeface(Typeface.DEFAULT_BOLD);
    label.setTextColor(0xff8a1a00);
    label.setGravity(Gravity.CENTER);
    label.setSingleLine(true);
    label.setIncludeFontPadding(false);
    label.setPadding(dp(8), dp(4), dp(8), dp(4));
    GradientDrawable pill =
        new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM, new int[] {0xfffff3b0, 0xffffcf3a, 0xfff5a300});
    pill.setCornerRadius(dp(12));
    pill.setStroke(dp(2), Color.WHITE);
    label.setBackground(pill);
    LayoutParams lp = new LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
    addView(label, lp);

    count = new TextView(context);
    count.setTextSize(TypedValue.COMPLEX_UNIT_SP, 11);
    count.setTypeface(Typeface.DEFAULT_BOLD);
    count.setTextColor(Color.WHITE);
    count.setGravity(Gravity.CENTER);
    count.setIncludeFontPadding(false);
    GradientDrawable dot = new GradientDrawable();
    dot.setCornerRadius(dp(10));
    dot.setColor(0xffe8141d);
    dot.setStroke(dp(2), Color.WHITE);
    count.setBackground(dot);
    count.setMinWidth(dp(20));
    count.setPadding(dp(4), 0, dp(4), 0);
    LayoutParams cp = new LayoutParams(-2, dp(20), Gravity.TOP | Gravity.RIGHT);
    cp.rightMargin = dp(6);
    addView(count, cp);
    count.setVisibility(GONE);
  }

  void setCount(int n) {
    count.setText(n > 99 ? "99+" : String.valueOf(n));
    count.setVisibility(n > 0 ? VISIBLE : GONE);
  }

  /** Entrance pop, then the idle loops. */
  void start() {
    stop();
    if (!ValueAnimator.areAnimatorsEnabled()) return;
    setScaleX(0f);
    setScaleY(0f);
    ObjectAnimator in =
        ObjectAnimator.ofPropertyValuesHolder(
            this,
            PropertyValuesHolder.ofFloat(SCALE_X, 0f, 1f),
            PropertyValuesHolder.ofFloat(SCALE_Y, 0f, 1f));
    in.setInterpolator(new OvershootInterpolator(2.4f));
    in.setDuration(520);
    play(in);

    ObjectAnimator spin = ObjectAnimator.ofFloat(rays, ROTATION, 0f, 360f);
    spin.setDuration(9000);
    spin.setInterpolator(new LinearInterpolator());
    spin.setRepeatCount(ValueAnimator.INFINITE);
    play(spin);

    ObjectAnimator glow = ObjectAnimator.ofFloat(rays, ALPHA, .55f, 1f);
    glow.setDuration(900);
    glow.setRepeatMode(ValueAnimator.REVERSE);
    glow.setRepeatCount(ValueAnimator.INFINITE);
    play(glow);

    if (wheel) {
      // Spins up, slows down, rests — like someone just gave it a go.
      ObjectAnimator turn = ObjectAnimator.ofFloat(envelope, ROTATION, 0f, 1080f);
      turn.setDuration(3200);
      turn.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
      turn.setRepeatCount(ValueAnimator.INFINITE);
      turn.setStartDelay(300);
      play(turn);
    } else {
    // Shake + hop every ~2.2s: still most of the time, then a burst of motion.
    ObjectAnimator shake =
        ObjectAnimator.ofPropertyValuesHolder(
            envelope,
            PropertyValuesHolder.ofFloat(ROTATION, 0, 0, 0, 0, -14, 14, -10, 10, -5, 0),
            PropertyValuesHolder.ofFloat(TRANSLATION_Y, 0, 0, 0, 0, -dp(6), -dp(8), -dp(6), -dp(3), 0, 0),
            PropertyValuesHolder.ofFloat(SCALE_X, 1, 1, 1, 1, 1.12f, 1.12f, 1.08f, 1.04f, 1, 1),
            PropertyValuesHolder.ofFloat(SCALE_Y, 1, 1, 1, 1, 1.12f, 1.12f, 1.08f, 1.04f, 1, 1));
    shake.setDuration(2200);
    shake.setRepeatCount(ValueAnimator.INFINITE);
    play(shake);
    }

    ObjectAnimator pulse =
        ObjectAnimator.ofPropertyValuesHolder(
            label,
            PropertyValuesHolder.ofFloat(SCALE_X, 1f, 1.1f),
            PropertyValuesHolder.ofFloat(SCALE_Y, 1f, 1.1f));
    pulse.setDuration(650);
    pulse.setRepeatMode(ValueAnimator.REVERSE);
    pulse.setRepeatCount(ValueAnimator.INFINITE);
    play(pulse);

    ObjectAnimator beat =
        ObjectAnimator.ofPropertyValuesHolder(
            count,
            PropertyValuesHolder.ofFloat(SCALE_X, 1f, 1.25f, 1f),
            PropertyValuesHolder.ofFloat(SCALE_Y, 1f, 1.25f, 1f));
    beat.setDuration(800);
    beat.setRepeatCount(ValueAnimator.INFINITE);
    play(beat);
  }

  void stop() {
    for (Animator a : animators) a.cancel();
    animators.clear();
    setScaleX(1f);
    setScaleY(1f);
  }

  @Override
  protected void onDetachedFromWindow() {
    stop();
    super.onDetachedFromWindow();
  }

  private void play(Animator a) {
    animators.add(a);
    a.start();
  }

  private int dp(float v) {
    return Math.round(TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, v, getResources().getDisplayMetrics()));
  }

  /** Golden sunburst with twinkling sparkles. */
  private static final class Rays extends View {
    private final Paint ray = new Paint(Paint.ANTI_ALIAS_FLAG), star = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Path path = new Path();
    private float twinkle;

    Rays(Context c) {
      super(c);
      star.setColor(0xffffffff);
      ValueAnimator t = ValueAnimator.ofFloat(0f, 1f);
      t.setDuration(1400);
      t.setRepeatCount(ValueAnimator.INFINITE);
      t.addUpdateListener(
          a -> {
            twinkle = (float) a.getAnimatedValue();
            invalidate();
          });
      addOnAttachStateChangeListener(
          new OnAttachStateChangeListener() {
            @Override
            public void onViewAttachedToWindow(View v) {
              if (ValueAnimator.areAnimatorsEnabled()) t.start();
            }

            @Override
            public void onViewDetachedFromWindow(View v) {
              t.cancel();
            }
          });
    }

    @Override
    protected void onDraw(Canvas canvas) {
      float cx = getWidth() / 2f, cy = getHeight() / 2f, r = Math.min(cx, cy);
      ray.setShader(
          new RadialGradient(cx, cy, r, new int[] {0xccffd75a, 0x55ffb020, 0x00ffb020}, new float[] {0f, .55f, 1f}, Shader.TileMode.CLAMP));
      canvas.drawCircle(cx, cy, r * .62f, ray);
      for (int i = 0; i < 12; i++) {
        double a = Math.PI * 2 * i / 12, w = Math.PI / 30;
        path.reset();
        path.moveTo(cx, cy);
        path.lineTo(cx + (float) Math.cos(a - w) * r, cy + (float) Math.sin(a - w) * r);
        path.lineTo(cx + (float) Math.cos(a + w) * r, cy + (float) Math.sin(a + w) * r);
        path.close();
        canvas.drawPath(path, ray);
      }
      // Three sparkles, each fading in and out on its own phase (counter-rotated so they stay put
      // is unnecessary — rotating sparkles read as glitter).
      float[][] spots = {{.18f, .26f}, {.84f, .34f}, {.26f, .82f}};
      for (int i = 0; i < spots.length; i++) {
        float phase = (twinkle + i / 3f) % 1f, alpha = (float) Math.sin(phase * Math.PI);
        star.setAlpha((int) (255 * alpha));
        drawStar(canvas, getWidth() * spots[i][0], getHeight() * spots[i][1], r * .1f * (.6f + alpha * .6f));
      }
    }

    private void drawStar(Canvas c, float x, float y, float s) {
      path.reset();
      path.moveTo(x, y - s);
      path.quadTo(x, y, x + s, y);
      path.quadTo(x, y, x, y + s);
      path.quadTo(x, y, x - s, y);
      path.quadTo(x, y, x, y - s);
      c.drawPath(path, star);
    }
  }

  /** Mini lucky wheel: gold rim, alternating red/cream slices, gold hub. */
  private static final class MiniWheel extends View {
    private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF oval = new RectF();

    MiniWheel(Context c) {
      super(c);
    }

    @Override
    protected void onDraw(Canvas canvas) {
      float cx = getWidth() / 2f, cy = getHeight() / 2f, r = Math.min(cx, cy);
      p.setColor(0xff000000);
      p.setShader(new RadialGradient(cx, cy, r, new int[] {0xffb86a00, 0xffffd34d, 0xfffff1a8, 0xffc27300}, new float[] {.78f, .86f, .93f, 1f}, Shader.TileMode.CLAMP));
      canvas.drawCircle(cx, cy, r, p);
      p.setShader(null);
      float ir = r * .82f;
      oval.set(cx - ir, cy - ir, cx + ir, cy + ir);
      for (int i = 0; i < 8; i++) {
        p.setColor(i % 2 == 0 ? 0xffe8262b : 0xfffff1d0);
        canvas.drawArc(oval, i * 45f - 112.5f, 45f, true, p);
      }
      p.setColor(0x66ffd27a);
      p.setStyle(Paint.Style.STROKE);
      p.setStrokeWidth(r * .04f);
      canvas.drawCircle(cx, cy, ir, p);
      p.setStyle(Paint.Style.FILL);
      p.setColor(0xff000000);
      p.setShader(new RadialGradient(cx - r * .08f, cy - r * .08f, r * .3f, new int[] {0xfffff6c8, 0xffffc531, 0xffc27300}, new float[] {0f, .5f, 1f}, Shader.TileMode.CLAMP));
      canvas.drawCircle(cx, cy, r * .24f, p);
      p.setShader(null);
      // rim bulbs
      p.setColor(0xfffff6c8);
      for (int i = 0; i < 12; i++) {
        double a = Math.PI * 2 * i / 12;
        canvas.drawCircle(cx + (float) Math.cos(a) * r * .91f, cy + (float) Math.sin(a) * r * .91f, r * .045f, p);
      }
    }
  }

  /** The red pointer that sits above the mini wheel (does not spin). */
  private static final class Pointer extends View {
    private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Path path = new Path();

    Pointer(Context c) {
      super(c);
    }

    @Override
    protected void onDraw(Canvas canvas) {
      float w = getWidth(), h = getHeight();
      path.reset();
      path.moveTo(w / 2, h);
      path.lineTo(w * .08f, h * .35f);
      path.quadTo(w / 2, -h * .15f, w * .92f, h * .35f);
      path.close();
      p.setColor(0xffffd96a);
      canvas.drawPath(path, p);
      canvas.save();
      canvas.scale(.72f, .72f, w / 2, h * .5f);
      p.setColor(0xffe8262b);
      canvas.drawPath(path, p);
      canvas.restore();
    }
  }

  /** Red envelope with a gold 開 coin. */
  private static final class Envelope extends View {
    private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG), text = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF rect = new RectF();
    private final Path flap = new Path();

    Envelope(Context c) {
      super(c);
      text.setTypeface(Typeface.DEFAULT_BOLD);
      text.setTextAlign(Paint.Align.CENTER);
      text.setColor(0xff8a2a00);
    }

    @Override
    protected void onDraw(Canvas canvas) {
      float w = getWidth(), h = getHeight(), r = w * .2f;
      p.setShader(null);
      p.setColor(0x33000000);
      rect.set(w * .04f, h * .06f, w, h);
      canvas.drawRoundRect(rect, r, r, p);
      // the envelope itself, slightly inset so the shadow shows on the lower right
      w = w * .96f;
      h = h * .95f;
      rect.set(0, 0, w, h);
      p.setColor(0xff000000); // opaque: setColor(0x33…) below would otherwise stick
      p.setShader(new LinearGradient(0, 0, w, h, 0xffff5a45, 0xffc4141c, Shader.TileMode.CLAMP));
      canvas.drawRoundRect(rect, r, r, p);
      flap.reset();
      flap.moveTo(0, r);
      flap.quadTo(0, 0, r, 0);
      flap.lineTo(w - r, 0);
      flap.quadTo(w, 0, w, r);
      flap.lineTo(w, h * .36f);
      flap.quadTo(w / 2, h * .62f, 0, h * .36f);
      flap.close();
      p.setColor(0xff000000); // opaque: setColor(0x33…) below would otherwise stick
      p.setShader(new LinearGradient(0, 0, 0, h * .5f, 0xffff7a5c, 0xffe8362c, Shader.TileMode.CLAMP));
      canvas.drawPath(flap, p);
      float cx = w / 2, cy = h * .46f, cr = w * .25f;
      p.setColor(0xff000000);
      p.setShader(
          new RadialGradient(cx - cr * .35f, cy - cr * .35f, cr * 1.3f, new int[] {0xfffff6c8, 0xffffc531, 0xffc27300}, new float[] {0f, .45f, 1f}, Shader.TileMode.CLAMP));
      canvas.drawCircle(cx, cy, cr, p);
      text.setTextSize(cr * 1.15f);
      Paint.FontMetrics m = text.getFontMetrics();
      canvas.drawText("開", cx, cy - (m.ascent + m.descent) / 2, text);
      p.setShader(null);
      p.setColor(0x33ffffff);
      canvas.drawRoundRect(new RectF(w * .12f, h * .7f, w * .88f, h * .74f), 4, 4, p);
    }
  }
}
