const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path");
const dist = path.join(__dirname, "../dist"),
  output = path.join(__dirname, "../../output");
const origin = "https://user-center-preview.test";
const state = {
  queries: [],
  empty: false,
  error: "",
  deny: false,
  unexpected: [],
};
const today = new Date(Date.now() + 8 * 3600000).toISOString().slice(0, 10);
function responseFor(query) {
  const end = query.get("end_date") || today,
    count = Number(query.get("days") || 7);
  const start =
    query.get("start_date") ||
    new Date(Date.parse(end) - (count - 1) * 86400000)
      .toISOString()
      .slice(0, 10);
  const days = Math.round((Date.parse(end) - Date.parse(start)) / 86400000) + 1;
  const daily = Array.from({ length: days }, (_, index) => ({
    date: new Date(Date.parse(start) + index * 86400000)
      .toISOString()
      .slice(0, 10),
    registrations: state.empty ? 0 : index % 6,
    active_users: state.empty ? 0 : (index % 8) + 2,
    total_users: state.empty ? 0 : 200 + index * 3,
    returning_active: state.empty ? 0 : index % 4,
  }));
  return {
    selected_range: { start_date: start, end_date: end, days },
    summary: state.empty
      ? {
          total_users: 0,
          today_registrations: 0,
          yesterday_registrations: 0,
          range_registrations: 0,
          today_active_users: 0,
          active_users_7d: 0,
          active_users_30d: 0,
          range_active_users: 0,
          range_inactive_users: 0,
          range_total_users: 0,
        }
      : {
          total_users: 387,
          today_registrations: 6,
          yesterday_registrations: 5,
          range_registrations: 48,
          today_active_users: 12,
          active_users_7d: 29,
          active_users_30d: 91,
          range_active_users: 29,
          range_inactive_users: 358,
          range_total_users: 387,
        },
    daily,
    activity_days: [
      { key: "1", label: "1天", users: state.empty ? 0 : 11 },
      { key: "2_3", label: "2–3天", users: state.empty ? 0 : 9 },
      { key: "4_7", label: "4–7天", users: state.empty ? 0 : 7 },
      { key: "8_plus", label: "8天及以上", users: state.empty ? 0 : 2 },
    ],
    activity_definition:
      "按北京时间自然日，对成功登录、打开应用或参与业务的用户去重统计。",
    metadata: {
      timezone: "UTC+08:00",
      generated_at: new Date().toISOString(),
      behavior_collection_since: null,
      returning_active_definition: "注册日期早于该日的活跃用户。",
      inactive_definition: "截止所选结束日已注册，但区间内没有活跃的用户。",
      history_note: "活跃仅依据已采集的成功行为；采集前的历史不补算。",
    },
  };
}
async function fixture(context) {
  await context.route("**/*", async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      endpoint = url.pathname.slice(4);
    const json = (data) => route.fulfill({ json: data });
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
      if (endpoint === "/products/runtime") return json({ hero_image: null, revision: 1 });
      if (endpoint.startsWith("/admin/")) {
        assert.equal(request.headers()["x-admin-token"], "mock-user-stats");
        if (state.deny)
          return route.fulfill({
            status: 401,
            json: { error: "管理登录已过期" },
          });
      }
      const defaults = {
        "/admin/account": { username: "运营管理员", auth_required: true },
        "/admin/overview": { total_users: 387, ad_platforms: [] },
        "/admin/rewards": [],
        "/admin/withdrawals": [],
        "/admin/settings": {},
        "/admin/hall-strategy": {},
        "/admin/invitations": { records: [], platform_code: "TEST26" },
        "/admin/users/search": { items: [], total: 0, page: 1, page_size: 20 },
      };
      if (Object.hasOwn(defaults, endpoint)) return json(defaults[endpoint]);
      if (endpoint === "/admin/user-center/stats") {
        state.queries.push(Object.fromEntries(url.searchParams));
        if (state.error)
          return route.fulfill({ status: 400, json: { error: state.error } });
        return json(responseFor(url.searchParams));
      }
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
      viewport: { width: 1440, height: 1100 },
    });
    await context.addInitScript(() =>
      sessionStorage.setItem("orange-admin-token", "mock-user-stats"),
    );
    await fixture(context);
    const page = await context.newPage();
    page.on("pageerror", (failure) => errors.push(failure.message));
    await page.goto(origin + "/admin");
    const nav = page.getByRole("navigation", { name: "后台导航" });
    await nav.getByRole("button", { name: "用户概览", exact: true }).click();
    await page.locator(".user-center-metrics").waitFor();
    assert.equal(await page.locator(".user-center-metrics article").count(), 6);
    assert.equal(
      await page
        .locator(".user-center-metrics article")
        .filter({ hasText: "累计用户" })
        .locator(":scope > b")
        .innerText(),
      "387人",
    );
    assert.equal(
      await page.getByRole("img", { name: "每日注册", exact: true }).count(),
      1,
    );
    assert.equal(
      await page.getByRole("img", { name: "每日活跃", exact: true }).count(),
      1,
    );
    assert.deepEqual(state.queries[0], { days: "30" });
    await nav.getByRole("button", { name: "运营管理", exact: true }).click();
    await nav.getByRole("button", { name: "资金管理", exact: true }).click();
    assert.equal(
      await nav
        .getByRole("button", { name: "用户中心", exact: true })
        .getAttribute("aria-expanded"),
      "true",
    );
    assert.equal(
      await nav
        .getByRole("button", { name: "运营管理", exact: true })
        .getAttribute("aria-expanded"),
      "true",
    );
    assert.equal(
      await nav
        .getByRole("button", { name: "资金管理", exact: true })
        .getAttribute("aria-expanded"),
      "true",
    );
    assert.equal(await nav.getByText("邮件", { exact: true }).count(), 0);
    await page.screenshot({
      path: path.join(output, "user-center-overview.png"),
    });
    await nav.getByRole("button", { name: "注册分析", exact: true }).click();
    await page
      .getByRole("img", { name: "累计用户趋势", exact: true })
      .waitFor();
    await page.getByRole("button", { name: "近7天", exact: true }).click();
    await page.waitForFunction(
      () => document.querySelectorAll(".chart-bar").length === 7,
    );
    assert.deepEqual(state.queries.at(-1), { days: "7" });
    await page.getByRole("button", { name: "近90天", exact: true }).click();
    await page.waitForFunction(
      () => document.querySelectorAll(".chart-bar").length === 90,
    );
    assert.deepEqual(state.queries.at(-1), { days: "90" });
    const customStart = new Date(Date.parse(today) - 6 * 86400000)
      .toISOString()
      .slice(0, 10);
    await page.getByLabel("统计开始日期", { exact: true }).fill(customStart);
    await page.getByLabel("统计结束日期", { exact: true }).fill(today);
    await page.getByRole("button", { name: "查询", exact: true }).click();
    await page.waitForFunction(
      () => document.querySelectorAll(".chart-bar").length === 7,
    );
    assert.deepEqual(state.queries.at(-1), {
      start_date: customStart,
      end_date: today,
    });
    await page.screenshot({
      path: path.join(output, "user-center-registrations.png"),
    });
    await nav.getByRole("button", { name: "活跃分析", exact: true }).click();
    await page.getByText("活跃天数分布", { exact: true }).waitFor();
    assert.equal(await page.locator(".distribution-row").count(), 4);
    assert.equal(
      await page
        .locator(".user-center-metrics article")
        .filter({ hasText: "近30日活跃" })
        .locator(":scope > b")
        .innerText(),
      "91人",
    );
    await page.screenshot({
      path: path.join(output, "user-center-activity.png"),
    });
    await page.setViewportSize({ width: 390, height: 844 });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
    await page.getByRole("button", { name: "打开导航", exact: true }).click();
    await page.screenshot({
      path: path.join(output, "admin-group-menu-mobile.png"),
      animations: "disabled",
    });
    await nav.getByRole("button", { name: "用户概览", exact: true }).click();
    assert.equal(await page.locator(".admin-nav-backdrop").count(), 0);
    await page.screenshot({
      path: path.join(output, "user-center-mobile.png"),
      animations: "disabled",
    });
    state.empty = true;
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page.getByText("所选时段暂无新增注册", { exact: true }).waitFor();
    await page.getByText("所选时段暂无活跃记录", { exact: true }).waitFor();
    state.error = "日期区间应为1至366天，结束日期不得早于开始日期";
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page.getByRole("alert").filter({ hasText: state.error }).waitFor();
    assert.equal(await page.locator(".user-center-metrics").count(), 0);
    state.error = "";
    state.empty = false;
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page
      .getByRole("button", { name: "查看用户列表", exact: true })
      .click();
    await page.locator(".user-list-search").waitFor();
    await page.getByRole("button", { name: "打开导航", exact: true }).click();
    await nav.getByRole("button", { name: "用户概览", exact: true }).click();
    await page.locator(".user-center-metrics").waitFor();
    state.deny = true;
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page.locator(".admin-login-card").waitFor();
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("orange-admin-token")),
      null,
    );
    assert.deepEqual(state.unexpected, []);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        multipleMenuGroups: true,
        threeUserViews: true,
        actualApiFields: true,
        presetAndCustomDates: true,
        chartsAndDistribution: true,
        emptyAndOriginalErrors: true,
        usersListNavigation: true,
        admin401Clears: true,
        mobile390: true,
        runtimeErrors: errors,
      }),
    );
  } finally {
    await browser.close();
  }
})().catch((failure) => {
  console.error(failure);
  process.exitCode = 1;
});
