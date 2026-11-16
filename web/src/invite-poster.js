function roundedRect(ctx, x, y, width, height, radius, color) {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.roundRect(x, y, width, height, radius);
  ctx.fill();
}
function image(source) {
  return new Promise((resolve, reject) => {
    const asset = new Image();
    asset.onload = () => resolve(asset);
    asset.onerror = () => reject(new Error("海报图片加载失败：" + source));
    asset.src = source;
  });
}
export function drawQr(ctx, qr, x, y, maxSize) {
  const scale = Math.floor(maxSize / (qr.size + 8));
  const size = scale * (qr.size + 8);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(x, y, size, size);
  ctx.fillStyle = "#30251d";
  qr.modules.forEach((row, yy) => {
    for (let xx = 0; xx < row.length; xx++) {
      if (row[xx] === "1")
        ctx.fillRect(x + (xx + 4) * scale, y + (yy + 4) * scale, scale, scale);
    }
  });
}
export async function createInvitePoster(summary, qr) {
  const [mascot, packet, shopping] = await Promise.all([
    image("/assets/mascot.png"),
    image("/assets/packet.png"),
    image("/assets/shopping.png"),
  ]);
  const canvas = document.createElement("canvas");
  canvas.width = 1080;
  canvas.height = 1600;
  const ctx = canvas.getContext("2d");
  const bg = ctx.createLinearGradient(0, 0, 1080, 1600);
  bg.addColorStop(0, "#fff7ec");
  bg.addColorStop(0.6, "#ffead5");
  bg.addColorStop(1, "#ffdbbb");
  ctx.fillStyle = bg;
  ctx.fillRect(0, 0, 1080, 1600);
  ctx.strokeStyle = "#efcdaa";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.ellipse(980, 650, 430, 570, 0.55, 0, Math.PI * 2);
  ctx.stroke();
  ctx.strokeStyle = "#ffffffbb";
  ctx.beginPath();
  ctx.ellipse(985, 650, 380, 510, 0.55, 0, Math.PI * 2);
  ctx.stroke();
  roundedRect(ctx, 82, 65, 75, 75, 24, "#f47540");
  ctx.fillStyle = "#fff8ee";
  ctx.font = 'bold 42px "Microsoft YaHei", sans-serif';
  ctx.fillText("橙", 99, 118);
  ctx.fillStyle = "#63432d";
  ctx.font = 'bold 44px "Microsoft YaHei", sans-serif';
  ctx.fillText("小橙有喜", 178, 118);
  roundedRect(ctx, 816, 80, 174, 47, 24, "#fffaf3");
  ctx.fillStyle = "#bf9366";
  ctx.font = '25px "Microsoft YaHei", sans-serif';
  ctx.fillText("好友邀请", 853, 113);
  ctx.fillStyle = "#66412d";
  ctx.font = 'bold 76px "Microsoft YaHei", sans-serif';
  ctx.fillText("把小欢喜，", 80, 272);
  ctx.fillStyle = "#ed6a3f";
  ctx.font = 'bold 91px "Microsoft YaHei", sans-serif';
  ctx.fillText("分享给朋友", 80, 381);
  ctx.fillStyle = "#b38b68";
  ctx.font = '30px "Microsoft YaHei", sans-serif';
  ctx.fillText("来大厅领福利，逛逛生活好物", 86, 445);
  ctx.drawImage(shopping, 40, 590, 340, 340);
  ctx.drawImage(mascot, 286, 440, 530, 530);
  ctx.drawImage(packet, 763, 620, 250, 250);
  ctx.fillStyle = "#edaa59";
  ctx.font = "52px sans-serif";
  ctx.fillText("✦", 855, 512);
  ctx.font = "28px sans-serif";
  ctx.fillText("✦", 204, 541);
  roundedRect(ctx, 80, 945, 920, 154, 29, "#fffaf2");
  ctx.fillStyle = "#b08a66";
  ctx.font = '27px "Microsoft YaHei", sans-serif';
  ctx.fillText("你的专属邀请码", 116, 1000);
  ctx.fillStyle = "#d66f3e";
  ctx.font = "bold 76px sans-serif";
  ctx.fillText(summary.invite_code, 502, 1043);
  ctx.fillStyle = "#c8a583";
  ctx.font = '23px "Microsoft YaHei", sans-serif';
  ctx.fillText("好事，和朋友一起分享", 116, 1045);
  roundedRect(ctx, 80, 1130, 920, 385, 30, "#ffffff");
  drawQr(ctx, qr, 108, 1158, 324);
  ctx.fillStyle = "#8b5a38";
  ctx.font = 'bold 39px "Microsoft YaHei", sans-serif';
  ctx.fillText("扫码输入手机号", 480, 1250);
  ctx.fillText("加入福利大厅", 480, 1309);
  ctx.fillStyle = "#b79978";
  ctx.font = '25px "Microsoft YaHei", sans-serif';
  ctx.fillText("和朋友一起，收获小欢喜", 480, 1371);
  ctx.fillStyle = "#c8ae91";
  ctx.font = "22px sans-serif";
  ctx.fillText(new URL(qr.url).host, 480, 1443);
  ctx.fillStyle = "#ba9470";
  ctx.font = '24px "Microsoft YaHei", sans-serif';
  ctx.textAlign = "center";
  ctx.fillText("小橙有喜 · 小小福利，暖暖日常", 540, 1563);
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error("海报生成失败"))),
      "image/png",
    ),
  );
}
