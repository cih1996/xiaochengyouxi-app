const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const base = "http://127.0.0.1:38180";
async function api(
  route,
  { body, method = body ? "POST" : "GET", token } = {},
) {
  const response = await fetch(base + "/api" + route, {
    method,
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: "Bearer " + token } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await response.json();
  assert.ok(response.ok, data.error);
  return data;
}
(async () => {
  const original = await api("/admin/reward-policy");
  const originalSettings = await api("/admin/settings");
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  const errors = [],
    output = path.join(__dirname, "output");
  fs.mkdirSync(output, { recursive: true });
  try {
    const admin = await browser.newPage({
      viewport: { width: 1512, height: 1100 },
    });
    admin.on("pageerror", (error) => errors.push(error.message));
    await admin.goto(base + "/admin");
    await admin.getByRole("button", { name: "运营设置", exact: true }).click();
    await admin.getByLabel("启用阶梯奖励", { exact: true }).check();
    await admin.getByLabel("未知设备最低金币", { exact: true }).fill("17");
    await admin.getByLabel("未知设备最高金币", { exact: true }).fill("19");
    const savedPromise = admin.waitForResponse(
      (response) =>
        response.url().endsWith("/api/admin/reward-policy") &&
        response.request().method() === "PUT",
    );
    await admin
      .getByRole("button", { name: "保存奖励策略", exact: true })
      .click();
    const saved = await (await savedPromise).json();
    assert.ok(saved.policy_version > original.policy_version);
    assert.equal(saved.unknown_device.min_coins, 17);
    assert.equal(
      await admin.getByLabel("平台奖励名称", { exact: true }).inputValue(),
      originalSettings.csj_expected_reward_name,
    );
    assert.equal(
      Number(
        await admin.getByLabel("平台奖励数量", { exact: true }).inputValue(),
      ),
      originalSettings.csj_expected_reward_amount,
    );
    await admin.getByText("奖励策略已保存", { exact: true }).waitFor();
    assert.equal(
      await admin.getByLabel("红包金币数", { exact: true }).count(),
      0,
    );
    await admin.screenshot({
      path: path.join(output, "reward-policy.png"),
      fullPage: true,
    });

    const platform = await api("/admin/invitations"),
      phone = "136" + String(Date.now()).slice(-8);
    await api("/invite/prebind", {
      body: { phone, invite_code: platform.platform_code },
    });
    await api("/auth/sms", { body: { phone, purpose: "register" } });
    const account = await api("/auth/register", {
      body: {
        phone,
        password: "RewardPolicy123!",
        code: "123456",
        nickname: "阶梯奖励验证",
      },
    });
    const user = await browser.newPage({
      viewport: { width: 390, height: 844 },
    });
    user.on("pageerror", (error) => errors.push(error.message));
    await user.addInitScript((token) => {
      localStorage.setItem("orange-token", token);
      localStorage.setItem("welcome_seen", "1");
    }, account.token);
    await user.goto(base);
    const quoteResponse = user.waitForResponse((response) =>
      response.url().endsWith("/api/rewards/quote"),
    );
    await user.getByRole("button", { name: "去领红包", exact: true }).click();
    const quote = await (await quoteResponse).json();
    assert.ok(quote.reward >= 17 && quote.reward <= 19);
    assert.equal(quote.task.reward, quote.reward);
    for (const key of [
      "reward_tier",
      "policy_version",
      "device_first_seen",
      "reward_basis",
    ])
      assert.equal(
        quote[key],
        undefined,
        "private field absent publicly: " + key,
      );
    await user.waitForFunction(
      (reward) =>
        document.querySelector(".packet-reward b")?.textContent ===
        String(reward),
      quote.reward,
    );
    await user
      .locator(".packet-modal")
      .evaluate((element) =>
        Promise.all(
          element.getAnimations().map((animation) => animation.finished),
        ),
      );
    await user.screenshot({ path: path.join(output, "reward-quote.png") });

    const startedResponse = user.waitForResponse((response) =>
      response.url().endsWith("/api/rewards/start"),
    );
    const completedResponse = user.waitForResponse((response) =>
      /\/api\/rewards\/[^/]+\/complete$/.test(response.url()),
    );
    await user
      .getByRole("button", { name: "看视频领金币", exact: true })
      .click();
    const startResponse = await startedResponse,
      session = await startResponse.json();
    assert.equal(
      startResponse.request().postDataJSON().quote_id,
      quote.quote_id,
    );
    assert.equal(
      session.reward,
      quote.reward,
      "session uses the displayed quote",
    );
    await api("/admin/reward-policy", {
      method: "PUT",
      body: { ...saved, unknown_device: { min_coins: 37, max_coins: 39 } },
    });
    await user
      .getByText("看完可领 " + session.reward + " 金币", { exact: true })
      .waitFor();
    await user.screenshot({ path: path.join(output, "reward-session.png") });
    const completed = await (await completedResponse).json();
    assert.equal(completed.reward, session.reward);
    assert.equal(completed.user.coins, account.user.coins + session.reward);
    await user.locator(".won-number").waitFor();
    assert.ok(
      (await user.locator(".won-number").innerText()).includes(
        String(session.reward),
      ),
    );
    assert.equal(
      (await api("/me", { token: account.token })).coins,
      completed.user.coins,
    );
    const ledger = await api("/ledger", { token: account.token });
    assert.ok(ledger.some((row) => row.coins === session.reward));

    await user.getByRole("button", { name: "开心收下", exact: true }).click();
    const staleResponse = user.waitForResponse((response) =>
      response.url().endsWith("/api/rewards/quote"),
    );
    await user.getByRole("button", { name: "去领红包", exact: true }).click();
    const stale = await (await staleResponse).json();
    assert.ok(stale.reward >= 37 && stale.reward <= 39);
    await api("/admin/reward-policy", {
      method: "PUT",
      body: { ...saved, unknown_device: { min_coins: 47, max_coins: 49 } },
    });
    const rejectedResponse = user.waitForResponse((response) =>
      response.url().endsWith("/api/rewards/start"),
    );
    await user
      .getByRole("button", { name: "看视频领金币", exact: true })
      .click();
    const rejected = await rejectedResponse;
    assert.equal(rejected.status(), 410);
    assert.equal((await rejected.json()).code, "QUOTE_EXPIRED");
    await user
      .getByRole("button", { name: "重新打开红包", exact: true })
      .waitFor();
    assert.equal(await user.locator(".stage-video").count(), 0);
    const reopenedResponse = user.waitForResponse((response) =>
      response.url().endsWith("/api/rewards/quote"),
    );
    await user
      .getByRole("button", { name: "重新打开红包", exact: true })
      .click();
    const reopened = await (await reopenedResponse).json();
    assert.ok(reopened.reward >= 47 && reopened.reward <= 49);
    await user
      .getByRole("button", { name: "看视频领金币", exact: true })
      .waitFor();
    assert.equal(
      await user.locator(".stage-video").count(),
      0,
      "reopening presents the new quote before watching",
    );
    assert.equal(
      (await api("/me", { token: account.token })).coins,
      completed.user.coins,
    );
    await user.getByRole("button", { name: "关闭", exact: true }).click();

    await admin.getByRole("button", { name: "用户管理", exact: true }).click();
    await admin.getByLabel("搜索用户", { exact: true }).fill(phone);
    await admin.getByRole("button", { name: "查询", exact: true }).click();
    await admin
      .getByRole("button", { name: "查看用户 " + phone, exact: true })
      .click();
    await admin.getByRole("button", { name: "广告会话", exact: true }).click();
    const row = admin
      .locator("tr")
      .filter({ has: admin.locator(`[title="${session.id}"]`) });
    await row.getByRole("button", { name: "明细", exact: true }).click();
    await admin.getByText("奖励发放依据", { exact: true }).waitFor();
    const panel = admin.locator(".ud-reward-proof");
    assert.ok((await panel.innerText()).includes("未知设备保守档"));
    assert.ok((await panel.innerText()).includes("V" + saved.policy_version));
    const records = await api(
        `/admin/users/${account.user.id}/records?section=ads`,
      ),
      record = records.items.find((row) => row.id === session.id);
    assert.equal(record.reward, session.reward);
    assert.equal(record.reward_tier, "unknown_device");
    assert.equal(record.policy_version, saved.policy_version);
    assert.equal(record.quote_id, quote.quote_id);
    assert.ok(record.reward_basis);
    await admin.screenshot({
      path: path.join(output, "reward-basis.png"),
      fullPage: true,
    });
    await admin
      .getByRole("button", { name: "关闭记录明细", exact: true })
      .click();

    await admin.getByRole("button", { name: "运营设置", exact: true }).click();
    await admin.getByLabel("启用阶梯奖励", { exact: true }).uncheck();
    await admin.getByLabel("固定奖励（金币）", { exact: true }).fill("43");
    const fixedSaved = admin.waitForResponse(
      (response) =>
        response.url().endsWith("/api/admin/reward-policy") &&
        response.request().method() === "PUT",
    );
    await admin
      .getByRole("button", { name: "保存奖励策略", exact: true })
      .click();
    assert.equal((await (await fixedSaved).json()).fixed_coins, 43);
    const fixedQuote = await api("/rewards/quote", {
      token: account.token,
      body: { task_id: quote.task_id },
    });
    assert.equal(fixedQuote.reward, 43);
    const publicTasks = await api("/tasks", { token: account.token });
    assert.ok(publicTasks.length > 0);
    assert.ok(
      publicTasks.every(
        (task) => task.reward_min === 43 && task.reward_max === 43,
      ),
    );
    assert.equal(
      (await api("/admin/settings")).csj_expected_reward_amount,
      originalSettings.csj_expected_reward_amount,
      "business coins do not reconfigure platform verification amount",
    );
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        status: "PASS",
        account_id: account.user.id,
        policy_version: saved.policy_version,
        quote_reward: quote.reward,
        credited_reward: completed.reward,
        fixed_reward: fixedQuote.reward,
        screenshots: [
          "reward-policy.png",
          "reward-quote.png",
          "reward-session.png",
          "reward-basis.png",
        ],
      }),
    );
  } finally {
    await api("/admin/reward-policy", { method: "PUT", body: original });
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
