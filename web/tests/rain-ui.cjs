const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const base = "http://127.0.0.1:38180";
async function api(route, body, token, method) {
  const response = await fetch(base + "/api" + route, {
    method: method || (body ? "POST" : "GET"),
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: "Bearer " + token } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const result = await response.json();
  assert.ok(response.ok, result.error);
  return result;
}
async function readyPacket(page, kind) {
  await page.waitForFunction(
    (kind) => {
      const field = document
        .querySelector(".rain-field")
        ?.getBoundingClientRect();
      return (
        field &&
        [...document.querySelectorAll(".rain-falling-packet")].some((node) => {
          const box = node.getBoundingClientRect();
          return (
            !node.disabled &&
            (kind === "large"
              ? node.classList.contains("large")
              : !node.classList.contains("large")) &&
            box.top > field.top + 15 &&
            box.bottom < field.bottom - 10
          );
        })
      );
    },
    kind,
    { timeout: 12000 },
  );
  return page.evaluate((kind) => {
    const field = document.querySelector(".rain-field").getBoundingClientRect();
    const node = [...document.querySelectorAll(".rain-falling-packet")].find(
      (node) => {
        const box = node.getBoundingClientRect();
        return (
          !node.disabled &&
          (kind === "large"
            ? node.classList.contains("large")
            : !node.classList.contains("large")) &&
          box.top > field.top + 15 &&
          box.bottom < field.bottom - 10
        );
      },
    );
    const box = node.getBoundingClientRect();
    return {
      id: node.dataset.packetId,
      x: box.x + box.width / 2,
      y: box.y + box.height / 2,
    };
  }, kind);
}
(async () => {
  const original = await api("/admin/rain-config"),
    originalControls = await api("/admin/hall-request-config");
  let browser;
  try {
    const output = path.join(__dirname, "output");
    fs.mkdirSync(output, { recursive: true });
    const platform = await api("/admin/invitations");
    let suffix = Number(String(Date.now()).slice(-8));
    async function account() {
      const phone = "132" + String(suffix++ % 1e8).padStart(8, "0");
      await api("/invite/prebind", {
        phone,
        invite_code: platform.platform_code,
      });
      await api("/auth/sms", { phone, purpose: "register" });
      return api("/auth/register", {
        phone,
        password: "RainUi123!",
        code: "123456",
        nickname: "红包雨界面验证",
      });
    }
    const first = await account(),
      second = await account();
    const settings = {
      ...original,
      enabled: true,
      duration_seconds: 22,
      packet_count: 8,
      small_min_coins: 1,
      small_max_coins: 1,
      grand_probability: 0,
      max_total_coins: 1000,
      fall_speed: 0.6,
      max_visible: 3,
    };
    await api("/admin/rain-config", settings, null, "PUT");
    await api(
      "/admin/hall-request-config",
      {
        ...originalControls,
        enabled: true,
        packet_enabled: true,
        product_enabled: true,
        cooldown_seconds: 5,
        daily_limit: 20,
      },
      null,
      "PUT",
    );
    browser = await chromium.launch({
      executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
      headless: true,
    });
    const context = await browser.newContext({
      viewport: { width: 390, height: 844 },
    });
    await context.addInitScript((token) => {
      localStorage.setItem("orange-token", token);
      localStorage.setItem("welcome_seen", "1");
      const start = AudioBufferSourceNode.prototype.start;
      window.__coinStarts = 0;
      AudioBufferSourceNode.prototype.start = function (...args) {
        window.__coinStarts++;
        return start.apply(this, args);
      };
    }, first.token);
    const page = await context.newPage(),
      errors = [];
    let claimRequests = 0,
      statusRequests = 0,
      rewardStarts = 0;
    page.on("pageerror", (e) => errors.push(e.message));
    page.on("request", (r) => {
      if (r.url().endsWith("/api/rain/claim")) claimRequests++;
      if (r.url().endsWith("/api/rain/status")) statusRequests++;
      if (r.url().includes("/api/rewards/")) rewardStarts++;
    });
    await page.goto(base);
    await page.locator(".hall-request-actions").waitFor();
    const packetResponse = page.waitForResponse(
      (r) =>
        r.url().endsWith("/api/hall/request") &&
        r.request().method() === "POST",
    );
    await page.getByRole("button", { name: "我要红包", exact: true }).click();
    const requested = await (await packetResponse).json();
    await page
      .locator('[data-event-id="' + requested.event.id + '"]')
      .waitFor();
    assert.equal(await page.locator(".request-product").isDisabled(), true);
    await page.waitForFunction(
      () => !document.querySelector(".request-product")?.disabled,
    );
    const productResponse = page.waitForResponse(
      (r) =>
        r.url().endsWith("/api/hall/request") &&
        r.request().method() === "POST",
    );
    await page.getByRole("button", { name: "好物捡漏", exact: true }).click();
    const product = await (await productResponse).json();
    await page.locator('[data-event-id="' + product.event.id + '"]').waitFor();
    await page.screenshot({ path: path.join(output, "rain-hall-actions.png") });
    const scheduled = await api("/admin/rain/start", { countdown_seconds: 10 });
    await page.locator(".rain-countdown-overlay").waitFor({ timeout: 11000 });
    await page.screenshot({ path: path.join(output, "rain-countdown.png") });
    await page
      .getByRole("button", { name: "关闭红包雨倒计时", exact: true })
      .click();
    assert.equal(await page.locator(".rain-countdown-overlay").count(), 0);
    await page.locator(".rain-overlay").waitFor({ timeout: 15000 });
    await page.locator(".rain-sky").evaluate((img) => img.decode());
    const point = await readyPacket(page, "small");
    assert.ok(
      (await page.locator(".rain-falling-packet").count()) <=
        settings.max_visible,
    );
    await page.screenshot({ path: path.join(output, "rain-active.png") });
    const balance = Number(
      (await page.locator(".balance-pill b").textContent()).replaceAll(",", ""),
    );
    await page.route(
      "**/api/rain/claim",
      async (route) => {
        await new Promise((resolve) => setTimeout(resolve, 350));
        await route.continue();
      },
      { times: 1 },
    );
    const receipt = page.waitForResponse((r) =>
      r.url().endsWith("/api/rain/claim"),
    );
    await page.mouse.click(point.x, point.y);
    const pending = page.locator('[data-packet-id="' + point.id + '"]');
    assert.equal(await pending.isDisabled(), true);
    await pending.evaluate((button) => button.click());
    const credited = await (await receipt).json();
    assert.equal(credited.reward, 1);
    assert.equal(credited.credited, true);
    await page.locator(".rain-reward-float").first().waitFor();
    assert.equal(
      Number(
        (await page.locator(".balance-pill b").textContent()).replaceAll(
          ",",
          "",
        ),
      ) - balance,
      1,
    );
    assert.equal(claimRequests, 1);
    assert.equal(await page.evaluate(() => window.__coinStarts), 1);
    await page.waitForTimeout(120);
    await page.screenshot({
      path: path.join(output, "rain-small-credited.png"),
    });
    await page.waitForTimeout(1500);
    const replayPoint = await readyPacket(page, "small");
    const external = await api(
      "/rain/claim",
      { packet_id: replayPoint.id },
      first.token,
    );
    assert.equal(external.credited, true);
    const replayResponse = page.waitForResponse((r) =>
      r.url().endsWith("/api/rain/claim"),
    );
    await page.mouse.click(replayPoint.x, replayPoint.y);
    const replay = await (await replayResponse).json();
    assert.equal(replay.already_claimed, true);
    assert.equal(replay.credited, false);
    await page.waitForTimeout(150);
    assert.equal(await page.evaluate(() => window.__coinStarts), 1);
    assert.equal(await page.locator(".rain-reward-float").count(), 0);
    const videoContext = await browser.newContext({
      viewport: { width: 390, height: 844 },
      recordVideo: {
        dir: path.join(output, "rain-video"),
        size: { width: 390, height: 844 },
      },
    });
    await videoContext.addInitScript((token) => {
      localStorage.setItem("orange-token", token);
      localStorage.setItem("welcome_seen", "1");
    }, second.token);
    const videoPage = await videoContext.newPage();
    const recordingAt = Date.now();
    await videoPage.goto(base);
    await videoPage.locator(".rain-overlay").waitFor();
    await videoPage.locator(".rain-falling-packet").first().waitFor();
    const clipAt = (Date.now() - recordingAt) / 1000;
    await videoPage.waitForTimeout(5200);
    const rawVideo = await videoPage.video().path();
    await videoContext.close();
    const clip = path.join(output, "rain-demo.webm");
    const ffmpeg =
      "C:/Users/Arlo/AppData/Local/ms-playwright/ffmpeg-1013/ffmpeg-win64.exe";
    const videoResult = spawnSync(
      ffmpeg,
      [
        "-y",
        "-i",
        rawVideo,
        "-ss",
        String(clipAt),
        "-t",
        "5",
        "-c:v",
        "libvpx-vp9",
        "-an",
        clip,
      ],
      { encoding: "utf8" },
    );
    assert.equal(videoResult.status, 0, "recorded rain video must encode");
    const status = await api("/rain/status");
    if (status.phase === "active")
      await page.waitForTimeout(
        Math.max(0, Date.parse(status.round.end_at) - Date.now()) + 250,
      );
    await api(
      "/admin/rain-config",
      { ...settings, grand_probability: 1, grand_coins: 88 },
      null,
      "PUT",
    );
    const admin = await browser.newPage({
      viewport: { width: 1440, height: 1100 },
    });
    await admin.goto(base + "/admin");
    await admin.getByRole("button", { name: "红包雨", exact: true }).click();
    await admin
      .getByRole("heading", { name: "红包雨设置", exact: true })
      .waitFor();
    await admin.getByLabel("同时可见红包数", { exact: true }).fill("2");
    await admin.getByLabel("大奖概率（%）", { exact: true }).fill("100");
    await admin.getByRole("button", { name: "保存设置", exact: true }).click();
    await admin.getByText("红包雨设置已保存", { exact: true }).waitFor();
    await admin.screenshot({
      path: path.join(output, "rain-admin.png"),
      fullPage: true,
    });
    await admin
      .getByRole("button", { name: "立即开一场", exact: true })
      .click();
    await admin
      .getByText("已安排开场，倒计时即将开始", { exact: true })
      .waitFor();
    await page.locator(".rain-countdown-overlay").waitFor({ timeout: 10000 });
    await page
      .getByRole("button", { name: "关闭红包雨倒计时", exact: true })
      .click();
    await page.locator(".rain-overlay").waitFor({ timeout: 18000 });
    const large = await readyPacket(page, "large");
    const priorCoins = Number(
      (await page.locator(".balance-pill b").textContent()).replaceAll(",", ""),
    );
    await page.mouse.click(large.x, large.y);
    await page
      .getByRole("heading", { name: "喜中大奖", exact: true })
      .waitFor();
    await page
      .getByText("观看视频广告，领取这份大奖", { exact: true })
      .waitFor();
    await page.screenshot({ path: path.join(output, "rain-large.png") });
    assert.equal(
      Number(
        (await page.locator(".balance-pill b").textContent()).replaceAll(
          ",",
          "",
        ),
      ),
      priorCoins,
    );
    assert.equal(rewardStarts, 0);
    await page.getByRole("button", { name: "关闭", exact: true }).click();
    const reduced = await browser.newPage({
      viewport: { width: 390, height: 844 },
      reducedMotion: "reduce",
    });
    await reduced.addInitScript((token) => {
      localStorage.setItem("orange-token", token);
      localStorage.setItem("welcome_seen", "1");
    }, second.token);
    await reduced.goto(base);
    await reduced.locator(".rain-falling-packet").first().waitFor();
    assert.equal(
      await reduced
        .locator(".rain-falling-packet")
        .first()
        .evaluate((node) => getComputedStyle(node).animationName),
      "none",
    );
    await reduced.close();
    const production = await browser.newPage({
      viewport: { width: 390, height: 844 },
    });
    await production.addInitScript((token) => {
      localStorage.setItem("orange-token", token);
      localStorage.setItem("welcome_seen", "1");
    }, first.token);
    const config = await api("/config");
    let prodJoin = 0,
      prodClaim = 0;
    production.on("request", (r) => {
      if (r.url().endsWith("/api/rain/join")) prodJoin++;
      if (r.url().endsWith("/api/rain/claim")) prodClaim++;
    });
    await production.route("**/api/config", (route) =>
      route.fulfill({
        json: { ...config, device_trust_enabled: true, ad_mode: "csj" },
      }),
    );
    await production.goto(base);
    await production
      .getByRole("link", { name: "打开App接红包", exact: true })
      .waitFor();
    assert.equal(prodJoin, 0);
    assert.equal(prodClaim, 0);
    assert.equal(await production.locator(".rain-falling-packet").count(), 0);
    await production.screenshot({
      path: path.join(output, "rain-web-app-required.png"),
    });
    await production.close();
    await page.getByRole("button", { name: "关闭红包雨", exact: true }).click();
    assert.equal(await page.locator(".rain-overlay").count(), 0);
    await page.locator(".rain-banner").click();
    await page.locator(".rain-overlay").waitFor();
    await page.locator(".tab-bar button").filter({ hasText: "捡漏" }).click();
    assert.equal(await page.locator(".hall-activity").count(), 0);
    const requestsAfterExit = statusRequests;
    await page.waitForTimeout(5200);
    assert.equal(statusRequests, requestsAfterExit);
    assert.deepEqual(errors, []);
    await admin.close();
    await context.close();
    console.log(
      JSON.stringify({
        passed: true,
        manualServerEvents: true,
        cooldown: true,
        countdownClosable: true,
        serverPacketWindows: true,
        visibleBound: true,
        realSmallCredit: true,
        pendingPreventsDuplicate: true,
        idempotentNoExtraSound: true,
        largeNoFakeAdCredit: true,
        adminConfigAndStart: true,
        productionAppOnly: true,
        leaveHallStopsPolling: true,
        reducedMotion: true,
        runtimeErrors: errors,
        video: clip,
        output,
      }),
    );
  } finally {
    if (browser) await browser.close();
    await api("/admin/rain-config", original, null, "PUT");
    await api("/admin/hall-request-config", originalControls, null, "PUT");
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
