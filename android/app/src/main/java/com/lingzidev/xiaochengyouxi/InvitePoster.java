package com.lingzidev.xiaochengyouxi;

import android.content.Context;
import android.graphics.*;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

final class InvitePoster {
  static final int WIDTH = 1080, HEIGHT = 1600;
  private static final int RED = 0xfff65331, INK = 0xff392923, MUTED = 0xffa67e64;

  static Bitmap render(Context context, JSONObject summary, JSONObject qr) throws JSONException {
    Bitmap bitmap = Bitmap.createBitmap(WIDTH, HEIGHT, Bitmap.Config.ARGB_8888);
    Canvas canvas = new Canvas(bitmap);
    Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG | Paint.FILTER_BITMAP_FLAG);
    paint.setShader(
        new LinearGradient(
            0,
            0,
            WIDTH,
            HEIGHT,
            new int[] {0xfffff9ef, 0xffffdfc3, 0xfffff9f2},
            null,
            Shader.TileMode.CLAMP));
    canvas.drawRect(0, 0, WIDTH, HEIGHT, paint);
    paint.setShader(null);
    paint.setColor(0x66ffffff);
    canvas.drawCircle(900, 115, 330, paint);
    canvas.drawCircle(125, 650, 240, paint);
    paint.setColor(RED);
    canvas.drawRoundRect(72, 72, 126, 126, 17, 17, paint);
    drawText(canvas, paint, "橙", 99, 111, 34, Color.WHITE, true, Paint.Align.CENTER);
    drawText(canvas, paint, "小橙有喜", 146, 111, 34, INK, true, Paint.Align.LEFT);
    drawText(canvas, paint, "把小欢喜，", 74, 254, 70, INK, true, Paint.Align.LEFT);
    drawText(canvas, paint, "分享给朋友", 74, 348, 78, RED, true, Paint.Align.LEFT);
    drawText(canvas, paint, "一起领福利 · 一起发现好物", 78, 414, 29, MUTED, false, Paint.Align.LEFT);

    drawArt(canvas, paint, context, "mascot", new RectF(500, 355, 1035, 850));
    drawArt(canvas, paint, context, "packet", new RectF(88, 465, 555, 855));
    paint.setColor(0xfff6bb5e);
    canvas.drawCircle(885, 450, 9, paint);
    canvas.drawCircle(122, 450, 7, paint);
    canvas.drawCircle(928, 770, 13, paint);
    paint.setColor(Color.WHITE);
    canvas.drawRoundRect(72, 840, 1008, 1013, 34, 34, paint);
    drawText(canvas, paint, "我的邀请码", WIDTH / 2f, 888, 27, MUTED, false, Paint.Align.CENTER);
    drawText(
        canvas,
        paint,
        summary.getString("invite_code"),
        WIDTH / 2f,
        969,
        65,
        INK,
        true,
        Paint.Align.CENTER);

    paint.setColor(Color.WHITE);
    canvas.drawRoundRect(72, 1040, 1008, 1536, 38, 38, paint);
    int size = qr.getInt("size");
    JSONArray rows = qr.getJSONArray("modules");
    if (rows.length() != size) throw new IllegalArgumentException("二维码矩阵尺寸不一致");
    QrLayout grid = new QrLayout(size, 392);
    int left = (WIDTH - grid.pixels) / 2, top = 1060 + (400 - grid.pixels) / 2;
    paint.setAntiAlias(false);
    paint.setColor(Color.WHITE);
    canvas.drawRect(left, top, left + grid.pixels, top + grid.pixels, paint);
    paint.setColor(INK);
    for (int y = 0; y < size; y++) {
      String row = rows.getString(y);
      if (row.length() != size) throw new IllegalArgumentException("二维码矩阵行长度错误");
      for (int x = 0; x < size; x++) {
        char module = row.charAt(x);
        if (module == '1') {
          int px = left + grid.position(x), py = top + grid.position(y);
          canvas.drawRect(px, py, px + grid.cell, py + grid.cell, paint);
        } else if (module != '0') throw new IllegalArgumentException("二维码矩阵内容错误");
      }
    }
    paint.setAntiAlias(true);
    drawText(
        canvas, paint, "扫码输入手机号，加入福利大厅", WIDTH / 2f, 1500, 28, MUTED, false, Paint.Align.CENTER);
    return bitmap;
  }

  private static void drawArt(
      Canvas canvas, Paint paint, Context context, String name, RectF rect) {
    int resource = context.getResources().getIdentifier(name, "drawable", context.getPackageName());
    Bitmap image = BitmapFactory.decodeResource(context.getResources(), resource);
    float scale = Math.min(rect.width() / image.getWidth(), rect.height() / image.getHeight());
    float width = image.getWidth() * scale, height = image.getHeight() * scale;
    RectF target =
        new RectF(
            rect.centerX() - width / 2,
            rect.centerY() - height / 2,
            rect.centerX() + width / 2,
            rect.centerY() + height / 2);
    canvas.drawBitmap(image, null, target, paint);
  }

  private static void drawText(
      Canvas canvas,
      Paint paint,
      String text,
      float x,
      float y,
      float size,
      int color,
      boolean bold,
      Paint.Align align) {
    paint.setColor(color);
    paint.setTextSize(size);
    paint.setTypeface(Typeface.create("sans-serif", bold ? Typeface.BOLD : Typeface.NORMAL));
    paint.setTextAlign(align);
    canvas.drawText(text, x, y, paint);
  }
}
