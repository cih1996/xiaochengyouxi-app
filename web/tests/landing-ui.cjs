const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs"),
  path = require("node:path");
const dist = path.join(__dirname, "../dist"),
  origin = "https://landing-preview.test";
async function fixture(context, requests) {
  await context.route("**/*", async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    if (url.pathname.startsWith("/api/")) {
      requests.push(url.pathname);
      if (url.pathname === "/api/config")
        return route.fulfill({
          json: {
            app_download_url: "https://download.test/orange.apk",
            app_name: "小橙有喜",
          },
        });
      return route.fulfill({
        status: 404,
        json: { error: "首页不应调用此接口" },
      });
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
async function check(userAgent, expectedText, isIos = false) {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  const requests = [];
  try {
    const context = await browser.newContext({
      userAgent,
      viewport: { width: 390, height: 844 },
    });
    await fixture(context, requests);
    const page = await context.newPage();
    await page.goto(origin + "/");
    await page
      .getByRole("heading", { name: "把每天的", exact: false })
      .waitFor();
    assert.equal(
      await page.getByText(expectedText, { exact: true }).count(),
      1,
    );
    assert.equal(await page.locator(".phone-shell").count(), 0);
    assert.equal(await page.locator(".tab-bar").count(), 0);
    assert.equal(
      await page.getByRole("link", { name: "管理入口", exact: false }).count(),
      1,
    );
    const cta = page.locator(".landing-download");
    if (isIos) assert.equal(await cta.isDisabled(), true);
    else
      assert.equal(
        await cta.getAttribute("href"),
        "https://download.test/orange.apk",
      );
    assert.deepEqual(requests, ["/api/config"]);
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
  } finally {
    await browser.close();
  }
}
(async () => {
  await check(
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/130 Mobile Safari/537.36",
    "下载安卓版",
  );
  await check(
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148",
    "iOS 版本暂未开放",
    true,
  );
  await check(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130 Safari/537.36",
    "下载安卓版",
  );
  console.log(
    JSON.stringify({
      passed: true,
      androidCTA: true,
      iosNotice: true,
      desktopCTA: true,
      noPhoneSimulator: true,
      noAccountApis: true,
      mobileWidth: false,
    }),
  );
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
