const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const base = process.env.ORANGE_WEB_URL || "http://127.0.0.1:38180";
async function api(route, body, token) {
  const response = await fetch(base + "/api" + route, {
    method: body ? "POST" : "GET",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: "Bearer " + token } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await response.json();
  return { response, data };
}
async function uiRegister(page, phone, nickname) {
  await page.addInitScript(() => localStorage.setItem("welcome_seen", "1"));
  await page.goto(base);
  await page.getByRole("button", { name: "登录领福利", exact: true }).click();
  await page.getByRole("button", { name: "注册账号", exact: true }).click();
  await page.getByLabel("手机号", { exact: true }).fill(phone);
  await page.getByLabel("昵称", { exact: true }).fill(nickname);
  await page.getByLabel("密码", { exact: true }).fill("InviteUi123!");
  await page.getByRole("button", { name: "获取验证码", exact: true }).click();
  await page
    .locator(".auth-sheet small")
    .getByText("测试验证码：123456", { exact: true })
    .waitFor();
  await page.getByRole("button", { name: "注册并登录", exact: true }).click();
  await page.locator(".auth-sheet").waitFor({ state: "hidden" });
}
(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  try {
    const output = path.join(__dirname, "output");
    fs.mkdirSync(output, { recursive: true });
    const page = await browser.newPage({
      viewport: { width: 390, height: 844 },
      acceptDownloads: true,
    });
    const errors = [];
    let streams = 0;
    page.on("pageerror", (e) => errors.push(e.message));
    page.on("request", (r) => {
      if (r.url().endsWith("/api/hall/stream")) streams++;
    });
    const platform = await api("/admin/invitations");
    assert.ok(platform.response.ok, platform.data.error);
    const platformCode = platform.data.platform_code;
    const suffix = Number(String(Date.now()).slice(-8));
    const phone = "137" + String(suffix).padStart(8, "0"),
      directPhone = "137" + String((suffix + 1) % 1e8).padStart(8, "0"),
      stalePhone = "137" + String((suffix + 2) % 1e8).padStart(8, "0");
    await page.goto(base + "/i/" + platformCode);
    await page
      .getByRole("heading", { name: "一起收获小欢喜", exact: true })
      .waitFor();
    await page.getByLabel("手机号", { exact: true }).fill(phone);
    await page.locator(".invite-agreement input").check();
    await page.screenshot({
      path: path.join(output, "invite-landing.png"),
      fullPage: true,
    });
    await page.getByRole("button", { name: "收下邀请", exact: true }).click();
    await page
      .getByRole("heading", { name: "邀请已绑定", exact: true })
      .waitFor();
    await uiRegister(page, phone, "邀请体验");
    await page.locator(".tab-bar").waitFor();
    assert.equal(await page.locator(".invite-bind-card").count(), 0);
    const token = await page.evaluate(() =>
      localStorage.getItem("orange-token"),
    );
    const me = (await api("/me", null, token)).data;
    assert.equal(me.invitation_required, false);
    assert.equal(me.inviter_code, platformCode);
    assert.match(me.invite_code, /^[A-Z0-9]{6}$/);
    assert.match(me.invite_code, /[A-Z]/);
    assert.match(me.invite_code, /[0-9]/);
    await page.locator(".tab-bar button").filter({ hasText: "我的" }).click();
    await page.getByRole("button", { name: /邀请好友/ }).click();
    await page.locator(".invite-poster-preview").waitFor();
    await page
      .locator(".invite-poster-preview")
      .evaluate((img) => img.decode());
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("link", { name: "保存海报", exact: true }).click();
    const download = await downloadPromise;
    const poster = path.join(output, "invite-poster.png");
    await download.saveAs(poster);
    const png = fs.readFileSync(poster);
    assert.equal(png.subarray(0, 8).toString("hex"), "89504e470d0a1a0a");
    assert.equal(png.readUInt32BE(16), 1080);
    assert.equal(png.readUInt32BE(20), 1600);
    const summary = (await api("/invite/summary", null, token)).data;
    const qr = (await api("/invite/qr/" + me.invite_code)).data;
    assert.equal(qr.url, summary.invite_url);
    assert.equal(qr.modules.length, qr.size);
    const decoded = spawnSync(
      "python",
      [
        "-c",
        "import cv2,numpy as np,sys; im=cv2.imdecode(np.fromfile(sys.argv[1],dtype=np.uint8),cv2.IMREAD_COLOR); value,_,_=cv2.QRCodeDetector().detectAndDecode(im[1130:1515,80:455]); print(value); assert value==sys.argv[2]",
        poster,
        summary.invite_url,
      ],
      { encoding: "utf8" },
    );
    assert.equal(decoded.status, 0, decoded.stderr || decoded.stdout);
    await page.getByRole("button", { name: "关闭", exact: true }).click();
    await page.locator(".app-top .icon-button").click();
    await page.getByRole("button", { name: "退出登录", exact: true }).click();
    await page.locator(".content-sheet").waitFor({ state: "hidden" });
    const beforeGate = streams;
    await uiRegister(page, directPhone, "待绑定体验");
    await page.locator(".invite-bind-card").waitFor();
    assert.equal(await page.locator(".app-surface").count(), 0);
    assert.equal(
      streams,
      beforeGate,
      "unbound registration must not start SSE",
    );
    const directToken = await page.evaluate(() =>
      localStorage.getItem("orange-token"),
    );
    const directUser = (await api("/me", null, directToken)).data;
    assert.equal(directUser.invitation_required, true);
    await page.getByLabel("邀请码", { exact: true }).fill("OOOOOO");
    await page.getByRole("button", { name: "绑定并进入", exact: true }).click();
    await page.locator(".invite-bind-card .form-error").waitFor();
    assert.equal(await page.locator(".app-surface").count(), 0);
    await page
      .getByLabel("邀请码", { exact: true })
      .fill(directUser.invite_code);
    await page.getByRole("button", { name: "绑定并进入", exact: true }).click();
    await page.waitForFunction(() =>
      /自己/.test(
        document.querySelector(".invite-bind-card .form-error")?.textContent ||
          "",
      ),
    );
    await page.screenshot({ path: path.join(output, "invite-gate.png") });
    await page
      .getByRole("button", { name: "暂不绑定，返回登录", exact: true })
      .click();
    await page.locator(".auth-sheet").waitFor();
    assert.equal(
      await page.evaluate(() => localStorage.getItem("orange-token")),
      null,
    );
    assert.equal(streams, beforeGate);
    await page
      .locator(".auth-sheet")
      .getByRole("button", { name: "登录", exact: true })
      .click();
    await page.locator(".invite-bind-card").waitFor();
    await page.getByLabel("邀请码", { exact: true }).fill(me.invite_code);
    await page.getByRole("button", { name: "绑定并进入", exact: true }).click();
    await page.locator(".tab-bar").waitFor();
    const boundToken = await page.evaluate(() =>
      localStorage.getItem("orange-token"),
    );
    const bound = (await api("/me", null, boundToken)).data;
    assert.equal(bound.invitation_required, false);
    assert.equal(bound.inviter_code, me.invite_code);
    const rebind = await api(
      "/invite/bind",
      { invite_code: platformCode },
      boundToken,
    );
    assert.ok(!rebind.response.ok);
    assert.ok(rebind.data.error);
    await api("/auth/sms", { phone: stalePhone, purpose: "register" });
    const stale = (
      await api("/auth/register", {
        phone: stalePhone,
        password: "InviteUi123!",
        code: "123456",
        nickname: "过期界面体验",
      })
    ).data;
    assert.equal(stale.user.invitation_required, true);
    const stalePage = await browser.newPage({
      viewport: { width: 390, height: 844 },
    });
    await stalePage.addInitScript(
      (session) => localStorage.setItem("orange-token", session),
      stale.token,
    );
    await stalePage.route("**/api/me", (route) =>
      route.fulfill({ json: { ...stale.user, invitation_required: false } }),
    );
    await stalePage.goto(base);
    await stalePage.locator(".invite-bind-card").waitFor();
    assert.equal(
      await stalePage.locator(".app-surface").count(),
      0,
      "real backend403 should gate stale client state",
    );
    await stalePage.close();
    const admin = await browser.newPage({
      viewport: { width: 1440, height: 1000 },
    });
    await admin.goto(base + "/admin");
    await admin.getByRole("button", { name: "邀请管理", exact: true }).click();
    await admin
      .getByRole("heading", { name: "平台邀请入口", exact: true })
      .waitFor();
    await admin.getByRole("cell", { name: phone, exact: true }).waitFor();
    await admin.screenshot({
      path: path.join(output, "invite-admin.png"),
      fullPage: true,
    });
    await admin.close();
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        prebindAutoBindsOnRegistration: true,
        directRegistrationGated: true,
        refusalLogsOut: true,
        badCodeRejected: true,
        selfCodeRejected: true,
        rebindRejected: true,
        stale403Gated: true,
        posterPng: "1080x1600",
        qrDecoded: decoded.stdout.trim(),
        personalCode: me.invite_code,
        adminInvitations: true,
        runtimeErrors: errors,
        output,
      }),
    );
  } finally {
    await browser.close();
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
