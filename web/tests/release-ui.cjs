const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const baseUrl = process.env.ORANGE_WEB_URL || "http://127.0.0.1:38180";

(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  try {
    const page = await browser.newPage({
      viewport: { width: 1440, height: 1000 },
      acceptDownloads: true,
    });
    const errors = [],
      rewardRequests = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("request", (request) => {
      if (/\/api\/rewards\//.test(request.url()))
        rewardRequests.push(request.url());
    });
    const config = {
      admin_auth_required: true,
      environment: "production",
      ad_mode: "csj",
      sms_mode: "mock",
      payment_mode: "mock",
      app_download_url:
        "https://orange.lingzidev.com/download/orange-rewards.apk",
    };
    await page.route("**/api/config", (route) =>
      route.fulfill({ json: config }),
    );
    let dataRequests = 0,
      denyToken = false;
    const data = {
      overview: {
        total_users: 7,
        coins_issued: 1500,
        rewards: 5,
        withdrawal_count: 0,
        ad_platforms: [{ name: "穿山甲", status: "已连接" }],
      },
      users: [],
      rewards: [],
      withdrawals: [],
      settings: {
        min_withdraw_yuan: 1,
        daily_reward_limit: 20,
        reward_amount: 100,
      },
      "hall-strategy": {
        enabled: true,
        interval_seconds: 20,
        video_weight: 70,
        product_weight: 30,
        native_ad_weight: 15,
        welcome_text: "欢迎",
        packet_text: "红包来了",
        product_text: "好物来了",
      },
      invitations: {
        platform_code: "AB8F6A",
        platform_url: "https://orange.lingzidev.com/i/AB8F6A",
        invited_count: 0,
        registered_count: 0,
        pending_count: 0,
        unbound_count: 0,
        records: [],
      },
      account: {
        username: "admin",
        auth_required: true,
        session_expires_at: "2026-11-12T00:00:00Z",
      },
    };
    await page.route("**/api/admin/**", async (route) => {
      const request = route.request(),
        name = request.url().split("/").pop();
      if (name === "login") {
        if (
          request.postDataJSON().username === "admin" &&
          request.postDataJSON().password === "AdminUi456!"
        )
          return route.fulfill({
            json: {
              token: "admin-test-token",
              username: "admin",
              expires_at: "2026-11-12T00:00:00Z",
            },
          });
        return route.fulfill({ status: 401, json: { error: "管理密码错误" } });
      }
      dataRequests++;
      if (
        denyToken ||
        request.headers()["x-admin-token"] !== "admin-test-token"
      )
        return route.fulfill({
          status: 401,
          json: { error: "管理登录已过期" },
        });
      return route.fulfill({ json: data[name] });
    });
    await page.addInitScript(() => localStorage.setItem("welcome_seen", "1"));
    await page.goto(baseUrl + "/admin");
    await page.locator(".admin-login-card").waitFor();
    assert.equal(dataRequests, 0, "no private admin requests before login");
    assert.equal(await page.locator(".stat-card").count(), 0);
    await page.getByLabel("管理密码", { exact: true }).fill("wrong-password");
    await page
      .getByRole("button", { name: "登录管理中心", exact: true })
      .click();
    await page.getByText("管理密码错误", { exact: true }).waitFor();
    assert.equal(dataRequests, 0);
    await page.getByLabel("管理密码", { exact: true }).fill("AdminUi456!");
    await page
      .getByRole("button", { name: "登录管理中心", exact: true })
      .click();
    await page.locator(".stat-card").first().waitFor();
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("orange-admin-token")),
      "admin-test-token",
    );
    assert.equal(
      await page.evaluate(() => localStorage.getItem("orange-admin-token")),
      null,
    );
    denyToken = true;
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page.locator(".admin-login-card").waitFor();
    assert.equal(
      await page.locator(".stat-card").count(),
      0,
      "401 must hide prior stats",
    );
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("orange-admin-token")),
      null,
    );
    await page.goto(baseUrl);
    await page.locator(".app-name small").waitFor();
    await page.locator(".tab-bar button").filter({ hasText: "我的" }).click();
    await page.getByRole("button", { name: "登录", exact: true }).click();
    assert.equal(
      await page.getByLabel("手机号", { exact: true }).inputValue(),
      "",
    );
    assert.equal(
      await page.getByLabel("密码", { exact: true }).inputValue(),
      "",
    );
    assert.equal(await page.locator(".demo-account").count(), 0);
    await page.getByRole("button", { name: "关闭", exact: true }).click();
    await page.locator(".tab-bar button").filter({ hasText: "大厅" }).click();
    await page.getByRole("button", { name: "去领红包", exact: true }).click();
    await page
      .getByRole("button", { name: "打开App领红包", exact: true })
      .waitFor();
    await page.route(config.app_download_url, (route) =>
      route.fulfill({
        status: 200,
        headers: {
          "Content-Type": "application/vnd.android.package-archive",
          "Content-Disposition": 'attachment; filename="orange-rewards.apk"',
        },
        body: "test apk download",
      }),
    );
    const downloading = page.waitForEvent("download");
    await page
      .getByRole("button", { name: "打开App领红包", exact: true })
      .click();
    await downloading;
    assert.deepEqual(
      rewardRequests,
      [],
      "CSJ browser must never start or complete mock rewards",
    );
    assert.equal(await page.locator(".stage-video,.stage-won").count(), 0);
    await page.goto(baseUrl + "/privacy");
    await page
      .getByRole("heading", { name: "隐私政策", exact: true })
      .waitFor();
    await page
      .getByRole("link", { name: "穿山甲广告 SDK 隐私政策", exact: true })
      .waitFor();
    await page.goto(baseUrl + "/terms");
    await page
      .getByRole("heading", { name: "用户协议", exact: true })
      .waitFor();
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        adminNoDataBeforeLogin: true,
        adminWrongPassword: true,
        adminSessionStorage: true,
        admin401ClearsStats: true,
        productionDemoHidden: true,
        csjDownloadOnly: true,
        mockRewardRequests: 0,
        privacyAndTerms: true,
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
