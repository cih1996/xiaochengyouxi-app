const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const path = require("node:path");
const fs = require("node:fs");
const base = "http://127.0.0.1:38180";
async function request(route, body) {
  const response = await fetch(base + "/api" + route, {
    method: body ? "POST" : "GET",
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await response.json();
  assert.ok(response.ok, data.error);
  return data;
}
(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  try {
    const output = path.join(__dirname, "output");
    fs.mkdirSync(output, { recursive: true });
    const profile = await request("/admin/users/1/detail");
    let suffix = Number(String(Date.now()).slice(-8));
    const children = [];
    for (let i = 0; i < 3; i++) {
      const phone = "134" + String(suffix++ % 1e8).padStart(8, "0");
      await request("/invite/prebind", {
        phone,
        invite_code: profile.user.invite_code,
      });
      await request("/auth/sms", { phone, purpose: "register" });
      children.push(
        (
          await request("/auth/register", {
            phone,
            password: "Investigate123!",
            code: "123456",
            nickname: "调查界面验证" + (i + 1),
          })
        ).user,
      );
    }
    const page = await browser.newPage({
        viewport: { width: 1512, height: 1100 },
      }),
      errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page
      .context()
      .grantPermissions(["clipboard-read", "clipboard-write"]);
    await page.goto(base + "/admin");
    await page.getByRole("button", { name: "用户管理", exact: true }).click();
    await page.locator(".user-list-row").first().waitFor();
    assert.equal(await page.locator(".user-warning-summary").count(), 1);
    await page
      .locator(".user-list-pagination")
      .getByRole("button", { name: "下一页", exact: true })
      .click();
    await page.waitForFunction(() =>
      document
        .querySelector(".user-list-pagination")
        ?.textContent.includes("第 2 /"),
    );
    await page
      .locator(".user-list-pagination")
      .getByRole("button", { name: "上一页", exact: true })
      .click();
    await page.getByLabel("搜索用户", { exact: true }).fill(profile.user.phone);
    await page.getByRole("button", { name: "查询", exact: true }).click();
    await page
      .getByRole("button", {
        name: "查看用户 " + profile.user.phone,
        exact: true,
      })
      .waitFor();
    assert.equal(await page.locator(".user-list-row").count(), 1);
    await page.screenshot({
      path: path.join(output, "user-search.png"),
      fullPage: true,
    });
    await page
      .getByRole("button", {
        name: "查看用户 " + profile.user.phone,
        exact: true,
      })
      .click();
    await page.locator(".ud-name h1").waitFor();
    assert.equal(await page.locator(".ud-tabs button").count(), 10);
    assert.ok(
      (await page.locator(".ud-profile").textContent()).includes(
        profile.user.phone,
      ),
    );
    await page.screenshot({
      path: path.join(output, "user-overview.png"),
      fullPage: true,
    });
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "金币账本", exact: true })
      .click();
    await page.locator(".ud-table-scroll tbody tr").first().waitFor();
    await page.getByLabel("每页条数", { exact: true }).selectOption("10");
    await page.waitForFunction(() =>
      document
        .querySelector(".ud-pagination")
        ?.textContent.includes("第 1 / 2"),
    );
    assert.equal(await page.locator(".ud-table-scroll tbody tr").count(), 10);
    await page
      .locator(".ud-pagination")
      .getByRole("button", { name: "下一页", exact: true })
      .click();
    await page.waitForFunction(() =>
      document
        .querySelector(".ud-pagination")
        ?.textContent.includes("第 2 / 2"),
    );
    await page.getByLabel("类型", { exact: true }).selectOption("check_in");
    await page.getByLabel("状态", { exact: true }).selectOption("credited");
    await page.getByLabel("最低金额（元）", { exact: true }).fill("0.04");
    await page.getByLabel("最高金额（元）", { exact: true }).fill("0.06");
    await page
      .locator(".ud-filter-actions")
      .getByRole("button", { name: "筛选", exact: true })
      .click();
    await page.locator(".ud-table-scroll tbody tr").first().waitFor();
    const expected = await request(
      "/admin/users/1/records?section=ledger&kind=check_in&status=credited&min_amount=0.04&max_amount=0.06&page_size=10",
    );
    assert.equal(
      await page.locator(".ud-table-scroll tbody tr").count(),
      expected.items.length,
    );
    await page.locator(".ud-table-scroll .ud-id").first().click();
    assert.equal(
      await page.evaluate(() => navigator.clipboard.readText()),
      expected.items[0].source,
    );
    await page.screenshot({
      path: path.join(output, "user-ledger-filtered.png"),
      fullPage: true,
    });
    await page.getByLabel("最低金额（元）", { exact: true }).fill("2");
    await page.getByLabel("最高金额（元）", { exact: true }).fill("1");
    await page
      .locator(".ud-filter-actions")
      .getByRole("button", { name: "筛选", exact: true })
      .click();
    await page
      .getByRole("alert")
      .filter({ hasText: "最低金额不能大于最高金额" })
      .waitFor();
    assert.equal(await page.locator(".ud-table-scroll tbody tr").count(), 0);
    await page
      .locator(".ud-filter-actions")
      .getByRole("button", { name: "重置", exact: true })
      .click();
    await page.locator(".ud-table-scroll tbody tr").first().waitFor();
    for (const label of [
      "广告会话",
      "广告回调",
      "提现记录",
      "行为时间线",
      "风险证据",
      "关联设备",
      "管理审计",
    ]) {
      await page
        .locator(".ud-tabs")
        .getByRole("button", { name: label, exact: true })
        .click();
      await page.waitForFunction(
        () => !document.querySelector(".ud-record-loading"),
      );
      assert.equal(await page.locator(".ud-error").count(), 0);
    }
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "关联设备", exact: true })
      .click();
    await page.getByText("设备与风险服务尚未接入", { exact: true }).waitFor();
    assert.equal(await page.locator(".ud-table-scroll tbody tr").count(), 0);
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "邀请关系", exact: true })
      .click();
    await page
      .getByRole("button", { name: new RegExp(children[0].phone) })
      .waitFor();
    await page
      .getByRole("button", { name: new RegExp(children[0].phone) })
      .click();
    await page.waitForFunction(
      (phone) =>
        document.querySelector(".ud-profile")?.textContent.includes(phone),
      children[0].phone,
    );
    await page.getByRole("button", { name: "账户处理", exact: true }).click();
    await page.getByLabel("账户状态", { exact: true }).selectOption("review");
    await page
      .getByLabel("处理原因", { exact: true })
      .fill("本地界面验证：待核查");
    await page.getByRole("button", { name: "保存处理", exact: true }).click();
    await page.waitForFunction(
      () =>
        document.querySelector(".ud-name .ud-status")?.textContent === "待核查",
    );
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "管理审计", exact: true })
      .click();
    await page
      .getByRole("cell", { name: "本地界面验证：待核查", exact: true })
      .waitFor();
    await page.screenshot({
      path: path.join(output, "user-audit.png"),
      fullPage: true,
    });
    await page.getByRole("button", { name: "用户列表", exact: true }).click();
    await page.getByRole("button", { name: /待核查人数/ }).click();
    await page.getByLabel("账号状态", { exact: true }).selectOption("review");
    await page
      .getByRole("button", {
        name: "查看用户 " + children[0].phone,
        exact: true,
      })
      .waitFor();
    assert.ok(
      (await page.locator(".user-list-row").allTextContents()).every((row) =>
        row.includes("待核查"),
      ),
    );
    await page
      .getByRole("button", {
        name: "查看用户 " + children[0].phone,
        exact: true,
      })
      .click();
    await page.locator(".ud-name h1").waitFor();
    await page.setViewportSize({ width: 800, height: 1000 });
    await page.screenshot({
      path: path.join(output, "user-overview-tablet.png"),
      fullPage: true,
    });
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
    await page.route(
      "**/api/admin/users/*/records?section=ads*",
      (route) =>
        route.fulfill({ status: 500, json: { error: "上游查询错误原文" } }),
      { times: 1 },
    );
    await page
      .locator(".ud-tabs")
      .getByRole("button", { name: "广告会话", exact: true })
      .click();
    await page
      .getByRole("alert")
      .filter({ hasText: "上游查询错误原文" })
      .waitFor();
    assert.equal(await page.locator(".ud-table-scroll tbody tr").count(), 0);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        search: true,
        listPagination: true,
        tenSections: true,
        ledgerPagination: true,
        filters: true,
        idCopy: true,
        directErrors: true,
        missingSdkClearlyShown: true,
        relatedAccountNavigation: true,
        accountHandlingAudited: true,
        statusQuickFilter: true,
        tabletNoBodyOverflow: true,
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
