const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs"),
  path = require("node:path");
const dist = path.join(__dirname, "../dist"),
  output = path.join(__dirname, "../../output");
const origin = "https://mail-preview.test";
const state = {
  enabled: true,
  reason: "邮件服务尚未配置",
  entry: "/mailbox/",
  deny: false,
  statusReads: 0,
  opensAtStatus: [],
  tickets: 0,
  posts: [],
  unauthenticatedRequests: 0,
  unexpected: [],
};
async function fixture(page) {
  await page.context().route("**/*", async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const json = (value) => route.fulfill({ json: value });
    if (url.pathname === "/mailbox/") {
      if (request.method() === "POST")
        state.posts.push({
          url: request.url(),
          fields: [...new URLSearchParams(request.postData()).keys()],
          ticket: new URLSearchParams(request.postData()).get("_ticket"),
        });
      return route.fulfill({
        contentType: "text/html",
        headers: { "X-Frame-Options": "SAMEORIGIN" },
        body: '<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8"></head><body><p>邮件窗口测试页面</p><textarea aria-label="草稿"></textarea></body></html>',
      });
    }
    if (url.pathname.startsWith("/api/")) {
      const endpoint = url.pathname.slice(4);
      if (endpoint === "/config")
        return json({
          admin_auth_required: true,
          environment: "production",
          ad_mode: "csj",
          sms_mode: "mock",
          payment_mode: "mock",
        });
      if (endpoint === "/tasks" || endpoint === "/products") return json([]);
      if (endpoint === "/products/runtime") return json({ hero_image: null, revision: 1 });
      if (endpoint === "/admin/login") {
        const body = request.postDataJSON();
        return body.username === "admin" && body.password === "MailUi123!"
          ? json({
              token: "mock-admin-session",
              username: "admin",
              expires_at: "2026-11-12T00:00:00Z",
            })
          : route.fulfill({
              status: 401,
              json: { error: "管理账号或密码错误" },
            });
      }
      if (request.headers()["x-admin-token"] !== "mock-admin-session")
        state.unauthenticatedRequests++;
      if (
        state.deny ||
        request.headers()["x-admin-token"] !== "mock-admin-session"
      )
        return route.fulfill({
          status: 401,
          json: { error: "管理登录已过期" },
        });
      if (endpoint === "/admin/mail/status") {
        state.statusReads++;
        state.opensAtStatus.push(await page.evaluate(() => window.__mailOpens));
        return json({
          enabled: state.enabled,
          entry_path: state.entry,
          ...(!state.enabled ? { error: state.reason } : {}),
        });
      }
      if (endpoint === "/admin/mail/session") {
        state.tickets++;
        return route.fulfill({
          json: {
            ticket: "one-use-test-ticket-" + state.tickets,
            entry_path: state.entry,
          },
          headers: {
            "Set-Cookie":
              "orange_mail_admin=mock-admin-session; Path=/mailbox/; Secure; HttpOnly; SameSite=Lax",
          },
        });
      }
      const records = {
        "/admin/account": {
          username: "admin",
          auth_required: true,
          session_expires_at: "2026-11-12T00:00:00Z",
        },
        "/admin/overview": { total_users: 0, ad_platforms: [] },
        "/admin/rewards": [],
        "/admin/withdrawals": [],
        "/admin/settings": {},
        "/admin/hall-strategy": {},
        "/admin/invitations": { records: [], platform_code: "TEST26" },
      };
      if (Object.hasOwn(records, endpoint)) return json(records[endpoint]);
      state.unexpected.push(endpoint);
      return route.fulfill({ status: 404, json: { error: "未提供接口" } });
    }
    const file = url.pathname.startsWith("/assets/")
      ? path.join(dist, decodeURIComponent(url.pathname.slice(1)))
      : path.join(dist, "index.html");
    return route.fulfill({
      contentType: {
        ".html": "text/html",
        ".js": "text/javascript",
        ".css": "text/css",
        ".png": "image/png",
      }[path.extname(file)],
      body: fs.readFileSync(file),
    });
  });
}
(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  const errors = [];
  try {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 1050 },
    });
    const page = await context.newPage();
    const popups = [];
    context.on("page", (popup) => popups.push(popup));
    page.on("pageerror", (error) => errors.push(error.message));
    await page.addInitScript(() => {
      const open = window.open.bind(window);
      window.__mailOpens = 0;
      window.__blockMail = false;
      window.open = (...args) => {
        window.__mailOpens++;
        return window.__blockMail ? null : open(...args);
      };
    });
    await fixture(page);
    await page.goto(origin + "/admin?tab=mail");
    await page.locator(".admin-login-card").waitFor();
    assert.equal(state.statusReads, 0);
    await page.getByLabel("管理密码", { exact: true }).fill("MailUi123!");
    await page
      .getByRole("button", { name: "登录管理中心", exact: true })
      .click();
    await page.locator(".admin-mail-entry").waitFor();
    assert.equal(
      await page
        .locator(".admin-sidebar")
        .getByText("邮件", { exact: true })
        .count(),
      0,
    );
    assert.equal(await page.locator("iframe").count(), 0);
    assert.equal(
      (await page.locator(".admin-topbar > span").innerText()).replace(
        /\s+/g,
        " ",
      ),
      "工作台 数据概览",
    );
    assert.equal(state.statusReads, 0, "old mail link must not open a popup");
    assert.equal(popups.length, 0);

    const popupReady = page.waitForEvent("popup");
    await page.locator(".admin-mail-entry").click();
    const popup = await popupReady;
    await popup.getByText("邮件窗口测试页面", { exact: true }).waitFor();
    assert.equal(state.posts.length, 1);
    assert.deepEqual(
      state.opensAtStatus,
      [1],
      "popup opens before asynchronous status request",
    );
    const first = state.posts[0],
      postedUrl = new URL(first.url);
    assert.equal(postedUrl.origin, origin);
    assert.equal(postedUrl.pathname, "/mailbox/");
    assert.equal(postedUrl.searchParams.get("_task"), "login");
    assert.equal(postedUrl.searchParams.get("_action"), "plugin.orange_sso");
    assert.equal(postedUrl.searchParams.has("_ticket"), false);
    assert.deepEqual(first.fields, ["_ticket"]);
    assert.equal(first.ticket, "one-use-test-ticket-1");
    assert.equal(await page.locator('input[name="_ticket"]').count(), 0);
    const storage = await page.evaluate(() =>
      JSON.stringify({ ...localStorage, ...sessionStorage }),
    );
    assert.equal(storage.includes("one-use-test-ticket"), false);
    const cookie = (await context.cookies()).find(
      (cookie) => cookie.name === "orange_mail_admin",
    );
    assert.ok(
      cookie && cookie.httpOnly && cookie.secure && cookie.path === "/mailbox/",
    );
    await popup.getByLabel("草稿").fill("保留未发送的草稿");
    await page.locator(".admin-mail-entry").click();
    assert.equal(popups.length, 1);
    assert.equal(state.statusReads, 1);
    assert.equal(state.tickets, 1);
    assert.equal(state.posts.length, 1);
    assert.equal(
      await popup.getByLabel("草稿").inputValue(),
      "保留未发送的草稿",
    );

    await page.setViewportSize({ width: 390, height: 844 });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
    const mailBox = await page.locator(".admin-mail-entry").boundingBox();
    assert.ok(mailBox && mailBox.x >= 0 && mailBox.x + mailBox.width <= 390);
    await page.screenshot({
      path: path.join(output, "mail-window-header-mobile.png"),
    });
    await page.setViewportSize({ width: 1440, height: 1050 });
    await page.screenshot({
      path: path.join(output, "mail-window-header.png"),
    });
    await popup.close();

    await page.evaluate(() => (window.__blockMail = true));
    await page.locator(".admin-mail-entry").click();
    await page
      .locator(".form-error")
      .filter({ hasText: "邮件窗口被拦截，请允许弹出窗口" })
      .waitFor();
    assert.equal(state.statusReads, 1);
    assert.equal(state.tickets, 1);
    assert.equal(await page.locator("iframe").count(), 0);
    await page.evaluate(() => (window.__blockMail = false));

    state.enabled = false;
    await page.locator(".admin-mail-entry").click();
    await page
      .locator(".form-error")
      .filter({ hasText: state.reason })
      .waitFor();
    assert.equal(state.tickets, 1);
    assert.ok(
      popups.every((popup) => popup.isClosed()),
      "failed new window is closed",
    );

    state.enabled = true;
    state.entry = "https://other.invalid/mailbox/";
    await page.locator(".admin-mail-entry").click();
    await page
      .locator(".form-error")
      .filter({ hasText: "邮件入口配置错误" })
      .waitFor();
    assert.equal(state.tickets, 1);
    assert.ok(popups.every((popup) => popup.isClosed()));

    state.entry = "/mailbox/";
    state.deny = true;
    await page.locator(".admin-mail-entry").click();
    await page.locator(".admin-login-card").waitFor();
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("orange-admin-token")),
      null,
    );
    assert.equal(await page.locator(".admin-topbar").count(), 0);
    assert.equal(await page.locator("iframe").count(), 0);
    assert.ok(popups.every((popup) => popup.isClosed()));
    assert.deepEqual(state.unexpected, []);
    assert.equal(state.unauthenticatedRequests, 0);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        popupBeforeFetch: true,
        repeatFocusRetainsDraft: true,
        postOnlyTicket: true,
        ticketNotInUrlOrStorage: true,
        secureHttpOnlyCookie: true,
        blockedPopupNoFallback: true,
        failedWindowClosed: true,
        admin401Clears: true,
        oldDeepLinkOverview: true,
        sidebarMailRemoved: true,
        mobileOverflow: false,
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
