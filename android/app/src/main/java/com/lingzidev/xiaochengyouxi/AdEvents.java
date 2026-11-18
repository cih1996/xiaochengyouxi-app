package com.lingzidev.xiaochengyouxi;

import android.os.Handler;
import android.os.Looper;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** Batches ad lifecycle events for the admin statistics. Failures are silent: stats only. */
final class AdEvents {
  private static final int BATCH = 20, MAX_PENDING = 200;
  private final Api api;
  private final Handler main = new Handler(Looper.getMainLooper());
  private JSONArray pending = new JSONArray();
  private boolean scheduled;

  AdEvents(Api api) {
    this.api = api;
  }

  void log(
      String network,
      String format,
      String codeId,
      String event,
      String session,
      Object errorCode,
      String message,
      long startedAt) {
    android.util.Log.d("AdEvents", network + " " + format + " " + event + (errorCode == null ? "" : " " + errorCode));
    main.post(
        () -> {
          try {
            JSONObject e = new JSONObject();
            e.put("network", network);
            e.put("format", format);
            e.put("event", event);
            if (codeId != null) e.put("code_id", codeId);
            if (session != null && !session.isEmpty()) e.put("session_id", session);
            if (errorCode != null) e.put("error_code", String.valueOf(errorCode));
            if (message != null && !message.isEmpty())
              e.put("message", message.substring(0, Math.min(200, message.length())));
            if (startedAt > 0) e.put("elapsed_ms", System.currentTimeMillis() - startedAt);
            pending.put(e);
          } catch (JSONException ignored) {
            return;
          }
          if (pending.length() >= BATCH) flush();
          else if (!scheduled) {
            scheduled = true;
            main.postDelayed(this::flush, 8000);
          }
        });
  }

  void flush() {
    scheduled = false;
    if (pending.length() == 0 || api.token == null || api.token.isEmpty()) return;
    // The server takes at most 30 events per request; send the rest in a following flush.
    JSONArray batch = new JSONArray(), rest = new JSONArray();
    for (int i = 0; i < pending.length(); i++) (i < 30 ? batch : rest).put(pending.opt(i));
    pending = rest;
    if (rest.length() > 0) {
      scheduled = true;
      main.postDelayed(this::flush, 1000);
    }
    api.post(
        "/ads/events",
        Api.object("events", batch),
        ignored -> {},
        ignored ->
            main.post(
                () -> {
                  // Put the batch back once (bounded) so a network blip doesn't lose stats.
                  JSONArray merged = new JSONArray();
                  for (int i = 0; i < batch.length() && merged.length() < MAX_PENDING; i++)
                    merged.put(batch.opt(i));
                  for (int i = 0; i < pending.length() && merged.length() < MAX_PENDING; i++)
                    merged.put(pending.opt(i));
                  pending = merged;
                  if (!scheduled) {
                    scheduled = true;
                    main.postDelayed(this::flush, 30000);
                  }
                }));
  }
}
