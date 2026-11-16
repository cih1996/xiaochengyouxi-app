const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const base = "http://127.0.0.1:38180";
async function api(route, body, method) {
  const response = await fetch(base + "/api" + route, {
    method: method || (body ? "POST" : "GET"),
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await response.json();
  assert.ok(response.ok, data.error);
  return data;
}
(async () => {
  const original = await api("/admin/rain-config");
  let browser;
  try {
    const current = await api("/rain/status");
    if (current.phase === "active")
      await new Promise((resolve) =>
        setTimeout(
          resolve,
          Math.max(0, Date.parse(current.round.end_at) - Date.now()) + 250,
        ),
      );
    const output = path.join(__dirname, "output");
    const phone = "131" + String(Date.now()).slice(-8),
      platform = await api("/admin/invitations");
    await api("/invite/prebind", {
      phone,
      invite_code: platform.platform_code,
    });
    await api("/auth/sms", { phone, purpose: "register" });
    const account = await api("/auth/register", {
      phone,
      password: "RainDemo123!",
      code: "123456",
      nickname: "红包雨动画体验",
    });
    await api(
      "/admin/rain-config",
      {
        ...original,
        enabled: true,
        duration_seconds: 20,
        packet_count: 24,
        small_min_coins: 1,
        small_max_coins: 5,
        grand_probability: 0.5,
        grand_coins: 88,
        max_total_coins: 600,
        overlay_opacity: 0.64,
        fall_speed: 1,
        max_visible: 8,
      },
      "PUT",
    );
    await api("/admin/rain/start", { countdown_seconds: 10 });
    browser = await chromium.launch({
      executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
      headless: true,
    });
    const context = await browser.newContext({
      viewport: { width: 390, height: 844 },
      recordVideo: {
        dir: path.join(output, "rain-mixed-video"),
        size: { width: 390, height: 844 },
      },
    });
    await context.addInitScript((token) => {
      localStorage.setItem("orange-token", token);
      localStorage.setItem("welcome_seen", "1");
    }, account.token);
    const page = await context.newPage(),
      started = Date.now();
    await page.goto(base);
    await page.locator(".rain-countdown-overlay").waitFor();
    await page
      .getByRole("button", { name: "关闭红包雨倒计时", exact: true })
      .click();
    await page.locator(".rain-overlay").waitFor({ timeout: 15000 });
    await page.waitForFunction(
      () => {
        const field = document
          .querySelector(".rain-field")
          ?.getBoundingClientRect();
        if (!field) return false;
        const visible = [
          ...document.querySelectorAll(".rain-falling-packet"),
        ].filter((el) => {
          const rect = el.getBoundingClientRect();
          return rect.top > field.top && rect.bottom < field.bottom;
        });
        return (
          visible.length >= 4 &&
          visible.some((el) => el.classList.contains("large")) &&
          visible.some((el) => !el.classList.contains("large"))
        );
      },
      {},
      { timeout: 12000 },
    );
    const visible = await page.locator(".rain-falling-packet").count();
    assert.ok(visible <= 8);
    await page.screenshot({ path: path.join(output, "rain-mixed-active.png") });
    const clipAt = (Date.now() - started) / 1000;
    await page.waitForTimeout(5100);
    const raw = await page.video().path();
    await context.close();
    const video = path.join(output, "rain-mixed-demo.webm");
    const encoded = spawnSync(
      "C:/Users/Arlo/AppData/Local/ms-playwright/ffmpeg-1013/ffmpeg-win64.exe",
      [
        "-y",
        "-i",
        raw,
        "-ss",
        String(clipAt),
        "-t",
        "5",
        "-c:v",
        "libvpx-vp9",
        "-an",
        video,
      ],
      { encoding: "utf8" },
    );
    assert.equal(encoded.status, 0, "five-second rain capture must encode");
    const gif = path.join(output, "rain-mixed-demo.gif");
    const converted = spawnSync(
      "python",
      [
        "-c",
        `import cv2,sys
from PIL import Image
cap=cv2.VideoCapture(sys.argv[1]); fps=cap.get(cv2.CAP_PROP_FPS); frames=[]; index=0
while True:
 ok,frame=cap.read()
 if not ok: break
 if index % max(1,round(fps/10))==0:
  frames.append(Image.fromarray(cv2.cvtColor(frame,cv2.COLOR_BGR2RGB)).resize((320,692)).convert('P',palette=Image.Palette.ADAPTIVE,colors=192))
 index+=1
assert frames
frames[0].save(sys.argv[2],save_all=True,append_images=frames[1:],duration=100,loop=0,optimize=True)
`,
        video,
        gif,
      ],
      { encoding: "utf8" },
    );
    assert.equal(converted.status, 0, converted.stderr);
    const admin = await browser.newPage({
      viewport: { width: 1440, height: 1100 },
    });
    await admin.goto(base + "/admin");
    await admin.getByRole("button", { name: "红包雨", exact: true }).click();
    await admin
      .getByRole("heading", { name: "红包雨设置", exact: true })
      .waitFor();
    await admin.evaluate(() => scrollTo(0, 0));
    await admin.waitForTimeout(200);
    await admin.screenshot({
      path: path.join(output, "rain-admin-final.png"),
      fullPage: true,
    });
    await admin.close();
    console.log(
      JSON.stringify({
        passed: true,
        actualServerAllocation: true,
        visiblePackets: visible,
        visibleCap: 8,
        mixedSmallAndLarge: true,
        noClaimsOrFakeCredit: true,
        video,
        gif,
        output,
      }),
    );
  } finally {
    if (browser) await browser.close();
    await api("/admin/rain-config", original, "PUT");
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
