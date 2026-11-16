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
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const phone = "139" + String(Date.now()).slice(-8);
    const initialPassword = "OrangeUi123!";
    const newPassword = "OrangeUi456!";
    const invitations = await (
      await fetch("http://127.0.0.1:38180/api/admin/invitations")
    ).json();
    const prebind = await fetch("http://127.0.0.1:38180/api/invite/prebind", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ phone, invite_code: invitations.platform_code }),
    });
    assert.ok(prebind.ok, await prebind.text());
    await page.addInitScript(() => localStorage.setItem("welcome_seen", "1"));
    await page.goto("http://127.0.0.1:38180");
    await page.getByRole("button", { name: "登录领福利", exact: true }).click();
    await page.getByRole("button", { name: "注册账号", exact: true }).click();
    await page.getByLabel("手机号", { exact: true }).fill(phone);
    await page.getByLabel("昵称", { exact: true }).fill("账号体验");
    await page.getByLabel("密码", { exact: true }).fill(initialPassword);
    await page.getByRole("button", { name: "获取验证码", exact: true }).click();
    await page
      .locator(".auth-sheet small")
      .getByText("测试验证码：123456", { exact: true })
      .waitFor();
    assert.equal(
      await page.locator(".input-action input").inputValue(),
      "123456",
    );
    await page.getByRole("button", { name: "注册并登录", exact: true }).click();
    await page.locator(".auth-sheet").waitFor({ state: "hidden" });
    assert.ok(await page.evaluate(() => localStorage.getItem("orange-token")));

    await page.locator(".tab-bar button").filter({ hasText: "我的" }).click();
    await page
      .getByRole("heading", { name: "账号体验", exact: true })
      .waitFor();
    await page.locator(".app-top .icon-button").click();
    await page.getByRole("button", { name: "修改密码", exact: true }).click();
    await page.getByLabel("新密码", { exact: true }).fill(newPassword);
    await page.getByRole("button", { name: "重新获取", exact: true }).click();
    await page.getByRole("button", { name: "确认修改", exact: true }).click();
    await page
      .getByRole("heading", { name: "欢迎回到小橙有喜", exact: true })
      .waitFor();
    assert.equal(
      await page.evaluate(() => localStorage.getItem("orange-token")),
      null,
    );
    await page.getByLabel("密码", { exact: true }).fill(newPassword);
    await page
      .locator(".auth-sheet")
      .getByRole("button", { name: "登录", exact: true })
      .click();
    await page.locator(".auth-sheet").waitFor({ state: "hidden" });
    assert.ok(await page.evaluate(() => localStorage.getItem("orange-token")));
    await page
      .getByRole("heading", { name: "账号体验", exact: true })
      .waitFor();

    await page.locator(".app-top .icon-button").click();
    await page.getByRole("button", { name: "退出登录", exact: true }).click();
    await page.locator(".content-sheet").waitFor({ state: "hidden" });
    assert.equal(
      await page.evaluate(() => localStorage.getItem("orange-token")),
      null,
    );
    await page
      .getByRole("heading", { name: "欢迎来到小橙有喜", exact: true })
      .waitFor();

    await page.evaluate(() =>
      localStorage.setItem("orange-token", "expired-ui-test-token"),
    );
    await page.reload();
    await page
      .getByRole("button", { name: "登录领福利", exact: true })
      .waitFor();
    assert.equal(
      await page.evaluate(() => localStorage.getItem("orange-token")),
      null,
    );
    assert.equal(await page.locator(".chat-packet").count(), 0);
    await page.getByText("登录已过期，请重新登录", { exact: true }).waitFor();
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        phone,
        registration: true,
        mockSms: true,
        passwordReset: true,
        newPasswordLogin: true,
        logout: true,
        expiredSessionCleanup: true,
        anonymousPacketsAfterExpiry: 0,
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
