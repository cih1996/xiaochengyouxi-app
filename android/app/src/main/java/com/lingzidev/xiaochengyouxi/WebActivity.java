package com.lingzidev.xiaochengyouxi;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.content.Intent;
import android.graphics.Bitmap;
import android.net.Uri;
import android.os.Bundle;
import android.text.TextUtils;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.webkit.JavascriptInterface;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.TextView;
import android.widget.Toast;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;

/**
 * Hosts H5 activity pages. The OrangeApp bridge only answers while the current page is on one of
 * our own hosts; navigation to any other host is refused. Coins are always settled by the server.
 */
public final class WebActivity extends Activity {
  static final String EXTRA_URL = "url", EXTRA_TITLE = "title", RESULT_OPEN = "open";
  static final int REQUEST = 4107;
  private static final int CREAM = 0xfffff8f2, INK = 0xff392923;

  private WebView web;
  private TextView title;
  private ProgressBar progress;
  private volatile String currentHost = "";
  private Set<String> hosts;

  static Set<String> allowedHosts() {
    Set<String> set = new HashSet<>(Arrays.asList(BuildConfig.ACTIVITY_HOSTS.split(",")));
    set.add(Uri.parse(BuildConfig.API_BASE_URL).getHost());
    set.add(Uri.parse(BuildConfig.WEB_BASE_URL).getHost());
    set.remove("");
    return set;
  }

  /** Relative paths resolve against the API host; absolute URLs must be https on an allowed host. */
  static String resolve(String target) {
    if (target.startsWith("/") && !target.startsWith("//")) return BuildConfig.WEB_BASE_URL + target;
    Uri uri = Uri.parse(target);
    if ("https".equals(uri.getScheme()) && allowedHosts().contains(host(uri))) return target;
    return null;
  }

  private static String host(Uri uri) {
    return uri.getHost() == null ? "" : uri.getHost().toLowerCase(Locale.ROOT);
  }

  private boolean allowed(Uri uri) {
    String scheme = uri.getScheme();
    boolean secure =
        "https".equals(scheme)
            || ("http".equals(scheme) && BuildConfig.API_BASE_URL.startsWith("http://"));
    return secure && hosts.contains(host(uri)) && uri.getUserInfo() == null;
  }

  private int dp(float value) {
    return Math.round(
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value, getResources().getDisplayMetrics()));
  }

  @SuppressLint({"SetJavaScriptEnabled", "AddJavascriptInterface"})
  @Override
  protected void onCreate(Bundle state) {
    super.onCreate(state);
    hosts = allowedHosts();
    String url = getIntent().getStringExtra(EXTRA_URL);
    if (url == null || !allowed(Uri.parse(url))) {
      Toast.makeText(this, "活动地址无效", Toast.LENGTH_SHORT).show();
      finish();
      return;
    }
    getWindow().setStatusBarColor(CREAM);
    getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR);

    LinearLayout root = new LinearLayout(this);
    root.setOrientation(LinearLayout.VERTICAL);
    root.setBackgroundColor(CREAM);
    root.setOnApplyWindowInsetsListener(
        (v, insets) -> {
          v.setPadding(0, insets.getSystemWindowInsetTop(), 0, insets.getSystemWindowInsetBottom());
          return insets.consumeSystemWindowInsets();
        });

    LinearLayout bar = new LinearLayout(this);
    bar.setGravity(Gravity.CENTER_VERTICAL);
    bar.setPadding(dp(6), 0, dp(6), 0);
    LineIcon back = new LineIcon(this, "back", INK);
    back.setPadding(dp(12), dp(12), dp(12), dp(12));
    back.setContentDescription("返回");
    back.setOnClickListener(v -> onBackPressed());
    bar.addView(back, new LinearLayout.LayoutParams(dp(46), dp(46)));
    title = new TextView(this);
    title.setTextColor(INK);
    title.setTextSize(17);
    title.getPaint().setFakeBoldText(true);
    title.setSingleLine(true);
    title.setEllipsize(TextUtils.TruncateAt.END);
    title.setGravity(Gravity.CENTER);
    title.setText(getIntent().getStringExtra(EXTRA_TITLE));
    bar.addView(title, new LinearLayout.LayoutParams(0, -1, 1));
    LineIcon close = new LineIcon(this, "close", INK);
    close.setPadding(dp(13), dp(13), dp(13), dp(13));
    close.setContentDescription("关闭");
    close.setOnClickListener(v -> finish());
    bar.addView(close, new LinearLayout.LayoutParams(dp(46), dp(46)));
    root.addView(bar, new LinearLayout.LayoutParams(-1, dp(50)));

    FrameLayout body = new FrameLayout(this);
    web = new WebView(this);
    progress = new ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal);
    progress.setMax(100);
    body.addView(web, new FrameLayout.LayoutParams(-1, -1));
    body.addView(progress, new FrameLayout.LayoutParams(-1, dp(3), Gravity.TOP));
    root.addView(body, new LinearLayout.LayoutParams(-1, 0, 1));
    setContentView(root);

    WebSettings s = web.getSettings();
    s.setJavaScriptEnabled(true);
    s.setDomStorageEnabled(true);
    s.setAllowFileAccess(false);
    s.setAllowContentAccess(false);
    s.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
    s.setUserAgentString(s.getUserAgentString() + " OrangeApp/" + BuildConfig.VERSION_NAME);
    web.addJavascriptInterface(new Bridge(), "OrangeApp");
    web.setWebViewClient(
        new WebViewClient() {
          @Override
          public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
            Uri uri = request.getUrl();
            if (allowed(uri)) return false;
            Toast.makeText(WebActivity.this, "该链接不能在活动页打开", Toast.LENGTH_SHORT).show();
            return true;
          }

          // Nothing from other hosts may load (scripts, iframes, images), so only our own code
          // can ever run next to the bridge.
          @Override
          public android.webkit.WebResourceResponse shouldInterceptRequest(
              WebView view, WebResourceRequest request) {
            Uri uri = request.getUrl();
            String scheme = uri.getScheme();
            if ("data".equals(scheme) || "blob".equals(scheme) || allowed(uri)) return null;
            return new android.webkit.WebResourceResponse(
                "text/plain", "utf-8", 403, "Blocked", null, new java.io.ByteArrayInputStream(new byte[0]));
          }

          @Override
          public void onPageStarted(WebView view, String url, Bitmap favicon) {
            currentHost = host(Uri.parse(url));
            progress.setVisibility(View.VISIBLE);
          }

          @Override
          public void doUpdateVisitedHistory(WebView view, String url, boolean reload) {
            currentHost = host(Uri.parse(url));
          }

          @Override
          public void onPageFinished(WebView view, String url) {
            progress.setVisibility(View.GONE);
          }
        });
    web.setWebChromeClient(
        new WebChromeClient() {
          @Override
          public void onProgressChanged(WebView view, int value) {
            progress.setProgress(value);
          }

          @Override
          public void onReceivedTitle(WebView view, String value) {
            if (value != null && !value.startsWith("http")) title.setText(value);
          }
        });
    currentHost = host(Uri.parse(url));
    web.loadUrl(url);
  }

  @Override
  public void onBackPressed() {
    if (web != null && web.canGoBack()) web.goBack();
    else super.onBackPressed();
  }

  @Override
  protected void onDestroy() {
    if (web != null) {
      web.removeJavascriptInterface("OrangeApp");
      web.destroy();
    }
    super.onDestroy();
  }

  /** JavaScript API exposed as window.OrangeApp. Every call re-checks the current page host. */
  private final class Bridge {
    private boolean trusted() {
      return hosts.contains(currentHost);
    }

    @JavascriptInterface
    public String version() {
      return BuildConfig.VERSION_NAME;
    }

    @JavascriptInterface
    public String token() {
      if (!trusted()) return "";
      return getSharedPreferences("account", 0).getString("token", "");
    }

    @JavascriptInterface
    public void toast(String text) {
      if (!trusted() || text == null) return;
      String shown = text.length() > 60 ? text.substring(0, 60) : text;
      runOnUiThread(() -> Toast.makeText(WebActivity.this, shown, Toast.LENGTH_SHORT).show());
    }

    /** Opens a built-in page (checkin, invite, activity, withdraw, shop, ledger, me) and closes the H5 page. */
    @JavascriptInterface
    public void open(String target) {
      if (!trusted() || target == null) return;
      runOnUiThread(
          () -> {
            setResult(RESULT_OK, new Intent().putExtra(RESULT_OPEN, target));
            finish();
          });
    }

    @JavascriptInterface
    public void share(String text) {
      if (!trusted() || text == null) return;
      String shown = text.length() > 300 ? text.substring(0, 300) : text;
      runOnUiThread(
          () ->
              startActivity(
                  Intent.createChooser(
                      new Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, shown),
                      "分享给好友")));
    }

    @JavascriptInterface
    public void close() {
      if (!trusted()) return;
      runOnUiThread(WebActivity.this::finish);
    }
  }
}
