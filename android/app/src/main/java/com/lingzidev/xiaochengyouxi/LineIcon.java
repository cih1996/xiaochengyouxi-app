package com.lingzidev.xiaochengyouxi;

import android.content.Context;
import android.graphics.*;
import android.view.View;

final class LineIcon extends View {
  String kind;
  int color;
  final Paint p = new Paint(3);

  LineIcon(Context c, String kind, int color) {
    super(c);
    this.kind = kind;
    this.color = color;
  }

  protected void onDraw(Canvas original) {
    super.onDraw(original);
    Canvas c = original;
    c.save();
    c.scale(getWidth() / 24f, getHeight() / 24f);
    p.setColor(color);
    p.setStrokeWidth(1.7f);
    p.setStyle(Paint.Style.STROKE);
    p.setStrokeCap(Paint.Cap.ROUND);
    p.setStrokeJoin(Paint.Join.ROUND);
    Path path = new Path();
    switch (kind) {
      case "eye":
      case "eye_off":
        path.moveTo(2, 12);
        path.quadTo(12, 1, 22, 12);
        path.quadTo(12, 23, 2, 12);
        c.drawPath(path, p);
        c.drawCircle(12, 12, 3, p);
        if (kind.equals("eye_off")) c.drawLine(4, 4, 20, 20, p);
        break;
      case "hall":
        c.drawRoundRect(3, 4, 21, 18, 5, 5, p);
        path.moveTo(8, 18);
        path.lineTo(5, 21);
        path.lineTo(5, 16);
        c.drawPath(path, p);
        c.drawLine(8, 10, 16, 10, p);
        c.drawLine(8, 14, 13, 14, p);
        break;
      case "shop":
        c.drawRoundRect(4, 8, 20, 21, 3, 3, p);
        c.drawArc(8, 2, 16, 14, 180, 180, false, p);
        break;
      case "me":
        c.drawCircle(12, 7, 4, p);
        c.drawArc(4, 13, 20, 27, 180, 180, false, p);
        break;
      case "coin":
        c.drawCircle(12, 12, 9, p);
        c.drawLine(12, 7, 12, 17, p);
        c.drawLine(9, 10, 15, 10, p);
        c.drawLine(9, 14, 15, 14, p);
        break;
      case "gift":
        c.drawRoundRect(3, 9, 21, 14, 1, 1, p);
        c.drawRect(5, 14, 19, 21, p);
        c.drawLine(12, 9, 12, 21, p);
        c.drawOval(6, 3, 12, 9, p);
        c.drawOval(12, 3, 18, 9, p);
        break;
      case "clock":
        c.drawCircle(12, 12, 9, p);
        c.drawLine(12, 7, 12, 12, p);
        c.drawLine(12, 12, 16, 14, p);
        break;
      case "arrow":
        path.moveTo(9, 5);
        path.lineTo(16, 12);
        path.lineTo(9, 19);
        c.drawPath(path, p);
        break;
      case "back":
        path.moveTo(15, 5);
        path.lineTo(8, 12);
        path.lineTo(15, 19);
        c.drawPath(path, p);
        break;
      case "close":
        c.drawLine(6, 6, 18, 18, p);
        c.drawLine(18, 6, 6, 18, p);
        break;
      case "check":
        path.moveTo(5, 12);
        path.lineTo(10, 17);
        path.lineTo(20, 6);
        c.drawPath(path, p);
        break;
      case "notice":
        c.drawArc(6, 4, 18, 20, 180, 180, false, p);
        path.moveTo(6, 12);
        path.lineTo(5, 18);
        path.lineTo(19, 18);
        path.lineTo(18, 12);
        c.drawPath(path, p);
        c.drawArc(10, 18, 14, 23, 0, 180, false, p);
        break;
      case "shield":
        path.moveTo(12, 3);
        path.lineTo(20, 6);
        path.lineTo(19, 14);
        path.quadTo(17, 19, 12, 22);
        path.quadTo(7, 19, 5, 14);
        path.lineTo(4, 6);
        path.close();
        c.drawPath(path, p);
        break;
    }
    c.restore();
  }
}
