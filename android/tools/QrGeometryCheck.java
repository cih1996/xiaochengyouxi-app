package com.lingzidev.xiaochengyouxi;

public final class QrGeometryCheck {
  public static void main(String[] args) {
    int checks = 0;
    for (int size=21;size<=177;size+=4) {
      QrLayout grid=new QrLayout(size,392);
      if(grid.pixels>392)throw new AssertionError("QR exceeds poster area");
      if(grid.position(0)!=4*grid.cell)throw new AssertionError("Left quiet zone must be four modules");
      int contentEnd=grid.position(size-1)+grid.cell;
      if(grid.pixels-contentEnd!=4*grid.cell)throw new AssertionError("Right quiet zone must be four modules");
      for(int module=0;module<size-1;module++) {
        if(grid.position(module+1)-grid.position(module)!=grid.cell)throw new AssertionError("Modules must align to integer pixels");
      }
      checks++;
    }
    System.out.println("QR geometry passed across "+checks+" standard dimensions; integer pixels and four-module quiet zones.");
  }
}
