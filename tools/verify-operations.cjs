const { chromium } = require('playwright');
const path = require('node:path');
const fs = require('node:fs');
const assert = require('node:assert/strict');
let browser;
const base = 'http://127.0.0.1:38180';
async function api(endpoint, body, token) {
  const response = await fetch(base + '/api' + endpoint, {
    method: body ? 'POST' : 'GET',
    headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: 'Bearer ' + token } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await response.json();
  if (!response.ok) throw new Error(JSON.stringify(data));
  return data;
}
(async () => {
  const auth = await api('/auth/login', { phone: '13800138000', password: 'Orange123!' });
  const before = auth.user.coins;
  browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  const output = path.resolve(__dirname, '../output');
  await page.addInitScript(token => localStorage.setItem('orange-token', token), auth.token);
  await page.goto(base);
  await page.locator('.balance-pill b').filter({ hasText: before.toLocaleString('zh-CN') }).waitFor();
  await page.locator('.tab-bar button').filter({ hasText: '我的' }).click();
  await page.waitForTimeout(700);
  await page.screenshot({ path: path.join(output, 'web-mobile-mine.png') });
  await page.getByRole('button', { name: '去提现' }).click();
  await page.getByLabel('收款账号').fill('local-demo-account');
  await page.getByRole('button', { name: '确认提现', exact: true }).click();
  await page.locator('.content-sheet').waitFor({ state: 'hidden' });
  assert.equal((await api('/me', null, auth.token)).coins, before - 1000);
  const admin = await browser.newPage({ viewport: { width: 1440, height: 1080 } });
  await admin.goto(base + '/admin');
  await admin.locator('.platform-grid b').first().waitFor();
  await admin.getByRole('button', { name: '提现管理', exact: true }).click();
  const record = admin.getByRole('row').filter({ hasText: 'local-demo-account' }).filter({ hasText: '模拟申请' });
  await record.getByRole('button', { name: '驳回', exact: true }).click();
  await admin.getByRole('row').filter({ hasText: 'local-demo-account' }).filter({ hasText: '模拟已退回' }).first().waitFor();
  assert.equal((await api('/me', null, auth.token)).coins, before);
  await admin.screenshot({ path: path.join(output, 'web-admin-withdrawals.png'), fullPage: true });
  await admin.getByRole('button', { name: '推送策略', exact: true }).click();
  await admin.getByLabel('欢迎语', { exact: true }).waitFor();
  await admin.getByRole('button', { name: '保存策略', exact: true }).click();
  await admin.locator('.admin-toast').filter({ hasText: '推送策略已保存' }).waitFor();
  await admin.getByRole('button', { name: '数据概览', exact: true }).click();
  await admin.screenshot({ path: path.join(output, 'web-admin.png'), fullPage: true });
  await page.reload();
  await page.locator('.chat-packet').first().waitFor();
  await page.waitForTimeout(1000);
  await page.screenshot({ path: path.join(output, 'web-mobile-hall.png') });
  await page.locator('.tab-bar button').filter({ hasText: '捡漏' }).click();
  await page.waitForTimeout(700);
  await page.screenshot({ path: path.join(output, 'web-mobile-deals.png') });
  const result = { passed: true, withdrawalDebit: 1000, refundCoins: 1000, finalCoins: before, strategySaved: true };
  fs.writeFileSync(path.join(output, 'operations-verification.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
  await browser.close();
})().catch(async error => { console.error(error); if (browser) await browser.close(); process.exitCode = 1; });
