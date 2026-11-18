package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ComponentName;
import android.content.ContentValues;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.Environment;
import android.provider.MediaStore;
import android.util.Base64;
import android.widget.Toast;
import androidx.core.content.FileProvider;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStream;

/**
 * Shares an activity poster (PNG drawn by the H5 page) without the WeChat SDK: friends through
 * WeChat's own share screen; Moments by saving to the album and opening WeChat, because WeChat no
 * longer accepts images for Moments from third-party intents.
 */
final class PosterShare {
  static final String WECHAT = "com.tencent.mm";
  private static final int MAX_BYTES = 8 * 1024 * 1024;

  private PosterShare() {}

  static void share(Activity activity, String target, String base64, String text) {
    byte[] png;
    try {
      png = Base64.decode(base64, Base64.DEFAULT);
    } catch (IllegalArgumentException e) {
      toast(activity, "海报生成失败，请重试");
      return;
    }
    if (png.length < 8 || png.length > MAX_BYTES || png[0] != (byte) 0x89 || png[1] != 'P') {
      toast(activity, "海报生成失败，请重试");
      return;
    }
    share(activity, target, png, text);
  }

  /** Same as above for a poster drawn natively. */
  static void share(Activity activity, String target, android.graphics.Bitmap poster, String text) {
    java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
    poster.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, out);
    share(activity, target, out.toByteArray(), text);
  }

  static void share(Activity activity, String target, byte[] png, String text) {
    try {
      switch (target) {
        case "wechat":
          toWeChatFriend(activity, png, text);
          break;
        case "moments":
          saveToAlbum(activity, png);
          if (openWeChat(activity))
            toast(activity, "海报已保存到相册，在朋友圈点“+”选择这张图片发布", true);
          else toast(activity, "海报已保存到相册，打开微信朋友圈发布即可", true);
          break;
        default:
          saveToAlbum(activity, png);
          toast(activity, "海报已保存到相册");
      }
    } catch (IOException | RuntimeException e) {
      toast(activity, "操作失败：" + e.getMessage());
    }
  }

  private static void toWeChatFriend(Activity activity, byte[] png, String text) throws IOException {
    File dir = new File(activity.getCacheDir(), "invite-posters");
    if (!dir.isDirectory() && !dir.mkdirs()) throw new IOException("海报目录创建失败");
    File file = new File(dir, "newbie-poster.png");
    try (OutputStream out = new FileOutputStream(file)) {
      out.write(png);
    }
    Uri uri = FileProvider.getUriForFile(activity, activity.getPackageName() + ".invite-files", file);
    Intent send = new Intent(Intent.ACTION_SEND);
    send.setType("image/png");
    send.putExtra(Intent.EXTRA_STREAM, uri);
    send.setClipData(ClipData.newRawUri("邀请海报", uri));
    send.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
    if (installed(activity)) {
      Intent wx = new Intent(send).setComponent(new ComponentName(WECHAT, "com.tencent.mm.ui.tools.ShareImgUI"));
      try {
        activity.startActivity(wx);
        return;
      } catch (ActivityNotFoundException | SecurityException ignored) {
        // WeChat moved its share screen: fall back to the system chooser below.
      }
    } else {
      toast(activity, "未检测到微信，可选择其他方式发送");
    }
    if (text != null && !text.isEmpty()) send.putExtra(Intent.EXTRA_TEXT, text);
    activity.startActivity(Intent.createChooser(send, "发送给好友"));
  }

  static boolean installed(Activity activity) {
    return activity.getPackageManager().getLaunchIntentForPackage(WECHAT) != null;
  }

  private static boolean openWeChat(Activity activity) {
    Intent launch = activity.getPackageManager().getLaunchIntentForPackage(WECHAT);
    if (launch == null) return false;
    try {
      activity.startActivity(launch);
      return true;
    } catch (ActivityNotFoundException e) {
      return false;
    }
  }

  static void saveToAlbum(Activity activity, byte[] png) throws IOException {
    String name = "小橙有喜-红包海报-" + System.currentTimeMillis() + ".png";
    if (Build.VERSION.SDK_INT >= 29) {
      ContentValues values = new ContentValues();
      values.put(MediaStore.Images.Media.DISPLAY_NAME, name);
      values.put(MediaStore.Images.Media.MIME_TYPE, "image/png");
      values.put(MediaStore.Images.Media.RELATIVE_PATH, Environment.DIRECTORY_PICTURES + "/小橙有喜");
      values.put(MediaStore.Images.Media.IS_PENDING, 1);
      Uri uri = activity.getContentResolver().insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values);
      if (uri == null) throw new IOException("相册图片创建失败");
      try (OutputStream out = activity.getContentResolver().openOutputStream(uri)) {
        if (out == null) throw new IOException("相册图片写入失败");
        out.write(png);
      }
      values.clear();
      values.put(MediaStore.Images.Media.IS_PENDING, 0);
      activity.getContentResolver().update(uri, values, null, null);
    } else {
      // Android 9 and below: app-specific pictures folder (no storage permission requested).
      File dir = activity.getExternalFilesDir(Environment.DIRECTORY_PICTURES);
      if (dir == null) throw new IOException("图片目录不可用");
      try (OutputStream out = new FileOutputStream(new File(dir, name))) {
        out.write(png);
      }
    }
  }

  private static void toast(Activity activity, String text) {
    toast(activity, text, false);
  }

  private static void toast(Activity activity, String text, boolean longer) {
    activity.runOnUiThread(
        () -> Toast.makeText(activity, text, longer ? Toast.LENGTH_LONG : Toast.LENGTH_SHORT).show());
  }
}
