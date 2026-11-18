package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import android.content.Context;
import android.view.View;
import org.json.JSONObject;

/**
 * One ad network SDK behind a common shape. The server decides which networks to try and in
 * what order; an adapter only loads/shows what it is told and reports what happened. Rewards are
 * granted by the server after the network's own server callback, never by these callbacks.
 */
interface AdNetwork {
  String id();

  /** Starts the SDK once with the app id from the server; later calls just report readiness. */
  void init(Context app, String appId, Runnable ready, Failed failed);

  boolean ready();

  /**
   * Loads one rewarded video. {@code session} carries user_id and media_extra, which the network
   * passes through to its server callback so the server can match the reward session.
   */
  void loadRewarded(Activity activity, String codeId, JSONObject session, Rewarded callbacks);

  /** Loads and renders one template feed card no wider than {@code widthPx}. */
  void loadFeed(Activity activity, String codeId, int widthPx, Feed callbacks);

  interface Failed {
    void failed(String code, String message);
  }

  /** Callbacks may arrive on any thread; the mediator moves them to the UI thread. */
  interface Rewarded extends Failed {
    /** Ready to play: call {@code show.run()} on the UI thread. */
    void loaded(Runnable show);

    void shown();

    void clicked();

    /** The SDK's own reward signal (diagnostic only). */
    void verified(boolean valid, Integer errorCode, String message);

    void completed();

    void skipped();

    void playError(String message);

    void closed();
  }

  interface Feed extends Failed {
    void rendered(View view, Runnable destroy);

    void shown();

    void clicked();

    /** The user chose "not interested" / closed the card. */
    void dismissed();
  }
}
