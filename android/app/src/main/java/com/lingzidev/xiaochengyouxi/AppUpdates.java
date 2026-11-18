package com.lingzidev.xiaochengyouxi;

import android.animation.ObjectAnimator;
import android.animation.ValueAnimator;
import android.app.Activity;
import android.app.Dialog;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.SystemClock;
import android.provider.Settings;
import android.util.Log;
import android.view.Gravity;
import android.view.View;
import android.widget.*;
import androidx.core.content.FileProvider;
import java.io.*;
import java.net.URL;
import java.security.MessageDigest;
import java.util.Arrays;
import javax.net.ssl.HttpsURLConnection;
import org.json.*;

final class AppUpdates {
  private static final String TAG = "AppUpdate";
  private static final int INSTALL = 9071;
  private final MainActivity app;
  private final Api api;
  private Info info;
  private Dialog dialog;
  private ProgressBar progress;
  private TextView progressText;
  private ObjectAnimator artMotion;
  private Runnable startup;
  private boolean checking,
      manualCheck,
      downloading,
      ready,
      awaitingSettings,
      installing,
      destroyed,
      // a startup check already finished (prefetched while the rest of startup was loading)
      completed;
  private boolean manualOffer;
  private boolean forceGate;
  private String manualError;
  private String pendingErrorTitle, pendingErrorMessage;
  private Runnable pendingErrorRetry;
  private long lastCheck;
  private volatile boolean cancelDownload;
  private volatile HttpsURLConnection connection;

  AppUpdates(MainActivity app, Api api, Bundle state) {
    this.app = app;
    this.api = api;
    String saved = api.preferences.getString("required_update", "");
    if (state != null && state.containsKey("update_info")) {
      saved = state.getString("update_info", "");
      ready = state.getBoolean("update_ready");
      awaitingSettings = state.getBoolean("update_settings");
    }
    if (!saved.isEmpty()) {
      try {
        Info previous = new Info(new JSONObject(saved));
        if (previous.version > BuildConfig.VERSION_CODE) info = previous;
      } catch (JSONException error) {
        Log.e(TAG, "Stored update metadata: " + error.getMessage());
      }
    }
    forceGate =
        api.preferences.getInt("required_min_version", 0) > BuildConfig.VERSION_CODE
            || (info != null && info.required);
    api.updateRequired = forceGate;
  }

  /** Starts the update check early, in parallel with the rest of startup. */
  void prefetch() {
    if (!checking && !completed) check(false);
  }

  void start(Runnable continuation) {
    startup = continuation;
    if (checking) return; // the prefetched check continues startup when it answers
    if (completed) {
      maybePresent();
      return;
    }
    check(false);
  }

  void manual() {
    if (downloading) {
      presentDownload();
      return;
    }
    check(true);
  }

  void check(boolean manual) {
    if (manual) {
      manualError = null;
      closeDialog();
    }
    manualCheck |= manual;
    if (checking || destroyed) return;
    checking = true;
    lastCheck = SystemClock.elapsedRealtime();
    api.get(
        "/app/update?version_code=" + BuildConfig.VERSION_CODE,
        result -> {
          if (destroyed) return;
          checking = false;
          completed = true;
          boolean requested = manualCheck;
          manualCheck = false;
          try {
            JSONObject value = (JSONObject) result;
            forceGate = value.getBoolean("enabled") && value.getBoolean("force_update");
            api.updateRequired = forceGate;
            if (forceGate)
              api.preferences
                  .edit()
                  .putInt("required_min_version", value.getInt("min_supported_version_code"))
                  .apply();
            else api.preferences.edit().remove("required_min_version").apply();
            if (!value.getBoolean("enabled") || !value.getBoolean("update_available")) {
              if (value.getBoolean("enabled") && value.getBoolean("force_update"))
                throw new JSONException("更新配置缺少可用版本");
              info = null;
              ready = false;
              awaitingSettings = false;
              api.preferences.edit().remove("required_update").apply();
              if (requested) app.tip("当前已是最新版本");
              continueStartup();
              return;
            }
            Info next = new Info(value);
            if (next.version <= BuildConfig.VERSION_CODE) throw new JSONException("更新版本号与当前版本不匹配");
            boolean same =
                info != null && info.version == next.version && info.sha.equals(next.sha);
            if (!same) {
              ready = false;
              awaitingSettings = false;
            }
            info = next;
            api.preferences
                .edit()
                .putString("required_update", info.required ? info.raw.toString() : "")
                .apply();
            manualOffer = requested;
            if (awaitingSettings) resume();
            else maybePresent();
          } catch (JSONException error) {
            checkFailed(error.getMessage(), requested);
          }
        },
        message -> {
          if (destroyed) return;
          checking = false;
          completed = true;
          boolean requested = manualCheck;
          manualCheck = false;
          checkFailed(message, requested);
        });
  }

  private void checkFailed(String message, boolean manual) {
    Log.e(TAG, "GET /api/app/update version=" + BuildConfig.VERSION_CODE + ": " + message);
    if (manual) {
      manualError = message;
      maybePresent();
    } else if (required()) {
      manualError = message;
      maybePresent();
    } else continueStartup();
  }

  void resume() {
    if (!app.servicesReady || destroyed) return;
    if (awaitingSettings && info != null) {
      awaitingSettings = false;
      if (Build.VERSION.SDK_INT < 26 || app.getPackageManager().canRequestPackageInstalls()) {
        verifyReadyAndInstall();
      } else showError("安装授权未开启", "请允许安装应用后继续。", this::verifyReadyAndInstall);
      return;
    }
    maybePresent();
    if (dialog != null && artMotion != null) artMotion.start();
    if (!downloading
        && !installing
        && dialog == null
        && SystemClock.elapsedRealtime() - lastCheck >= 60000) check(false);
  }

  void maybePresent() {
    if (destroyed
        || checking
        || downloading
        || installing
        || dialog != null
        || !app.canPresentUpdate()) return;
    if (manualError != null) {
      showError("检查更新失败", manualError, () -> check(true));
      return;
    }
    if (pendingErrorRetry != null) {
      showError(pendingErrorTitle, pendingErrorMessage, pendingErrorRetry);
      return;
    }
    if (awaitingSettings) return;
    if (info == null) {
      continueStartup();
      return;
    }
    if (!info.required
        && !manualOffer
        && api.preferences.getInt("dismissed_update", 0) == info.version) {
      continueStartup();
      return;
    }
    showOffer();
  }

  boolean required() {
    return forceGate;
  }

  private void continueStartup() {
    if (startup == null) return;
    Runnable next = startup;
    startup = null;
    next.run();
  }

  private LinearLayout card(String title) {
    closeDialog();
    LinearLayout card = app.col();
    card.setPadding(app.dp(23), app.dp(23), app.dp(23), app.dp(17));
    card.setBackground(app.grad(0xffffeddb, 0xfffffaf4, 27));
    LinearLayout hero = app.row();
    ImageView mascot = app.art("mascot");
    app.add(hero, mascot, 76, 76);
    LinearLayout copy = app.col();
    copy.setPadding(app.dp(10), 0, 0, 0);
    app.add(copy, app.text(title, 22, MainActivity.INK, true), -1, -2);
    if (info != null) {
      app.gap(copy, 7);
      app.add(copy, app.text("小橙有喜 " + info.name, 13, MainActivity.RED, true), -1, -2);
    }
    app.flex(hero, copy);
    app.add(card, hero, -1, 76);
    app.gap(card, 12);
    if (Build.VERSION.SDK_INT < 26 || ValueAnimator.areAnimatorsEnabled()) {
      artMotion = ObjectAnimator.ofFloat(mascot, View.TRANSLATION_Y, 0, -app.dp(4), 0);
      artMotion.setDuration(2400);
      artMotion.setRepeatCount(ValueAnimator.INFINITE);
    }
    return card;
  }

  private void show(LinearLayout card) {
    dialog = app.sheet(card);
    dialog.setCancelable(false);
    dialog.setCanceledOnTouchOutside(false);
    dialog.show();
    dialog
        .getWindow()
        .setLayout(
            Math.min(app.getResources().getDisplayMetrics().widthPixels - app.dp(40), app.dp(370)),
            -2);
    if (artMotion != null) artMotion.start();
  }

  private void secondary(LinearLayout card) {
    TextView later = app.label(required() ? "退出应用" : "稍后再说", 13, MainActivity.MUTED);
    later.setGravity(Gravity.CENTER);
    later.setOnClickListener(
        v -> {
          closeDialog();
          if (required()) app.finish();
          else {
            if (info != null)
              api.preferences.edit().putInt("dismissed_update", info.version).apply();
            manualOffer = false;
            manualError = null;
            continueStartup();
          }
        });
    app.add(card, later, -1, 40);
  }

  private void showOffer() {
    LinearLayout card = card(info.title.isEmpty() ? "小欢喜有更新" : info.title);
    ScrollView notes = new ScrollView(app);
    notes.setFillViewport(false);
    notes.addView(app.label(info.notes, 14, 0xff8e6d59));
    app.add(card, notes, -1, 106);
    app.gap(card, 18);
    app.add(
        card,
        app.button(ready ? "继续安装" : "立即更新", ready ? this::verifyReadyAndInstall : this::download),
        -1,
        52);
    secondary(card);
    show(card);
  }

  private void showError(String title, String message, Runnable retry) {
    if (destroyed) return;
    if (!app.foreground) {
      closeDialog();
      pendingErrorTitle = title;
      pendingErrorMessage = message;
      pendingErrorRetry = retry;
      return;
    }
    pendingErrorTitle = null;
    pendingErrorMessage = null;
    pendingErrorRetry = null;
    LinearLayout card = card(title);
    TextView description = app.label(message, 13, MainActivity.MUTED);
    description.setMaxLines(7);
    app.add(card, description, -1, -2);
    app.gap(card, 20);
    app.add(card, app.button("重试", retry), -1, 52);
    secondary(card);
    show(card);
  }

  private File file(Info version) {
    return new File(
        new File(app.getCacheDir(), "updates"), "xiaochengyouxi-" + version.version + ".apk");
  }

  private void download() {
    if (downloading || info == null) return;
    downloading = true;
    cancelDownload = false;
    Info selected = info;
    presentDownload();
    new Thread(
            () -> {
              File target = file(selected),
                  partial =
                      new File(target.getPath() + "." + java.util.UUID.randomUUID() + ".part");
              boolean cached = target.isFile();
              try {
                File directory = target.getParentFile();
                if (!directory.isDirectory() && !directory.mkdirs())
                  throw new IOException("无法创建更新目录");
                if (target.isFile()) {
                  verify(target, selected);
                } else {
                  URL url = new URL(selected.url);
                  if (!"https".equalsIgnoreCase(url.getProtocol()))
                    throw new IOException("更新地址必须使用 HTTPS");
                  connection = (HttpsURLConnection) url.openConnection();
                  connection.setInstanceFollowRedirects(false);
                  connection.setConnectTimeout(10000);
                  connection.setReadTimeout(15000);
                  int status = connection.getResponseCode();
                  if (status != 200) throw new IOException("更新下载 HTTP " + status);
                  long length = connection.getContentLengthLong();
                  if (length > 0 && length != selected.size)
                    throw new IOException("更新文件长度与发布信息不一致");
                  long received = 0;
                  int previous = -1;
                  try (InputStream input = connection.getInputStream();
                      OutputStream output = new FileOutputStream(partial)) {
                    byte[] buffer = new byte[32768];
                    int count;
                    while ((count = input.read(buffer)) != -1) {
                      if (cancelDownload) break;
                      received += count;
                      if (received > selected.size) throw new IOException("更新文件超出发布大小");
                      output.write(buffer, 0, count);
                      int percent = (int) (received * 100 / selected.size);
                      if (percent != previous) {
                        previous = percent;
                        long done = received;
                        app.runOnUiThread(() -> updateProgress(percent, done, selected.size));
                      }
                    }
                  }
                  if (cancelDownload) return;
                  verify(partial, selected);
                  if (cancelDownload) return;
                  if (!partial.renameTo(target)) throw new IOException("无法保存已验证更新文件");
                }
                if (cancelDownload) return;
                app.runOnUiThread(
                    () -> {
                      if (destroyed || cancelDownload || !sameInfo(selected)) return;
                      ready = true;
                      closeDialog();
                      install(selected);
                    });
              } catch (Exception error) {
                if (cached && target.exists() && !target.delete())
                  Log.e(TAG, "Could not remove invalid cached update");
                String message = error.getMessage();
                if (!cancelDownload) {
                  Log.e(TAG, "Download version=" + selected.version, error);
                  app.runOnUiThread(
                      () -> {
                        if (!destroyed) showError("下载未完成", message, this::download);
                      });
                }
              } finally {
                HttpsURLConnection active = connection;
                if (active != null) active.disconnect();
                connection = null;
                if (partial.exists() && !partial.delete())
                  Log.e(TAG, "Could not remove incomplete update");
                app.runOnUiThread(
                    () -> {
                      downloading = false;
                      if (!destroyed && (cancelDownload || !sameInfo(selected))) {
                        closeDialog();
                        maybePresent();
                      }
                    });
              }
            },
            "app-update-download")
        .start();
  }

  private void presentDownload() {
    if (!app.foreground || destroyed) return;
    LinearLayout card = card("小欢喜正在赶来");
    progressText = app.text("正在准备下载", 13, MainActivity.MUTED, false);
    app.add(card, progressText, -1, -2);
    app.gap(card, 14);
    progress = new ProgressBar(app, null, android.R.attr.progressBarStyleHorizontal);
    progress.setMax(100);
    progress.setProgressTintList(android.content.res.ColorStateList.valueOf(MainActivity.RED));
    progress.setProgressBackgroundTintList(android.content.res.ColorStateList.valueOf(0xffffdcc5));
    app.add(card, progress, -1, 8);
    app.gap(card, 20);
    app.add(
        card,
        app.button(
            "取消下载",
            () -> {
              cancelDownload = true;
              progressText.setText("正在取消…");
              HttpsURLConnection active = connection;
              if (active != null) active.disconnect();
            }),
        -1,
        48);
    show(card);
  }

  private void updateProgress(int percent, long received, long total) {
    if (destroyed || progress == null || !downloading) return;
    progress.setProgress(percent);
    progressText.setText(
        percent + "%  ·  " + megabytes(received) + " / " + megabytes(total) + " MB");
  }

  private String megabytes(long size) {
    return String.format(java.util.Locale.CHINA, "%.1f", size / 1048576.0);
  }

  @SuppressWarnings("deprecation")
  private void verify(File apk, Info expected) throws Exception {
    if (apk.length() != expected.size) throw new IOException("更新文件大小不一致，请重新下载");
    MessageDigest digest = MessageDigest.getInstance("SHA-256");
    try (InputStream input = new FileInputStream(apk)) {
      byte[] buffer = new byte[32768];
      int count;
      while ((count = input.read(buffer)) != -1) digest.update(buffer, 0, count);
    }
    if (!hex(digest.digest()).equalsIgnoreCase(expected.sha))
      throw new IOException("更新文件校验失败，请重新下载");
    PackageManager manager = app.getPackageManager();
    int flags =
        Build.VERSION.SDK_INT >= 28
            ? PackageManager.GET_SIGNING_CERTIFICATES
            : PackageManager.GET_SIGNATURES;
    PackageInfo archive = manager.getPackageArchiveInfo(apk.getPath(), flags);
    PackageInfo installed = manager.getPackageInfo(app.getPackageName(), flags);
    if (archive == null || !app.getPackageName().equals(archive.packageName))
      throw new IOException("更新文件不属于小橙有喜");
    long code = Build.VERSION.SDK_INT >= 28 ? archive.getLongVersionCode() : archive.versionCode;
    if (code != expected.version || code <= BuildConfig.VERSION_CODE)
      throw new IOException("更新文件版本与发布信息不一致");
    if (Build.VERSION.SDK_INT >= 28
        && (archive.signingInfo == null || installed.signingInfo == null))
      throw new IOException("更新文件未包含有效签名");
    Signature[] incoming =
        Build.VERSION.SDK_INT >= 28
            ? archive.signingInfo.getApkContentsSigners()
            : archive.signatures;
    Signature[] current =
        Build.VERSION.SDK_INT >= 28
            ? installed.signingInfo.getApkContentsSigners()
            : installed.signatures;
    if (incoming == null || current == null || !sameSigners(incoming, current))
      throw new IOException("更新文件签名与当前应用不一致");
  }

  static boolean sameSigners(Signature[] left, Signature[] right) {
    if (left.length != right.length || left.length == 0) return false;
    String[] a = new String[left.length], b = new String[right.length];
    for (int i = 0; i < a.length; i++) {
      a[i] = left[i].toCharsString();
      b[i] = right[i].toCharsString();
    }
    Arrays.sort(a);
    Arrays.sort(b);
    return Arrays.equals(a, b);
  }

  static String hex(byte[] bytes) {
    StringBuilder value = new StringBuilder(bytes.length * 2);
    for (byte part : bytes) value.append(String.format(java.util.Locale.ROOT, "%02x", part & 255));
    return value.toString();
  }

  private void verifyReadyAndInstall() {
    if (info == null || downloading) return;
    Info selected = info;
    downloading = true;
    cancelDownload = false;
    presentDownload();
    if (progress != null) {
      progress.setIndeterminate(true);
      progressText.setText("正在校验安装包");
    }
    new Thread(
            () -> {
              try {
                verify(file(selected), selected);
                app.runOnUiThread(
                    () -> {
                      downloading = false;
                      if (destroyed) return;
                      if (cancelDownload || !sameInfo(selected)) {
                        closeDialog();
                        maybePresent();
                      } else {
                        ready = true;
                        closeDialog();
                        install(selected);
                      }
                    });
              } catch (Exception error) {
                Log.e(TAG, "Install archive validation version=" + selected.version, error);
                File invalid = file(selected);
                if (invalid.exists() && !invalid.delete())
                  Log.e(TAG, "Could not remove invalid install archive");
                app.runOnUiThread(
                    () -> {
                      downloading = false;
                      ready = false;
                      if (!destroyed) {
                        if (cancelDownload || !sameInfo(selected)) {
                          closeDialog();
                          maybePresent();
                        } else showError("安装校验未通过", error.getMessage(), this::download);
                      }
                    });
              }
            },
            "app-update-install-check")
        .start();
  }

  private boolean sameInfo(Info selected) {
    return info != null
        && info.version == selected.version
        && info.size == selected.size
        && info.sha.equals(selected.sha);
  }

  private void install(Info selected) {
    if (destroyed || cancelDownload || !sameInfo(selected)) return;
    if (!app.foreground) return;
    closeDialog();
    try {
      if (Build.VERSION.SDK_INT >= 26 && !app.getPackageManager().canRequestPackageInstalls()) {
        awaitingSettings = true;
        app.startActivity(
            new Intent(
                Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                Uri.parse("package:" + app.getPackageName())));
        return;
      }
      Uri uri =
          FileProvider.getUriForFile(app, app.getPackageName() + ".update-files", file(selected));
      Intent request =
          new Intent(Intent.ACTION_INSTALL_PACKAGE)
              .setData(uri)
              .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
              .putExtra(Intent.EXTRA_RETURN_RESULT, true);
      installing = true;
      app.startActivityForResult(request, INSTALL);
    } catch (RuntimeException error) {
      Log.e(TAG, "Launch installer version=" + selected.version, error);
      installing = false;
      awaitingSettings = false;
      showError("安装未打开", error.getMessage(), this::verifyReadyAndInstall);
    }
  }

  void activityResult(int request, int result) {
    if (request != INSTALL) return;
    installing = false;
    if (result == Activity.RESULT_OK) app.finish();
    else maybePresent();
  }

  void save(Bundle state) {
    if (info != null) state.putString("update_info", info.raw.toString());
    state.putBoolean("update_ready", ready);
    state.putBoolean("update_settings", awaitingSettings);
  }

  void pause() {
    if (artMotion != null) artMotion.cancel();
  }

  private void closeDialog() {
    if (artMotion != null) artMotion.cancel();
    artMotion = null;
    if (dialog != null) dialog.dismiss();
    dialog = null;
    progress = null;
    progressText = null;
  }

  void destroy() {
    destroyed = true;
    cancelDownload = true;
    HttpsURLConnection active = connection;
    if (active != null) active.disconnect();
    closeDialog();
  }

  private static final class Info {
    final JSONObject raw;
    final int version;
    final String name, title, notes, url, sha;
    final long size;
    final boolean required;

    Info(JSONObject value) throws JSONException {
      raw = value;
      version = value.getInt("latest_version_code");
      name = value.getString("latest_version_name");
      title = value.getString("title");
      notes = value.getString("release_notes");
      url = value.getString("apk_url");
      sha = value.getString("apk_sha256");
      size = value.getLong("apk_size_bytes");
      required = value.getBoolean("force_update");
      if (!url.startsWith("https://") || !sha.matches("[0-9a-fA-F]{64}") || size <= 0)
        throw new JSONException("更新文件信息无效");
    }
  }
}
