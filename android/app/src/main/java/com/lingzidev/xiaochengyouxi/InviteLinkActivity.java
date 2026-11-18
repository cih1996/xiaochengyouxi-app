package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;

/**
 * Receives invite links (xiaochengyouxi://invite?code=XXXX, opened from the invite landing page),
 * remembers the code and brings the existing app task forward. MainActivity accepts the
 * invitation for the current 新人礼包 round once the user is signed in.
 */
public final class InviteLinkActivity extends Activity {
  @Override
  protected void onCreate(Bundle state) {
    super.onCreate(state);
    Uri data = getIntent() == null ? null : getIntent().getData();
    String code = data == null ? null : data.getQueryParameter("code");
    if (code != null) {
      code = code.trim().toUpperCase(java.util.Locale.ROOT);
      if (code.matches("[A-Z0-9]{4,16}"))
        getSharedPreferences("account", 0).edit().putString(MainActivity.PENDING_INVITE, code).apply();
    }
    startActivity(
        new Intent(this, MainActivity.class)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP | Intent.FLAG_ACTIVITY_SINGLE_TOP));
    finish();
  }
}
