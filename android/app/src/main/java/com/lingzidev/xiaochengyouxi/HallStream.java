package com.lingzidev.xiaochengyouxi;

import android.app.Activity;
import java.io.BufferedReader;
import java.io.Closeable;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

final class HallStream implements Closeable {
  interface Message {
    void received(JSONObject message);
  }

  private final Activity activity;
  private final Message onMessage;
  private final Api.Failure onFailure;
  private volatile HttpURLConnection connection;
  private volatile boolean closed;

  HallStream(Activity activity, String token, Message onMessage, Api.Failure onFailure) {
    this.activity = activity;
    this.onMessage = onMessage;
    this.onFailure = onFailure;
    new Thread(() -> read(token), "hall-stream").start();
  }

  private void read(String token) {
    try {
      HttpURLConnection request =
          (HttpURLConnection)
              new URL(
                      BuildConfig.API_BASE_URL
                          + "/api/hall/stream"
                          + (BuildConfig.ADS_ENABLED ? "?native_ads=1" : ""))
                  .openConnection();
      connection = request;
      if (closed) return;
      request.setConnectTimeout(7000);
      // Server sends a keep-alive every 15s; a silent socket (doze, network switch) is treated as dead.
      request.setReadTimeout(45000);
      request.setRequestProperty("Accept", "text/event-stream");
      request.setRequestProperty("Authorization", "Bearer " + token);
      int status = request.getResponseCode();
      if (status >= 400) {
        StringBuilder body = new StringBuilder();
        try (BufferedReader input =
            new BufferedReader(
                new InputStreamReader(request.getErrorStream(), StandardCharsets.UTF_8))) {
          String line;
          while ((line = input.readLine()) != null) body.append(line);
        }
        throw new IOException(new JSONObject(body.toString()).getString("error"));
      }
      try (BufferedReader input =
          new BufferedReader(
              new InputStreamReader(request.getInputStream(), StandardCharsets.UTF_8))) {
        String event = "message";
        StringBuilder data = new StringBuilder();
        String line;
        while (!closed && (line = input.readLine()) != null) {
          if (line.isEmpty()) {
            if (data.length() > 0) receive(event, data.toString());
            event = "message";
            data.setLength(0);
          } else if (line.startsWith("event:")) {
            event = line.substring(6).trim();
          } else if (line.startsWith("data:")) {
            if (data.length() > 0) data.append('\n');
            data.append(line.substring(5).trim());
          }
        }
      }
      if (!closed) throw new IOException("大厅连接已结束");
    } catch (Exception failure) {
      if (!closed) {
        activity.runOnUiThread(
            () -> {
              if (!closed && !activity.isDestroyed()) {
                closed = true;
                onFailure.failed(failure.getMessage());
              }
            });
      }
    } finally {
      if (connection != null) connection.disconnect();
    }
  }

  private void receive(String event, String data) throws Exception {
    JSONObject message = new JSONObject(data);
    if (event.equals("error")) throw new IOException(message.getString("error"));
    String kind = message.getString("kind");
    message.getString("id");
    message.getString("text");
    message.getString("created_at");
    switch (kind) {
      case "text":
        break;
      case "red_packet":
        message.getJSONObject("task");
        break;
      case "product":
        message.getJSONObject("product");
        break;
      case "native_ad":
        message.getString("ad_code_id");
        break;
      default:
        throw new IOException("未知大厅消息类型：" + kind);
    }
    activity.runOnUiThread(
        () -> {
          if (!closed && !activity.isDestroyed()) onMessage.received(message);
        });
  }

  boolean closed() {
    return closed;
  }

  @Override
  public void close() {
    closed = true;
    if (connection != null) connection.disconnect();
  }
}
