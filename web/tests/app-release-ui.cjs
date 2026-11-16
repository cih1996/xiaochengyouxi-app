const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const dist = path.join(__dirname, "../dist");
const output = path.join(__dirname, "../../output");
const origin = "http://release-preview.test";
const fields = [
  "enabled",
  "latest_version_code",
  "latest_version_name",
  "min_supported_version_code",
  "title",
  "release_notes",
  "apk_url",
  "apk_sha256",
  "apk_size_bytes",
];
const state = {
  release: {
    enabled: false,
    latest_version_code: 0,
    latest_version_name: "",
    min_supported_version_code: 0,
    title: "发现新版本",
    release_notes: "",
    apk_url: "",
    apk_sha256: "",
    apk_size_bytes: 0,
    revision: 1,
    update_available: true,
    force_update: true,
  },
  runtime: { revision: 1, hall_notice: "", updated_at: null },
  releases: [],
  notices: [],
  runtimeReads: 0,
  unexpected: [],
  deny: false,
};
async function fixture(page) {
  await page.route("**/*", async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      endpoint = url.pathname.slice(4);
    if (url.pathname.startsWith("/api")) {
      const json = (data) => route.fulfill({ json: data });
      if (endpoint === "/config")
        return json({
          admin_auth_required: true,
          environment: "production",
          ad_mode: "csj",
          sms_mode: "mock",
          payment_mode: "mock",
          device_trust_enabled: true,
        });
      if (endpoint === "/tasks" || endpoint === "/products") return json([]);
      if (endpoint === "/app/runtime") {
        state.runtimeReads++;
        return json(state.runtime);
      }
      if (endpoint.startsWith("/admin/") && state.deny)
        return route.fulfill({
          status: 401,
          json: { error: "管理登录已过期" },
        });
      const data = {
        "/admin/overview": {
          total_users: 0,
          coins_issued: 0,
          rewards: 0,
          withdrawal_count: 0,
          ad_platforms: [],
        },
        "/admin/rewards": [],
        "/admin/withdrawals": [],
        "/admin/settings": {
          daily_reward_limit: 20,
          min_withdraw_yuan: 1,
          csj_expected_reward_name: "金币",
          csj_expected_reward_amount: 100,
        },
        "/admin/hall-strategy": {},
        "/admin/invitations": { records: [], platform_code: "TEST26" },
        "/admin/account": {
          username: "发布管理员",
          auth_required: true,
          session_expires_at: "2026-11-12T00:00:00Z",
        },
      };
      if (Object.hasOwn(data, endpoint)) return json(data[endpoint]);
      if (endpoint === "/admin/app-release") {
        if (request.method() === "GET") return json(state.release);
        const body = request.postDataJSON();
        state.releases.push(body);
        if (
          body.enabled &&
          (!body.apk_url || !body.apk_sha256 || !body.apk_size_bytes)
        )
          return route.fulfill({
            status: 400,
            json: { error: "启用更新检测前，请填写完整安装包信息" },
          });
        state.release = {
          ...body,
          revision: state.release.revision + 1,
          update_available: true,
          force_update: false,
        };
        return json(state.release);
      }
      if (endpoint === "/admin/app-runtime") {
        if (request.method() === "GET") return json(state.runtime);
        const body = request.postDataJSON();
        state.notices.push(body);
        state.runtime = {
          ...body,
          revision: state.runtime.revision + 1,
          updated_at: "2026-11-11T08:00:00Z",
        };
        return json(state.runtime);
      }
      state.unexpected.push(endpoint);
      return route.fulfill({
        status: 404,
        json: { error: "未提供接口：" + endpoint },
      });
    }
    const file = url.pathname.startsWith("/assets/")
      ? path.join(dist, decodeURIComponent(url.pathname.slice(1)))
      : path.join(dist, "index.html");
    const contentType = {
      ".html": "text/html",
      ".js": "text/javascript",
      ".css": "text/css",
      ".png": "image/png",
      ".wav": "audio/wav",
    }[path.extname(file)];
    return route.fulfill({ contentType, body: fs.readFileSync(file) });
  });
}
async function nextResponse(page, endpoint, action) {
  const pending = page.waitForResponse((response) =>
    response.url().endsWith(endpoint),
  );
  await action();
  return pending;
}
(async () => {
  fs.mkdirSync(output, { recursive: true });
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  const errors = [];
  try {
    const page = await browser.newPage({
      viewport: { width: 1512, height: 1100 },
    });
    page.on("pageerror", (error) => errors.push(error.message));
    await fixture(page);
    await page.addInitScript(() =>
      sessionStorage.setItem("orange-admin-token", "mock-admin-session"),
    );
    await page.goto(origin + "/admin");
    await page.getByRole("button", { name: "版本发布", exact: true }).click();
    await page.getByLabel("开启版本检测", { exact: true }).waitFor();
    assert.equal(
      await page.getByLabel("开启版本检测", { exact: true }).isChecked(),
      false,
    );
    assert.equal(
      await page.locator(".release-package").getAttribute("open"),
      null,
    );
    await nextResponse(page, "/api/admin/app-release", () =>
      page.getByRole("button", { name: "保存发布设置", exact: true }).click(),
    );
    assert.equal(state.releases.at(-1).enabled, false);
    assert.deepEqual(
      Object.keys(state.releases.at(-1)).sort(),
      [...fields].sort(),
    );

    const imported = {
      enabled: true,
      latest_version_code: 9,
      latest_version_name: "0.3.6",
      min_supported_version_code: 0,
      title: "小欢喜，又升级了",
      release_notes: "新增版本更新提醒\n大厅公告及时送达\n体验细节持续优化",
      apk_url: "https://orange.lingzidev.com/download/orange-rewards.apk",
      apk_sha256: "f".repeat(64),
      apk_size_bytes: 24122432,
      revision: 999,
      force_update: true,
      update_available: true,
    };
    const beforeImport = state.releases.length;
    await page.getByLabel("选择发布信息文件", { exact: true }).setInputFiles({
      name: "更新发布.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify(imported)),
    });
    await page.getByText("已导入，保存后生效", { exact: true }).waitFor();
    assert.equal(
      state.releases.length,
      beforeImport,
      "import is a draft, not publication",
    );
    assert.equal(
      await page.getByLabel("最新版本名称", { exact: true }).inputValue(),
      "0.3.6",
    );
    assert.equal(
      await page.getByLabel("SHA-256", { exact: true }).inputValue(),
      imported.apk_sha256,
    );
    assert.equal(
      await page
        .getByLabel("最低支持版本（代码）", { exact: true })
        .inputValue(),
      "0",
    );
    await nextResponse(page, "/api/admin/app-release", () =>
      page.getByRole("button", { name: "保存发布设置", exact: true }).click(),
    );
    assert.deepEqual(
      state.releases.at(-1),
      Object.fromEntries(fields.map((key) => [key, imported[key]])),
    );
    assert.equal(
      state.release.revision,
      3,
      "imported revision cannot replace server revision",
    );
    await page.getByText("发布设置已保存", { exact: true }).waitFor();
    await page
      .locator(".release-switch > span")
      .evaluate((element) =>
        Promise.all(
          element
            .getAnimations({ subtree: true })
            .map((animation) => animation.finished),
        ),
      );
    await page.screenshot({
      path: path.join(output, "app-release-panel.png"),
      fullPage: true,
    });

    await page.getByLabel("最低支持版本（代码）", { exact: true }).fill("8");
    await nextResponse(page, "/api/admin/app-release", () =>
      page.getByRole("button", { name: "保存发布设置", exact: true }).click(),
    );
    assert.equal(state.releases.at(-1).latest_version_code, 9);
    assert.equal(state.releases.at(-1).min_supported_version_code, 8);
    await page
      .getByLabel("大厅公告", { exact: true })
      .fill("周末福利已送达，来大厅收下小欢喜。");
    assert.equal(
      await page
        .getByLabel("大厅公告", { exact: true })
        .getAttribute("maxlength"),
      "200",
    );
    await nextResponse(page, "/api/admin/app-runtime", () =>
      page.getByRole("button", { name: "保存公告", exact: true }).click(),
    );
    assert.deepEqual(state.notices.at(-1), {
      hall_notice: "周末福利已送达，来大厅收下小欢喜。",
    });
    await page.getByLabel("大厅公告", { exact: true }).fill("");
    await nextResponse(page, "/api/admin/app-runtime", () =>
      page.getByRole("button", { name: "保存公告", exact: true }).click(),
    );
    assert.deepEqual(state.notices.at(-1), { hall_notice: "" });

    const invalid = '{"enabled":';
    const originalError = await page.evaluate((text) => {
      try {
        JSON.parse(text);
      } catch (error) {
        return error.message;
      }
    }, invalid);
    await page.getByLabel("选择发布信息文件", { exact: true }).setInputFiles({
      name: "错误.json",
      mimeType: "application/json",
      buffer: Buffer.from(invalid),
    });
    await page.getByRole("alert").waitFor();
    assert.equal(await page.getByRole("alert").innerText(), originalError);
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page.getByLabel("最新版本名称", { exact: true }).waitFor();
    await page.locator(".release-package summary").click();
    await page.getByLabel("下载地址", { exact: true }).fill("");
    await nextResponse(page, "/api/admin/app-release", () =>
      page.getByRole("button", { name: "保存发布设置", exact: true }).click(),
    );
    await page
      .getByText("启用更新检测前，请填写完整安装包信息", { exact: true })
      .waitFor();
    assert.equal(
      await page.getByLabel("最新版本名称", { exact: true }).inputValue(),
      "0.3.6",
    );
    await page.getByLabel("下载地址", { exact: true }).fill(imported.apk_url);
    await nextResponse(page, "/api/admin/app-release", () =>
      page.getByRole("button", { name: "保存发布设置", exact: true }).click(),
    );
    await page.getByText("发布设置已保存", { exact: true }).waitFor();
    await page.setViewportSize({ width: 390, height: 844 });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({
      path: path.join(output, "app-release-mobile.png"),
      fullPage: true,
    });
    state.deny = true;
    await nextResponse(page, "/api/admin/app-release", () =>
      page.getByRole("button", { name: "刷新数据", exact: true }).click(),
    );
    await page.locator(".admin-login-card").waitFor();
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("orange-admin-token")),
      null,
    );
    assert.equal(await page.locator(".app-release-page").count(), 0);
    state.deny = false;

    const hall = await browser.newPage({
      viewport: { width: 390, height: 844 },
    });
    hall.on("pageerror", (error) => errors.push(error.message));
    await fixture(hall);
    await hall.clock.install();
    await hall.addInitScript(() => {
      localStorage.setItem("welcome_seen", "1");
      window.__visible = true;
      Object.defineProperty(document, "visibilityState", {
        get: () => (window.__visible ? "visible" : "hidden"),
      });
    });
    state.runtime.hall_notice = "周末福利已送达，来大厅收下小欢喜。";
    await hall.goto(origin);
    await hall.locator(".hall-notice").waitFor();
    await hall.locator(".hero-mascot").evaluate((image) => image.decode());
    await hall.screenshot({ path: path.join(output, "hall-notice.png") });
    const reads = state.runtimeReads;
    state.runtime.hall_notice = "公告已经更新，祝你今天收获小欢喜。";
    await nextResponse(hall, "/api/app/runtime", () =>
      hall.clock.fastForward(45000),
    );
    await hall.getByText(state.runtime.hall_notice, { exact: true }).waitFor();
    assert.equal(state.runtimeReads, reads + 1);
    await hall.evaluate(() => {
      window.__visible = false;
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await hall.clock.fastForward(90000);
    assert.equal(
      state.runtimeReads,
      reads + 1,
      "hidden pages make no announcement requests",
    );
    await nextResponse(hall, "/api/app/runtime", () =>
      hall.evaluate(() => {
        window.__visible = true;
        document.dispatchEvent(new Event("visibilitychange"));
      }),
    );
    assert.equal(state.runtimeReads, reads + 2);
    await hall.locator(".tab-bar button").filter({ hasText: "捡漏" }).click();
    await hall.clock.fastForward(90000);
    await hall.evaluate(() => window.dispatchEvent(new Event("focus")));
    assert.equal(
      state.runtimeReads,
      reads + 2,
      "leaving hall cleans the interval and listeners",
    );
    await nextResponse(hall, "/api/app/runtime", () =>
      hall.locator(".tab-bar button").filter({ hasText: "大厅" }).click(),
    );
    assert.equal(state.runtimeReads, reads + 3);
    await nextResponse(hall, "/api/app/runtime", () =>
      hall.clock.fastForward(45000),
    );
    assert.equal(
      state.runtimeReads,
      reads + 4,
      "returning to hall creates only one interval",
    );
    state.runtime.hall_notice = "";
    await nextResponse(hall, "/api/app/runtime", () =>
      hall.evaluate(() => window.dispatchEvent(new Event("focus"))),
    );
    await hall.locator(".hall-notice").waitFor({ state: "hidden" });
    assert.deepEqual(state.unexpected, []);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        draftImport: true,
        whitelistBody: true,
        optionalAndMinimumVersions: true,
        noticeSaveAndClear: true,
        originalErrors: true,
        admin401Clears: true,
        announcementVisiblePolling: true,
        hiddenNoRequests: true,
        cleanupOnTabLeave: true,
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
