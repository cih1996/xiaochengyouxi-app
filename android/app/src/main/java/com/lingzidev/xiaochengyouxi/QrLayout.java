package com.lingzidev.xiaochengyouxi;

final class QrLayout {
  static final int QUIET_MODULES = 4;
  final int modules, cell, pixels;

  QrLayout(int modules, int maximumPixels) {
    if (modules <= 0) throw new IllegalArgumentException("二维码尺寸错误");
    this.modules = modules;
    cell = maximumPixels / (modules + QUIET_MODULES * 2);
    if (cell < 1) throw new IllegalArgumentException("二维码显示区域太小");
    pixels = (modules + QUIET_MODULES * 2) * cell;
  }

  int position(int module) {
    return (module + QUIET_MODULES) * cell;
  }
}
