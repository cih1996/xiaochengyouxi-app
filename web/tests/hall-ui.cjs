const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");

(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  try {
    const page = await browser.newPage({
      viewport: { width: 390, height: 844 },
    });
    const errors = [],
      streamRequests = [],
      historyRequests = [];
    const testPhone = "136" + String(Date.now()).slice(-8);
    const requestApi = async (path, body) => {
      const response = await fetch("http://127.0.0.1:38180/api" + path, {
        method: body ? "POST" : "GET",
        headers: { "Content-Type": "application/json" },
        body: body ? JSON.stringify(body) : undefined,
      });
      const data = await response.json();
      assert.ok(response.ok, data.error);
      return data;
    };
    const invite = await requestApi("/admin/invitations");
    await requestApi("/invite/prebind", {
      phone: testPhone,
      invite_code: invite.platform_code,
    });
    await requestApi("/auth/sms", { phone: testPhone, purpose: "register" });
    await requestApi("/auth/register", {
      phone: testPhone,
      password: "HallUi123!",
      code: "123456",
      nickname: "大厅会话体验",
    });
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("request", (request) => {
      if (request.url().endsWith("/api/hall/stream"))
        streamRequests.push(request.headers());
      if (/\/api\/(?:admin\/)?messages/.test(request.url()))
        historyRequests.push(request.url());
    });
    await page.addInitScript(() => localStorage.setItem("welcome_seen", "1"));
    await page.goto("http://127.0.0.1:38180");
    await page.getByRole("button", { name: "登录领福利", exact: true }).click();
    await page.getByLabel("手机号", { exact: true }).fill(testPhone);
    await page.getByLabel("密码", { exact: true }).fill("HallUi123!");
    await page
      .locator(".auth-sheet")
      .getByRole("button", { name: "登录", exact: true })
      .click();
    await page.locator(".auth-sheet").waitFor({ state: "hidden" });
    await page.locator('[data-event-kind="text"]').waitFor();
    await page.locator('[data-event-kind="red_packet"]').first().waitFor();
    await page
      .locator('[data-event-kind="product"]')
      .first()
      .waitFor({ timeout: 12000 });
    const ids = await page
      .locator("[data-event-id]")
      .evaluateAll((nodes) => nodes.map((node) => node.dataset.eventId));
    assert.equal(
      ids.length,
      3,
      "fresh stream should have welcome, packet, and product",
    );
    assert.equal(streamRequests.length, 1);
    assert.match(streamRequests[0].authorization, /^Bearer /);
    assert.equal(streamRequests[0]["last-event-id"], undefined);
    await page.locator(".tab-bar button").filter({ hasText: "捡漏" }).click();
    await page.locator(".tab-bar button").filter({ hasText: "大厅" }).click();
    assert.deepEqual(
      await page
        .locator("[data-event-id]")
        .evaluateAll((nodes) => nodes.map((node) => node.dataset.eventId)),
      ids,
      "tab switch must preserve in-memory events",
    );
    const storage = await page.evaluate(() => ({
      local: Object.keys(localStorage).sort(),
      session: Object.keys(sessionStorage),
    }));
    assert.deepEqual(
      storage,
      { local: ["orange-token", "welcome_seen"], session: [] },
      "hall must not write history storage",
    );
    await page.reload();
    await page.locator('[data-event-kind="red_packet"]').first().waitFor();
    const newIds = await page
      .locator("[data-event-id]")
      .evaluateAll((nodes) => nodes.map((node) => node.dataset.eventId));
    assert.equal(
      newIds.length,
      2,
      "reload should start a new stream, without prior product history",
    );
    assert.ok(
      newIds.every((id) => !ids.includes(id)),
      "reload must use fresh event IDs",
    );
    await page.locator(".tab-bar button").filter({ hasText: "我的" }).click();
    await page.locator(".app-top .icon-button").click();
    await page.getByRole("button", { name: "退出登录", exact: true }).click();
    await page.locator(".content-sheet").waitFor({ state: "hidden" });
    await page.locator(".auth-sheet").waitFor();
    await page.getByRole("button", { name: "关闭", exact: true }).click();
    await page.locator(".tab-bar button").filter({ hasText: "大厅" }).click();
    await page
      .getByRole("button", { name: "登录领福利", exact: true })
      .waitFor();
    await page.waitForTimeout(6200);
    assert.equal(
      await page.locator("[data-event-id]").count(),
      0,
      "logout must abort the stream and clear session events",
    );
    assert.equal(streamRequests.length, 2, "no reconnect while logged out");
    assert.deepEqual(historyRequests, []);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        authenticatedStream: true,
        eventKinds: ["text", "red_packet", "product"],
        tabMemoryPreserved: true,
        reloadStartsFresh: true,
        historyStorage: false,
        logoutStopsStream: true,
        historyRequests: 0,
        runtimeErrors: errors,
      }),
    );
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
