const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const base = "http://127.0.0.1:38180";
async function api(route, body, token) {
  const response = await fetch(base + "/api" + route, {
    method: body ? "POST" : "GET",
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
(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  try {
    const output = path.join(__dirname, "output");
    fs.mkdirSync(output, { recursive: true });
    const platform = await api("/admin/invitations");
    let suffix = Number(String(Date.now()).slice(-8));
    async function account(bound) {
      const phone = "135" + String(suffix++ % 1e8).padStart(8, "0");
      if (bound)
        await api("/invite/prebind", {
          phone,
          invite_code: platform.platform_code,
        });
      await api("/auth/sms", { phone, purpose: "register" });
      return {
        phone,
        ...(await api("/auth/register", {
          phone,
          password: "CheckinUi123!",
          code: "123456",
          nickname: "欢迎签到体验",
        })),
      };
    }
    const first = await account(true);
    assert.equal(first.user.checked_in_today, false);
    const page = await browser.newPage({
        viewport: { width: 390, height: 844 },
      }),
      errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(base);
    await page.locator(".welcome-screen").waitFor();
    assert.equal(await page.locator(".auth-sheet").count(), 0);
    await page.locator(".welcome-mascot").evaluate((img) => img.decode());
    await page.screenshot({ path: path.join(output, "welcome.png") });
    await page.getByRole("button", { name: "开启小欢喜", exact: true }).click();
    await page.locator(".auth-sheet").waitFor();
    assert.equal(
      await page.evaluate(() => localStorage.getItem("welcome_seen")),
      "1",
    );
    await page.getByLabel("手机号", { exact: true }).fill(first.phone);
    await page.getByLabel("密码", { exact: true }).fill("CheckinUi123!");
    await page.getByRole("button", { name: "显示密码", exact: true }).click();
    assert.equal(
      await page.getByLabel("密码", { exact: true }).getAttribute("type"),
      "text",
    );
    await page.getByRole("button", { name: "隐藏密码", exact: true }).click();
    assert.equal(
      await page.getByLabel("密码", { exact: true }).getAttribute("type"),
      "password",
    );
    await page.screenshot({ path: path.join(output, "login-polished.png") });
    await page
      .locator(".auth-sheet")
      .getByRole("button", { name: "登录", exact: true })
      .click();
    await page.locator(".auth-sheet").waitFor({ state: "hidden" });
    const checkin = page.locator(".checkin-card");
    await page.locator(".checkin-pending").waitFor();
    assert.equal(await checkin.isDisabled(), false);
    assert.match(
      await checkin
        .locator(".tool-icon svg")
        .evaluate((node) => getComputedStyle(node).animationName),
      /checkinBounce/,
    );
    await page.waitForTimeout(450);
    await page.screenshot({ path: path.join(output, "checkin-before.png") });
    await page.route(
      "**/api/check-in",
      (route) =>
        route.fulfill({ status: 500, json: { error: "签到服务测试错误原文" } }),
      { times: 1 },
    );
    await checkin.click();
    await page.getByText("签到服务测试错误原文", { exact: true }).waitFor();
    assert.equal(await checkin.isDisabled(), false);
    assert.equal(await page.locator(".checkin-pending").count(), 1);
    assert.equal(await page.locator(".stage-won").count(), 0);
    const before = Number(
      (await page.locator(".balance-pill b").textContent()).replaceAll(",", ""),
    );
    await checkin.click();
    await page.locator(".stage-won").waitFor();
    const reward = Number(
      (await page.locator(".won-number").textContent()).replace(/[^\d]/g, ""),
    );
    assert.equal(await checkin.isDisabled(), true);
    assert.equal(await page.locator(".checkin-pending").count(), 0);
    assert.equal(
      await checkin
        .locator(".tool-icon svg")
        .evaluate((node) => getComputedStyle(node).animationName),
      "none",
    );
    await page.getByRole("button", { name: "开心收下", exact: true }).click();
    const after = Number(
      (await page.locator(".balance-pill b").textContent()).replaceAll(",", ""),
    );
    assert.equal(after - before, reward);
    await page.waitForTimeout(1700);
    await page.screenshot({ path: path.join(output, "checkin-after.png") });
    await page.reload();
    await page.getByRole("button", { name: /今日已签到/ }).waitFor();
    assert.equal(await page.locator(".welcome-screen").count(), 0);
    assert.equal(await page.locator(".checkin-card").isDisabled(), true);
    assert.equal(await page.locator(".checkin-pending").count(), 0);
    await page.screenshot({ path: path.join(output, "checkin-reloaded.png") });
    await page.locator(".tab-bar button").filter({ hasText: "我的" }).click();
    await page.locator(".app-top .icon-button").click();
    await page.getByRole("button", { name: "退出登录", exact: true }).click();
    await page.locator(".auth-sheet").waitFor();
    assert.equal(await page.locator(".welcome-screen").count(), 0);
    assert.equal(
      await page.evaluate(() => localStorage.getItem("orange-token")),
      null,
    );
    assert.equal(
      await page.getByLabel("密码", { exact: true }).inputValue(),
      "",
    );
    const race = await account(true);
    const racePage = await browser.newPage({
      viewport: { width: 390, height: 844 },
    });
    await racePage.addInitScript(
      (token) => localStorage.setItem("orange-token", token),
      race.token,
    );
    await racePage.goto(base);
    await racePage.locator(".checkin-pending").waitFor();
    assert.equal(await racePage.locator(".welcome-screen").count(), 0);
    await api("/check-in", {}, race.token);
    await racePage.locator(".checkin-card").click();
    await racePage.locator(".checkin-done").waitFor();
    assert.equal(await racePage.locator(".checkin-card").isDisabled(), true);
    assert.equal(await racePage.locator(".stage-won").count(), 0);
    await racePage.close();
    const unbound = await account(false);
    const gate = await browser.newPage({
      viewport: { width: 390, height: 844 },
    });
    await gate.addInitScript(
      (token) => localStorage.setItem("orange-token", token),
      unbound.token,
    );
    await gate.goto(base);
    await gate.locator(".invite-bind-card").waitFor();
    assert.equal(await gate.locator(".welcome-screen").count(), 0);
    assert.equal(await gate.locator(".app-surface").count(), 0);
    await gate.close();
    for (const route of [
      "/privacy",
      "/terms",
      "/i/" + platform.platform_code,
      "/admin",
    ]) {
      const side = await browser.newPage({
        viewport: { width: 390, height: 844 },
      });
      await side.goto(base + route);
      assert.equal(await side.locator(".welcome-screen").count(), 0);
      await side.close();
    }
    const reduced = await browser.newPage({
      viewport: { width: 390, height: 844 },
      reducedMotion: "reduce",
    });
    const fresh = await account(true);
    await reduced.addInitScript(
      (token) => localStorage.setItem("orange-token", token),
      fresh.token,
    );
    await reduced.goto(base);
    await reduced.locator(".checkin-pending").waitFor();
    assert.equal(
      await reduced
        .locator(".checkin-card .tool-icon svg")
        .evaluate((node) => getComputedStyle(node).animationName),
      "none",
    );
    await reduced.close();
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        welcomeFirstVisitOnly: true,
        sideRoutesSkipWelcome: true,
        passwordVisibility: true,
        uncheckedHighlighted: true,
        failurePreservesUnchecked: true,
        checkedDisablesAndStopsAnimation: true,
        reloadRestoresChecked: true,
        duplicateRaceRefreshesState: true,
        logoutToLogin: true,
        inviteGatePreserved: true,
        reducedMotion: true,
        reward,
        runtimeErrors: errors,
        output,
      }),
    );
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
