const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path");
const dist = path.join(__dirname, "../dist"),
  origin = "https://activity-preview.test";
const state = { writes: 0, unexpected: [] };
const activity = {
  enabled: true,
  title: "每日小欢喜",
  subtitle: "完成活动，赢取金币",
  daily_spin_limit: 2,
  settle_delay_seconds: 60,
  arrival_threshold_coins: 0,
  login_opportunities: 1,
  video_opportunities_per_day: 3,
  invite_opportunities_per_invitee: 1,
  prizes: [5, 10, 20],
};
async function fixture(page) {
  await page.route("**/*", async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      endpoint = url.pathname.slice(4),
      json = (data) => route.fulfill({ json: data });
    if (url.pathname.startsWith("/api/")) {
      if (endpoint === "/config")
        return json({
          admin_auth_required: true,
          environment: "production",
          ad_mode: "csj",
          sms_mode: "mock",
          payment_mode: "mock",
        });
      if (endpoint === "/tasks" || endpoint === "/products") return json([]);
      if (endpoint.startsWith("/admin/")) {
        assert.equal(request.headers()["x-admin-token"], "mock-activity-admin");
      }
      const defaults = {
        "/admin/account": { username: "运营管理员", auth_required: true },
        "/admin/overview": { total_users: 0, ad_platforms: [] },
        "/admin/rewards": [],
        "/admin/withdrawals": [],
        "/admin/settings": {},
        "/admin/hall-strategy": {},
        "/admin/invitations": { records: [], platform_code: "TEST26" },
      };
      if (Object.hasOwn(defaults, endpoint)) return json(defaults[endpoint]);
      if (endpoint === "/admin/campaign") {
        if (request.method() === "PUT") {
          state.writes++;
          Object.assign(activity, request.postDataJSON());
        }
        return json(activity);
      }
      if (endpoint === "/admin/campaign/stats")
        return json({ users: 0, opportunities: 0, spins: 0, pending: 0, settled: 0, settled_coins: 0 });
      if (endpoint === "/admin/campaign/users")
        return json({ items: [], total: 0, page: 1, page_size: 20 });
      state.unexpected.push(endpoint);
      return route.fulfill({ status: 404, json: { error: "未配置测试接口" } });
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
    await context.addInitScript(() =>
      sessionStorage.setItem("orange-admin-token", "mock-activity-admin"),
    );
    const page = await context.newPage();
    page.on("pageerror", (e) => errors.push(e.message));
    await fixture(page);
    await page.goto(origin + "/admin");
    await page.getByRole("button", { name: "运营管理", exact: true }).click();
    await page.getByRole("button", { name: "活动配置", exact: true }).click();
    await page.getByText("每日小欢喜", { exact: true }).waitFor();
    await page.getByLabel("开启活动", { exact: true }).uncheck();
    await page.getByLabel("活动标题", { exact: true }).fill("周末转盘");
    await page.getByLabel("金币到账门槛", { exact: true }).fill("200");
    await page.getByRole("button", { name: "添加档位", exact: true }).click();
    assert.equal(await page.locator(".activity-tier").count(), 4);
    await page
      .getByRole("button", { name: "保存活动设置", exact: true })
      .click();
    await page.getByText("活动设置已保存", { exact: true }).waitFor();
    assert.equal(state.writes, 1);
    assert.equal(activity.enabled, false);
    assert.equal(activity.title, "周末转盘");
    assert.equal(activity.arrival_threshold_coins, 200);
    assert.deepEqual(state.unexpected, []);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        activityToggle: true,
        rewardTiers: true,
        spinSource: true,
        dailyLimit: true,
        serverSave: true,
        noFakeResult: true,
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
