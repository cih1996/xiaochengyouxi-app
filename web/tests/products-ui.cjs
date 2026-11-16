const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs"),
  path = require("node:path");
const dist = path.join(__dirname, "../dist"),
  output = path.join(__dirname, "../../output");
const origin = "https://products-preview.test";
const now = Math.floor(Date.now() / 1000) * 1000;
const initialProduct = {
  id: 10,
  title: "轻巧随行保温杯",
  url: "https://shop-preview.test/item/10",
  image: "/assets/product-2.png",
  price: 49.9,
  price_text: "49.90",
  price_fen: 4990,
  commission_percent: 20,
  reward_coins: 40000,
  platform: "淘宝",
  category: "家居",
  external_id: "cup-10",
  starts_at: new Date(now - 86400000).toISOString(),
  expires_at: new Date(now + 6 * 86400000).toISOString(),
  enabled: true,
  status: "active",
  revision: 1,
};
const state = {
  products: [{ ...initialProduct }],
  config: { platform_share_percent: 20, default_valid_days: 11, revision: 1 },
  key: null,
  keyWrites: 0,
  changes: [],
  queries: [],
  configs: [],
  denies: false,
  saveError: "",
  expired: false,
  publicEmpty: false,
  detailReads: 0,
  impressions: 0,
  clickKinds: [],
  unexpected: [],
};
const imageBytes = fs.readFileSync(path.join(dist, "assets/product-1.png"));
function wireProduct(product) {
  const timestamp = Date.now();
  return {
    ...product,
    server_now: new Date(timestamp).toISOString(),
    seconds_until_start: Math.max(
      0,
      Math.ceil((Date.parse(product.starts_at) - timestamp) / 1000),
    ),
    seconds_until_expiry: Math.max(
      0,
      Math.ceil((Date.parse(product.expires_at) - timestamp) / 1000),
    ),
  };
}
async function fixture(context) {
  await context.route("**/*", async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      endpoint = url.pathname.slice(4);
    const json = (data) => route.fulfill({ json: data });
    if (url.origin === "https://shop-preview.test")
      return route.fulfill({
        contentType: "text/html; charset=utf-8",
        body: "<p>目标购物平台</p>",
      });
    if (url.pathname.startsWith("/api/")) {
      if (endpoint === "/config")
        return json({
          admin_auth_required: true,
          environment: "production",
          ad_mode: "csj",
          device_trust_enabled: true,
          sms_mode: "mock",
          payment_mode: "mock",
        });
      if (endpoint === "/tasks") return json([]);
      if (endpoint === "/products")
        return json(state.publicEmpty ? [] : [initialProduct]);
      if (endpoint === "/products/runtime")
        return json({ hero_image: null, revision: 1 });
      if (/^\/products\/10\/(click|impression)$/.test(endpoint)) {
        if (endpoint.endsWith("/impression")) state.impressions++;
        else state.clickKinds.push(request.postDataJSON().kind);
        return json({ ok: true });
      }
      if (endpoint === "/products/10") {
        state.detailReads++;
        return state.expired
          ? route.fulfill({
              status: 404,
              json: { error: "商品已下架或已过期" },
            })
          : json({ ...initialProduct, reward_coins: 45000 });
      }
      if (endpoint === "/me")
        return json({
          id: 1,
          phone: "13800138000",
          nickname: "测试用户",
          coins: 0,
          total_earned: 0,
          today_earned: 0,
          invitation_required: false,
          checked_in_today: false,
        });
      if (endpoint === "/app/runtime")
        return json({ revision: 1, hall_notice: "" });
      if (endpoint === "/rain/status")
        return json({
          enabled: false,
          phase: "disabled",
          server_now: new Date().toISOString(),
          next_start_at: null,
          round: null,
          config: { enabled: false },
        });
      if (endpoint === "/hall/stream")
        return route.fulfill({
          contentType: "text/event-stream",
          body:
            "data: " +
            JSON.stringify({
              id: "product-push",
              kind: "product",
              text: "今天发现一件好物",
              product: initialProduct,
              created_at: new Date().toISOString(),
            }) +
            "\n\n",
        });
      if (endpoint.startsWith("/admin/")) {
        assert.equal(request.headers()["x-admin-token"], "mock-products-admin");
        if (state.denies)
          return route.fulfill({
            status: 401,
            json: { error: "管理登录已过期" },
          });
      }
      const defaults = {
        "/admin/account": { username: "商品管理员", auth_required: true },
        "/admin/overview": { total_users: 0, ad_platforms: [] },
        "/admin/rewards": [],
        "/admin/withdrawals": [],
        "/admin/settings": {},
        "/admin/hall-strategy": {},
        "/admin/invitations": { records: [], platform_code: "TEST26" },
      };
      if (Object.hasOwn(defaults, endpoint)) return json(defaults[endpoint]);
      if (endpoint === "/admin/products/config") {
        if (request.method() === "PUT") {
          state.configs.push(request.postDataJSON());
          state.config = {
            ...request.postDataJSON(),
            revision: state.config.revision + 1,
          };
        }
        return json(state.config);
      }
      if (endpoint === "/admin/products/display-config") {
        if (request.method() === "PUT")
          state.configs.push(request.postDataJSON());
        return json({ hero_image: null, revision: 1 });
      }
      if (endpoint === "/admin/products/analytics")
        return json({
          days: 7,
          definition: "跳转次数 ÷ 详情打开次数",
          summary: {
            clicks: 0,
            unique_users: 0,
            detail_opens: 0,
            outbound_clicks: 0,
          },
          summary_ctr: null,
          products: [],
          impressions: { count: 0, unique_users: 0 },
          daily: [],
        });
      if (endpoint === "/admin/products/images")
        return json({
          image_id: "hero-image-1",
          image: "/media/hero-image-1.png",
          content_type: "image/png",
          size_bytes: 10,
        });
      if (endpoint === "/admin/products/api-key") {
        if (request.method() === "POST") {
          state.keyWrites++;
          state.key = "mock-product-key-" + state.keyWrites;
        }
        return json({
          enabled: Boolean(state.key),
          key: state.key,
          header_name: "X-Product-Api-Key",
          endpoint: "/api/integration/products",
        });
      }
      if (endpoint === "/admin/products" && request.method() === "GET") {
        state.queries.push(Object.fromEntries(url.searchParams));
        let items = state.products.filter(
          (product) =>
            (url.searchParams.get("status") === "all" ||
              product.status === url.searchParams.get("status")) &&
            product.title.includes(url.searchParams.get("q")),
        );
        return json({
          items: items.map(wireProduct),
          total: items.length,
          page: Number(url.searchParams.get("page")),
          page_size: 20,
        });
      }
      if (
        endpoint === "/admin/products" ||
        /^\/admin\/products\/\d+$/.test(endpoint)
      ) {
        assert.match(
          request.headers()["content-type"],
          /^multipart\/form-data; boundary=/,
        );
        const data = await new Request(request.url(), {
          method: "POST",
          headers: request.headers(),
          body: request.postDataBuffer(),
        }).formData();
        const fields = Object.fromEntries(
          [...data].filter(([key]) => key !== "image"),
        );
        state.changes.push({
          endpoint,
          fields,
          image: Boolean(data.get("image")),
        });
        if (data.get("image"))
          assert.deepEqual(
            Buffer.from(await data.get("image").arrayBuffer()),
            imageBytes,
          );
        if (state.saveError)
          return route.fulfill({
            status: 400,
            json: { error: state.saveError },
          });
        const existing = state.products.find((product) =>
          endpoint.endsWith("/" + product.id),
        );
        const product = {
          ...(existing || initialProduct),
          ...fields,
          id: existing?.id || 11,
          image: "/assets/product-1.png",
          price: Number(fields.price || existing.price),
          price_text: fields.price || existing.price_text,
          commission_percent: Number(
            fields.commission_percent || existing.commission_percent,
          ),
          enabled: (fields.enabled || String(existing?.enabled)) === "true",
          reward_coins: 23456,
        };
        product.status = product.enabled ? "active" : "disabled";
        state.products = [
          product,
          ...state.products.filter((item) => item.id !== product.id),
        ];
        return json(product);
      }
      state.unexpected.push(endpoint);
      return route.fulfill({
        status: 404,
        json: { error: "未配置的测试接口" },
      });
    }
    if (url.pathname === "/docs/PRODUCT-API.md")
      return route.fulfill({
        contentType: "text/markdown; charset=utf-8",
        body: fs.readFileSync(path.join(dist, "docs/PRODUCT-API.md")),
      });
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
    await context.addInitScript(() => {
      sessionStorage.setItem("orange-admin-token", "mock-products-admin");
      localStorage.setItem("welcome_seen", "1");
      Object.defineProperty(navigator, "clipboard", {
        value: {
          writeText: async (text) => {
            window.__copied = text;
          },
        },
      });
    });
    await fixture(context);
    const page = await context.newPage();
    page.on("pageerror", (failure) => errors.push(failure.message));
    await page.goto(origin + "/admin");
    await page.getByRole("button", { name: "运营管理", exact: true }).click();
    await page
      .locator(".admin-sidebar")
      .getByRole("button", { name: "商品库", exact: true })
      .click();
    await page
      .locator(".product-table")
      .getByText("40,000", { exact: true })
      .waitFor();
    await page.getByLabel("平台分成", { exact: true }).fill("25");
    assert.equal(
      await page.getByLabel("默认有效期", { exact: true }).count(),
      0,
    );
    await page.getByRole("button", { name: "保存设置", exact: true }).click();
    await page.getByText("商品设置已保存", { exact: true }).waitFor();
    assert.deepEqual(state.configs[0], {
      platform_share_percent: 25,
      default_valid_days: 11,
    });

    await page.getByRole("button", { name: "新增商品", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "新增商品" });
    const start = await dialog
        .getByLabel("生效时间", { exact: true })
        .inputValue(),
      end = await dialog.getByLabel("到期时间", { exact: true }).inputValue();
    assert.equal(new Date(end) - new Date(start), 7 * 86400000);
    assert.equal(
      await dialog.getByLabel("生效时间", { exact: true }).getAttribute("step"),
      "1",
    );
    assert.equal(
      await dialog.getByLabel("到期时间", { exact: true }).getAttribute("step"),
      "1",
    );
    await dialog.getByLabel("商品图片", { exact: true }).setInputFiles({
      name: "product-test.png",
      mimeType: "image/png",
      buffer: imageBytes,
    });
    await dialog.getByLabel("商品标题", { exact: true }).fill("日用精选纸巾");
    await dialog
      .getByLabel("商品链接", { exact: true })
      .fill("https://shop-preview.test/item/11");
    await dialog.getByLabel("券后价（元）", { exact: true }).fill("12.30");
    await dialog.getByLabel("返佣比例（%）", { exact: true }).fill("15");
    await dialog.getByLabel("平台", { exact: true }).fill("京东");
    await page.screenshot({ path: path.join(output, "product-editor.png") });
    await dialog.getByRole("button", { name: "保存商品", exact: true }).click();
    await page.getByText("商品已保存", { exact: true }).waitFor();
    assert.equal(state.changes[0].image, true);
    assert.equal(Number(state.changes[0].fields.price), 12.3);
    assert.equal(state.changes[0].fields.commission_percent, "15");
    assert.equal(state.changes[0].fields.enabled, "true");
    assert.ok(state.changes[0].fields.expires_at.endsWith("Z"));
    const row = page.getByRole("row").filter({ hasText: "日用精选纸巾" });
    await row.getByRole("button", { name: "编辑", exact: true }).click();
    await page
      .getByLabel("商品标题", { exact: true })
      .fill("日用精选纸巾升级装");
    await page.getByRole("button", { name: "保存商品", exact: true }).click();
    await page
      .getByRole("row")
      .filter({ hasText: "日用精选纸巾升级装" })
      .waitFor();
    assert.deepEqual(state.changes[1].fields, { title: "日用精选纸巾升级装" });
    const oldCupExpiry = state.products.find(
      (item) => item.id === 10,
    ).expires_at;
    await page
      .getByRole("row")
      .filter({ hasText: "日用精选纸巾升级装" })
      .getByRole("button", { name: "编辑", exact: true })
      .click();
    const scheduledStart = new Date(now + 86400000 + 19000);
    const scheduledExpiry = new Date(now + 3 * 86400000 + 37000);
    const local = (value) =>
      new Date(value.getTime() - value.getTimezoneOffset() * 60000)
        .toISOString()
        .slice(0, 19);
    await page
      .getByLabel("生效时间", { exact: true })
      .fill(local(scheduledStart));
    await page
      .getByLabel("到期时间", { exact: true })
      .fill(local(scheduledExpiry));
    await page.getByRole("button", { name: "保存商品", exact: true }).click();
    await page
      .getByRole("row")
      .filter({ hasText: "日用精选纸巾升级装" })
      .getByText("待生效", { exact: true })
      .waitFor();
    assert.deepEqual(state.changes[2].fields, {
      starts_at: scheduledStart.toISOString(),
      expires_at: scheduledExpiry.toISOString(),
    });
    assert.equal(
      state.products.find((item) => item.id === 10).expires_at,
      oldCupExpiry,
    );
    assert.match(
      await page
        .getByRole("row")
        .filter({ hasText: "日用精选纸巾升级装" })
        .locator(".product-remaining")
        .innerText(),
      /^距生效 /,
    );
    assert.match(
      await page
        .getByRole("row")
        .filter({ hasText: "轻巧随行保温杯" })
        .locator(".product-remaining")
        .innerText(),
      /^剩余 /,
    );
    await page
      .getByRole("row")
      .filter({ hasText: "日用精选纸巾升级装" })
      .getByRole("button", { name: "编辑", exact: true })
      .click();
    await page.getByLabel("商品标题", { exact: true }).fill("被拒绝的修改");
    state.saveError = "商品链接不被支持";
    await page.getByRole("button", { name: "保存商品", exact: true }).click();
    await page
      .getByRole("alert")
      .filter({ hasText: state.saveError })
      .waitFor();
    assert.equal(
      await page.getByLabel("商品标题", { exact: true }).inputValue(),
      "被拒绝的修改",
    );
    await page
      .getByRole("button", { name: "关闭商品编辑", exact: true })
      .click();
    state.saveError = "";
    await page.getByLabel("商品状态", { exact: true }).selectOption("expired");
    await page.getByText("暂无符合条件的商品", { exact: true }).waitFor();
    assert.equal(state.queries.at(-1).status, "expired");
    await page.getByLabel("商品状态", { exact: true }).selectOption("all");
    await page
      .getByRole("button", { name: "生成接入密钥", exact: true })
      .click();
    await page.getByLabel("商品接入密钥", { exact: true }).waitFor();
    assert.equal(
      await page.getByLabel("商品接入密钥").inputValue(),
      "mock-product-key-1",
    );
    await page.getByRole("button", { name: "复制", exact: true }).click();
    assert.equal(
      await page.evaluate(() => window.__copied),
      "mock-product-key-1",
    );
    await page.getByRole("button", { name: "轮换密钥", exact: true }).click();
    assert.equal(state.keyWrites, 1);
    await page.getByRole("button", { name: "取消", exact: true }).click();
    assert.equal(state.keyWrites, 1);
    await page.getByRole("button", { name: "轮换密钥", exact: true }).click();
    await page.getByRole("button", { name: "确认轮换", exact: true }).click();
    await page.getByText("接入密钥已生成", { exact: true }).waitFor();
    assert.equal(state.keyWrites, 2);
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page.getByLabel("商品接入密钥", { exact: true }).waitFor();
    assert.equal(
      await page.getByLabel("商品接入密钥").inputValue(),
      "mock-product-key-2",
    );
    await page.getByText("JSON 示例", { exact: true }).click();
    const jsonExample = JSON.parse(
      await page.locator(".product-example pre").innerText(),
    );
    assert.ok(jsonExample.image_base64 && jsonExample.expires_at);
    assert.equal(
      await page
        .getByRole("link", { name: "接口文档", exact: true })
        .getAttribute("href"),
      "/docs/product-api",
    );
    const docs = await context.newPage();
    await docs.setViewportSize({ width: 390, height: 844 });
    const docsApiRequests = [];
    docs.on("request", (request) => {
      if (new URL(request.url()).pathname.startsWith("/api/"))
        docsApiRequests.push(request.url());
    });
    await docs.goto(origin + "/docs/product-api");
    await docs
      .getByRole("heading", { name: "商品接入文档", exact: true })
      .waitFor();
    assert.equal(
      await docs.getByRole("link", { name: "下载 Markdown", exact: true }).count(),
      0,
    );
    let publicExample = JSON.parse(
      await docs.locator(".product-doc-code pre").last().innerText(),
    );
    assert.ok(publicExample.image_base64 && publicExample.expires_at);
    await docs.getByRole("button", { name: "先上传图片", exact: true }).click();
    publicExample = JSON.parse(
      await docs.locator(".product-doc-code pre").last().innerText(),
    );
    assert.ok(publicExample.image_id && publicExample.expires_at);
    assert.equal(Object.hasOwn(publicExample, "image_base64"), false);
    assert.match(
      await docs.locator(".product-doc-python").innerText(),
      /\/api\/integration\/product-images/,
    );
    assert.equal(
      await docs.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
    assert.deepEqual(docsApiRequests, []);
    await docs.screenshot({
      path: path.join(output, "product-api-docs-mobile.png"),
    });
    await docs.setViewportSize({ width: 1440, height: 1050 });
    await docs.screenshot({ path: path.join(output, "product-api-docs.png") });
    await docs.close();
    await page.screenshot({
      path: path.join(output, "product-library-admin.png"),
    });
    await page.setViewportSize({ width: 390, height: 844 });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
    await page.screenshot({
      path: path.join(output, "product-library-mobile.png"),
    });
    state.denies = true;
    await page.getByRole("button", { name: "刷新数据", exact: true }).click();
    await page.locator(".admin-login-card").waitFor();
    assert.equal(await page.locator(".product-key").count(), 0);
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("orange-admin-token")),
      null,
    );

    state.denies = false;
    const consumer = await context.newPage();
    await consumer.setViewportSize({ width: 390, height: 844 });
    consumer.on("pageerror", (failure) => errors.push(failure.message));
    await consumer.goto(origin + "/");
    await consumer
      .getByRole("heading", { name: "把每天的", exact: false })
      .waitFor();
    assert.equal(await consumer.locator(".phone-shell").count(), 0);
    assert.equal(await consumer.locator(".tab-bar").count(), 0);
    assert.equal(
      await consumer.getByRole("link", { name: "商品接入", exact: true }).count(),
      0,
    );
    await consumer.screenshot({
      path: path.join(output, "products-consumer.png"),
    });
    assert.deepEqual(state.unexpected, []);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        uploadRealImageBytes: true,
        patchOnlyChangedFields: true,
        configuration: true,
        keyViewCopyAndConfirmedRotation: true,
        publicLanding: true,
        internalDocsHidden: true,
        admin401Clears: true,
        mobileOverflow: false,
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
