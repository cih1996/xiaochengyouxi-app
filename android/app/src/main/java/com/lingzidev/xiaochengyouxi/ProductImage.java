package com.lingzidev.xiaochengyouxi;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.util.Log;
import android.view.Gravity;
import android.view.View;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.TextView;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;

/** A product photo with a visible, retryable network error. */
final class ProductImage extends FrameLayout {
  private final String source;
  private final ImageView photo;
  private final TextView status;
  private volatile HttpURLConnection connection;
  private volatile int generation;
  private boolean attached, loaded, loading;

  ProductImage(Context context, String source, String description, ImageView.ScaleType scaleType) {
    super(context);
    this.source = source;
    setBackgroundColor(0xfffff2e8);
    photo = new ImageView(context);
    photo.setScaleType(scaleType);
    photo.setContentDescription(description);
    addView(photo, new LayoutParams(-1, -1));
    status = new TextView(context);
    status.setTextSize(11);
    status.setTextColor(0xffa38c7e);
    status.setGravity(Gravity.CENTER);
    status.setMaxLines(5);
    int padding = Math.round(10 * getResources().getDisplayMetrics().density);
    status.setPadding(padding, padding, padding, padding);
    status.setOnClickListener(v -> load());
    addView(status, new LayoutParams(-1, -1));
  }

  @Override
  protected void onAttachedToWindow() {
    super.onAttachedToWindow();
    attached = true;
    if (!loaded) load();
  }

  @Override
  protected void onDetachedFromWindow() {
    attached = false;
    generation++;
    loading = false;
    HttpURLConnection active = connection;
    if (active != null) active.disconnect();
    super.onDetachedFromWindow();
  }

  private void load() {
    if (!attached || loading) return;
    loading = true;
    status.setVisibility(View.VISIBLE);
    status.setText("图片加载中");
    status.setEnabled(false);
    int request = ++generation;
    new Thread(
            () -> {
              HttpURLConnection current = null;
              try {
                if (source.trim().isEmpty()) throw new IOException("商品图片地址为空");
                URL url =
                    new URL(source.startsWith("/") ? BuildConfig.API_BASE_URL + source : source);
                if (!source.startsWith("/") && !url.getProtocol().equals("https"))
                  throw new IOException("商品图片地址须使用 HTTPS");
                current = (HttpURLConnection) url.openConnection();
                connection = current;
                if (request != generation) return;
                current.setConnectTimeout(7000);
                current.setReadTimeout(12000);
                current.setRequestProperty("Accept", "image/*");
                int code = current.getResponseCode();
                if (code != 200) throw new IOException("HTTP " + code);
                ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                try (InputStream input = current.getInputStream()) {
                  byte[] buffer = new byte[8192];
                  int count;
                  while ((count = input.read(buffer)) != -1) {
                    if (request != generation) return;
                    bytes.write(buffer, 0, count);
                    if (bytes.size() > 20 * 1024 * 1024) throw new IOException("商品图片超过 20 MB");
                  }
                }
                byte[] encoded = bytes.toByteArray();
                BitmapFactory.Options bounds = new BitmapFactory.Options();
                bounds.inJustDecodeBounds = true;
                BitmapFactory.decodeByteArray(encoded, 0, encoded.length, bounds);
                if (bounds.outWidth <= 0 || bounds.outHeight <= 0)
                  throw new IOException("无法识别商品图片格式");
                BitmapFactory.Options options = new BitmapFactory.Options();
                options.inSampleSize = 1;
                while (bounds.outWidth / options.inSampleSize > 1200
                    || bounds.outHeight / options.inSampleSize > 1200) options.inSampleSize *= 2;
                Bitmap bitmap = BitmapFactory.decodeByteArray(encoded, 0, encoded.length, options);
                if (bitmap == null) throw new IOException("商品图片解码失败");
                post(
                    () -> {
                      if (!attached || request != generation) {
                        bitmap.recycle();
                        return;
                      }
                      loading = false;
                      loaded = true;
                      photo.setImageBitmap(bitmap);
                      status.setVisibility(View.GONE);
                    });
              } catch (Exception | OutOfMemoryError error) {
                if (request != generation) return;
                Log.e("ProductImage", "商品图片加载失败: " + source, error);
                String message = error.getMessage();
                if (message == null || message.trim().isEmpty())
                  message = error.getClass().getSimpleName();
                String detail = message;
                post(
                    () -> {
                      if (!attached || request != generation) return;
                      loading = false;
                      status.setEnabled(true);
                      status.setText("图片加载失败\n" + detail + "\n点击重试");
                    });
              } finally {
                if (current != null) current.disconnect();
                if (connection == current) connection = null;
              }
            },
            "product-photo")
        .start();
  }
}
