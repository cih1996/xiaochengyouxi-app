package com.lingzidev.xiaochengyouxi;

public final class RainTimingCheck {
  public static void main(String[] args) {
    long starts=RainTiming.parse("2026-11-11T04:00:00.000Z");
    long expires=RainTiming.parse("2026-11-11T04:00:08.000Z");
    if(!RainTiming.remaining(3600001).equals("01:00:01"))throw new AssertionError("Countdown must ceil remaining seconds");
    if(!RainTiming.remaining(-1).equals("00:00:00"))throw new AssertionError("Countdown cannot become negative");
    if(RainTiming.progress(starts,expires,starts-1000)!=0)throw new AssertionError("Unrevealed packet must not advance");
    if(RainTiming.progress(starts,expires,starts+4000)!=.5f)throw new AssertionError("Resume must use server window progress");
    if(RainTiming.progress(starts,expires,expires+1)!=1)throw new AssertionError("Expired packet window cannot extend");
    if(!RainTiming.remaining(999).equals("00:00:01"))throw new AssertionError("Last second must remain visible");
    try {RainTiming.parse("2026-11-32T04:00:00.000Z");throw new AssertionError("Invalid upstream date must surface");}
    catch(IllegalArgumentException expected) { }
    try {RainTiming.progress(starts,starts,starts);throw new AssertionError("Invalid upstream window must surface");}
    catch(IllegalArgumentException expected) { }
    System.out.println("Rain timing checks passed: ISO UTC, countdown ceiling, resume progress and expiry boundaries.");
  }
}
