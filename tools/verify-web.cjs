const { chromium } = require('playwright');
const path = require('node:path');
const fs = require('node:fs');
const assert = require('node:assert/strict');
let browser;

(async () => {
  const output = path.resolve(__dirname, '../output');
  browser = await chromium.launch({
    executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe',
    headless: true,
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1080 } });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('http://127.0.0.1:38180');
  await page.getByRole('button', { name: '登录领福利', exact: true }).waitFor();
  await page.screenshot({ path: path.join(output, 'web-showcase.png'), fullPage: true });
  await page.getByRole('button', { name: '登录领福利', exact: true }).click();
  await page.getByRole('button', { name: '登录', exact: true }).click();
  await page.locator('.auth-sheet').waitFor({ state: 'hidden' });
  await page.locator('.chat-packet').first().waitFor();
  const before = Number((await page.locator('.balance-pill b').textContent()).replace(/,/g, ''));
  await page.locator('.chat-packet').first().click();
  await page.screenshot({ path: path.join(output, 'web-packet.png') });
  await page.getByRole('button', { name: '看视频领金币' }).click();
  await page.locator('.stage-won').waitFor({ timeout: 12000 });
  const reward = Number((await page.locator('.won-number').textContent()).replace(/[^\d]/g, ''));
  const after = Number((await page.locator('.balance-pill b').textContent()).replace(/,/g, ''));
  assert.equal(after - before, reward, 'displayed balance must match credited reward');
  await page.screenshot({ path: path.join(output, 'web-reward.png') });
  await page.getByRole('button', { name: '开心收下', exact: true }).click();
  await page.locator('.tab-bar button').filter({ hasText: '捡漏' }).click();
  assert.equal(await page.locator('.product-card').count(), 4);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: path.join(output, 'web-mobile-deals.png') });
  await page.locator('.product-card').first().click();
  await page.locator('.detail-commission').waitFor();
  await page.locator('.sheet-heading button').click();
  await page.locator('.tab-bar button').filter({ hasText: '我的' }).click();
  await page.screenshot({ path: path.join(output, 'web-mobile-mine.png') });
  await page.locator('.tab-bar button').filter({ hasText: '大厅' }).click();
  await page.screenshot({ path: path.join(output, 'web-mobile-hall.png') });
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
  assert.equal(overflow, false, 'mobile viewport should have no horizontal overflow');
  await page.setViewportSize({ width: 1440, height: 1080 });
  await page.goto('http://127.0.0.1:38180/admin');
  await page.locator('.stat-card').first().waitFor();
  await page.locator('.platform-grid b').first().waitFor();
  await page.screenshot({ path: path.join(output, 'web-admin.png'), fullPage: true });
  await page.getByRole('button', { name: '用户管理', exact: true }).click();
  await page.getByRole('cell', { name: '13800138000', exact: true }).waitFor();
  assert.deepEqual(errors, [], 'no browser runtime errors');
  const result = { passed: true, reward, balanceBefore: before, balanceAfter: after, products: 4, mobileOverflow: overflow, runtimeErrors: errors };
  fs.writeFileSync(path.join(output, 'web-verification.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
  await browser.close();
})().catch(async (error) => { console.error(error); if (browser) await browser.close(); process.exitCode = 1; });
