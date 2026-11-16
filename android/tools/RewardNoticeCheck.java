package com.lingzidev.xiaochengyouxi;

public final class RewardNoticeCheck {
  public static void main(String[] args) {
    RewardedAdProvider.Notice missing = new RewardedAdProvider.Notice();
    missing.arrived(false, null, null);
    if (missing.valid || missing.hasError() || missing.errorCode != null)
      throw new AssertionError("An unconfirmed callback must preserve missing diagnostics");
    missing.complete = true;
    if (missing.valid) throw new AssertionError("Video completion is not reward approval");

    RewardedAdProvider.Notice valid = new RewardedAdProvider.Notice();
    valid.arrived(true, null, null);
    valid.arrived(false, 500, "optional interaction failed");
    if (!valid.valid || valid.hasError())
      throw new AssertionError("A later optional result must not erase a positive notification");

    RewardedAdProvider.Notice failed = new RewardedAdProvider.Notice();
    failed.arrived(false, 402, "upstream rejected");
    if (failed.valid || !failed.hasError())
      throw new AssertionError("An explicit failure must remain distinguishable from pending");
    failed.arrived(true, null, null);
    if (!failed.valid || failed.hasError())
      throw new AssertionError("A later positive notification should clear the display failure");
    System.out.println("Reward notices passed: missing fields, completion, mixed notifications and explicit errors.");
  }
}
