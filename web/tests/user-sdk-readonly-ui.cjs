const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const base = "http://127.0.0.1:38182";
const settings = Object.fromEntries(
  fs
    .readFileSync(path.join(__dirname, "../../private/server.env"), "utf8")
    .split(/\r?\n/)
    .filter((line) => line && !line.startsWith("#") && line.includes("="))
    .map((line) => {
      const index = line.indexOf("=");
      return [
        line.slice(0, index),
        line.slice(index + 1).replace(/^['"]|['"]$/g, ""),
      ];
    }),
);
const adminToken = settings.ORANGE_ADMIN_TOKEN;
async function api(route) {
  const response = await fetch(base + "/api" + route, {
    headers: { "x-admin-token": adminToken },
  });
  const data = await response.json();
  assert.ok(response.ok, "read-only admin API must authenticate");
  return data;
}
(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  try {
    const users = await api("/admin/users/search?page_size=100");
    const candidates = await Promise.all(
      users.items.map(async (user) => ({
        user,
        records: await api(
          "/admin/users/" + user.id + "/records?section=risk&page_size=100",
        ),
      })),
    );
    const sample = candidates.find((candidate) =>
      candidate.records.items.some(
        (row) =>
          row.status === "reported" &&
          row.detail?.reportProof?.level === "signature_only",
      ),
    );
    assert.ok(
      sample,
      "real SDK verified report is required for this read-only check",
    );
    const row = sample.records.items.find(
      (row) =>
        row.status === "reported" &&
        row.detail?.reportProof?.level === "signature_only",
    );
    const output = path.join(__dirname, "output");
    fs.mkdirSync(output, { recursive: true });
    const page = await browser.newPage({
        viewport: { width: 1512, height: 1150 },
      }),
      errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.addInitScript(
      (value) => sessionStorage.setItem("orange-admin-token", value),
      adminToken,
    );
    await page.route("http://127.0.0.1:38180/api/**", async route => {
      const url = new URL(route.request().url());
      const response = await route.fetch({url:base + url.pathname + url.search});
      await route.fulfill({response});
    });
    await page.goto("http://127.0.0.1:38180/admin");
    await page.getByRole("button", { name: "用户管理", exact: true }).click();
    await page.getByLabel("搜索用户", { exact: true }).fill(sample.user.phone);
    await page.getByRole("button", { name: "查询", exact: true }).click();
    await page
      .getByRole("button", {
        name: "查看用户 " + sample.user.phone,
        exact: true,
      })
      .click();
    await page.locator(".ud-name h1").waitFor();
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "风险证据", exact: true })
      .click();
    await page.getByLabel("关联编号", { exact: true }).fill(row.report_id);
    await page
      .locator(".ud-filter-actions")
      .getByRole("button", { name: "筛选", exact: true })
      .click();
    await page.locator(".ud-table-scroll tbody tr").first().waitFor();
    await page.screenshot({
      path: path.join(output, "user-real-sdk-risk.png"),
      fullPage: true,
    });
    await page
      .locator(".ud-table-scroll tbody tr")
      .first()
      .getByRole("button", { name: "查看证据", exact: true })
      .click();
    await page.locator(".ud-record-detail").waitFor();
    assert.ok(
      (await page.locator(".ud-proof-summary").textContent()).includes(
        "持钥签名",
      ),
    );
    assert.ok(
      (await page.locator(".ud-proof-summary").textContent()).includes(
        String(row.score),
      ),
    );
    assert.equal(
      await page.getByText("硬件已验证", { exact: true }).count(),
      0,
    );
    await page.screenshot({
      path: path.join(output, "user-real-sdk-evidence.png"),
      fullPage: true,
    });
    await page
      .getByRole("button", { name: "关闭记录明细", exact: true })
      .click();
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "关联设备", exact: true })
      .click();
    await page.locator(".ud-table-scroll tbody tr").first().waitFor();
    assert.ok(
      (await page.locator(".ud-table-scroll").textContent()).includes(
        "持钥签名",
      ),
    );
    await page.screenshot({
      path: path.join(output, "user-real-sdk-devices.png"),
      fullPage: true,
    });
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "行为时间线", exact: true })
      .click();
    await page
      .getByLabel("类型", { exact: true })
      .selectOption("risk_assessment");
    await page
      .locator(".ud-filter-actions")
      .getByRole("button", { name: "筛选", exact: true })
      .click();
    await page.locator(".ud-timeline article").first().waitFor();
    await page.screenshot({
      path: path.join(output, "user-real-sdk-timeline.png"),
      fullPage: true,
    });
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        realSdkReadOnly: true,
        signatureOnlyLabel: true,
        scorePreserved: true,
        deviceEvidence: true,
        riskTimeline: true,
        ruleHits: row.detail.hits.length,
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
