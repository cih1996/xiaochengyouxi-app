package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.app.Dialog;
import android.graphics.Color;
import android.os.CountDownTimer;
import android.view.Gravity;
import android.widget.*;
import org.json.JSONObject;

interface RewardedAdProvider {
  interface Closed {
    void done(Notice notice);
  }

  final class Notice {
    boolean notified, valid, complete, skipped, playbackError;
    Integer errorCode;
    String errorMessage;

    void arrived(boolean accepted, Integer code, String message) {
      notified = true;
      valid |= accepted;
      if (!accepted && !valid) {
        errorCode = code;
        errorMessage = message;
      }
    }

    boolean hasError() {
      return playbackError
          || (!valid
              && ((errorCode != null && errorCode != 0)
                  || (errorMessage != null && !errorMessage.trim().isEmpty())));
    }
  }

  void show(
      Activity activity,
      JSONObject session,
      Closed closed,
      Api.Result clientEvent,
      Api.Failure failure);

  final class Mock implements RewardedAdProvider {
    public void show(
        Activity activity,
        JSONObject session,
        Closed closed,
        Api.Result clientEvent,
        Api.Failure failure) {
      int seconds = session.optInt("duration_seconds", 5);
      Dialog dialog =
          new Dialog(activity, android.R.style.Theme_Material_Light_NoActionBar_Fullscreen);
      LinearLayout box = new LinearLayout(activity);
      box.setOrientation(LinearLayout.VERTICAL);
      box.setGravity(Gravity.CENTER);
      box.setPadding(36, 50, 36, 50);
      box.setBackgroundColor(Color.rgb(255, 248, 242));
      TextView tag = new TextView(activity);
      tag.setText("广告体验");
      tag.setTextColor(0xffb99b8d);
      tag.setTextSize(14);
      box.addView(tag);
      TextView symbol = new TextView(activity);
      symbol.setText("小橙有喜");
      symbol.setTextColor(0xfff65331);
      symbol.setTextSize(48);
      symbol.setTypeface(null, 1);
      symbol.setPadding(0, 80, 0, 20);
      box.addView(symbol);
      TextView title = new TextView(activity);
      title.setText("把小确幸，装进口袋");
      title.setTextColor(0xff392923);
      title.setTextSize(23);
      box.addView(title);
      TextView subtitle = new TextView(activity);
      subtitle.setText("观看完成后领取金币");
      subtitle.setTextColor(0xffa08a7e);
      subtitle.setTextSize(14);
      subtitle.setPadding(0, 24, 0, 50);
      box.addView(subtitle);
      TextView timer = new TextView(activity);
      timer.setTextSize(18);
      timer.setTextColor(0xfff65331);
      box.addView(timer);
      TextView close = new TextView(activity);
      close.setText("提前离开");
      close.setTextColor(0xffa08a7e);
      close.setPadding(30, 60, 30, 30);
      box.addView(close);
      dialog.setContentView(box);
      final boolean[] done = {false};
      CountDownTimer counter =
          new CountDownTimer(seconds * 1000L + 250, 200) {
            public void onTick(long remaining) {
              timer.setText("还剩 " + Math.max(1, (remaining + 999) / 1000) + " 秒");
            }

            public void onFinish() {
              done[0] = true;
              dialog.dismiss();
              Notice notice = new Notice();
              notice.arrived(true, null, null);
              notice.complete = true;
              closed.done(notice);
            }
          };
      dialog.setOnDismissListener(
          d -> {
            counter.cancel();
            if (!done[0]) {
              Notice notice = new Notice();
              notice.skipped = true;
              closed.done(notice);
            }
          });
      close.setOnClickListener(v -> dialog.dismiss());
      dialog.show();
      counter.start();
    }
  }
}
