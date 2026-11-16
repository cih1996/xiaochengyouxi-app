package com.lingzidev.xiaochengyouxi;

import java.io.IOException;
import java.lang.reflect.Method;

public final class RiskPrivacyCheck {
  public static void main(String[] args) throws Exception {
    Method code=DeviceTrustClient.class.getDeclaredMethod("code",Throwable.class);
    code.setAccessible(true);
    Throwable[] errors={
      new IOException("HTTP 403: {score:99,reason:private_rule,appKey:private_value}"),
      new IOException("HTTP 503: internal upstream diagnostic"),
      new IOException("raw device evidence must not be returned"),
      new IllegalArgumentException("private configuration detail"),
      new UnsatisfiedLinkError("/private/native/library/path")
    };
    String[] expected={"HTTP_403","HTTP_503","IOException","IllegalArgumentException","UnsatisfiedLinkError"};
    for(int i=0;i<errors.length;i++) {
      String actual=(String)code.invoke(null,errors[i]);
      if(!actual.equals(expected[i]) || !actual.matches("[A-Za-z0-9_.]+"))
        throw new AssertionError("Failure audit exposed an unexpected value");
    }
    System.out.println("Risk error redaction passed: HTTP status/exception class only across five sensitive-message cases.");
  }
}
