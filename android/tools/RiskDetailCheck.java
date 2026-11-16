package com.lingzidev.xiaochengyouxi;

import java.io.IOException;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.security.KeyStoreException;
import java.security.ProviderException;

public final class RiskDetailCheck {
  public static void main(String[] args) throws Exception {
    Method detail = DeviceTrustClient.class.getDeclaredMethod("detail", Throwable.class, String.class);
    detail.setAccessible(true);
    String token = "private-current-token";
    Throwable error = new ProviderException("Failed to generate " + token,
        new KeyStoreException("Keystore error -10003 " + token));
    String text = (String) detail.invoke(null, error, token);
    if (!text.contains("ProviderException") || !text.contains("KeyStoreException")
        || !text.contains("-10003") || text.contains(token))
      throw new AssertionError("Private audit must retain the cause without the auth token");
    String http = (String) detail.invoke(null, new IOException("HTTP 403: {report:private-report,token:" + token + "}"), token);
    if (http.contains("private-report") || http.contains(token) || !http.contains("HTTP 403"))
      throw new AssertionError("HTTP bodies must not be copied to the local diagnostic detail");
    Throwable longError = new ProviderException("长".repeat(10000),
        new IllegalStateException("长".repeat(10000), new RuntimeException("长".repeat(10000),
            new IOException("长".repeat(10000), new RuntimeException("fifth cause")))));
    String bounded = (String) detail.invoke(null, longError, token);
    if (bounded.getBytes(StandardCharsets.UTF_8).length > 2048 || bounded.contains("fifth cause"))
      throw new AssertionError("Diagnostic cause chain exceeded its private audit boundary");
    System.out.println("Private risk diagnostics passed: Keystore cause, token removal, no HTTP body, bounded chain.");
  }
}
