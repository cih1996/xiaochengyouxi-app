const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
let browser;
const base = 'https://orange.lingzidev.com';
async function api(route, body) {
  const response = await fetch(base + '/api' + route, {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
  });
  const result = await response.json();
  assert.ok(response.ok, result.error);
  return result;
}
(async () => {
  const root = path.resolve(__dirname, '..');
  const output = path.join(root, 'output');
  const frames = path.join(output, 'checkin-frames');
  fs.mkdirSync(frames, { recursive: true });
  const phone = '135' + String(Date.now()).slice(-8), password = 'WelcomeUi031!';
  const code = (await api('/auth/sms', { phone, purpose: 'register' })).code;
  await api('/auth/register', { phone, password, code, nickname: '欢迎页体验' });
  browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(base);
  await page.getByRole('button', { name: '开启小欢喜' }).waitFor();
  await page.screenshot({ path: path.join(output, 'live-welcome.png') });
  await page.getByRole('button', { name: '开启小欢喜' }).click();
  await page.getByLabel('手机号', { exact: true }).fill(phone);
  await page.getByLabel('密码', { exact: true }).fill(password);
  await page.getByRole('button', { name: '显示密码', exact: true }).click();
  assert.equal(await page.getByLabel('密码', { exact: true }).getAttribute('type'), 'text');
  await page.getByRole('button', { name: '隐藏密码', exact: true }).click();
  await page.screenshot({ path: path.join(output, 'live-login-polished.png') });
  await page.locator('.auth-sheet').getByRole('button', { name: '登录', exact: true }).click();
  await page.locator('.invite-bind-page').waitFor();
  await page.getByLabel('邀请码', { exact: true }).fill('JZ8F6A');
  await page.getByRole('button', { name: '绑定并进入' }).click();
  await page.locator('.chat-packet').first().waitFor();
  await page.locator('.app-scroll').evaluate(view => { view.scrollTop = 0; });
  const pending = page.locator('button.checkin-pending');
  assert.equal(await pending.isEnabled(), true);
  await page.screenshot({ path: path.join(output, 'live-checkin-before.png') });
  for (let i = 0; i < 16; i++) {
    await page.screenshot({ path: path.join(frames, `${String(i).padStart(2, '0')}.png`), clip: { x: 0, y: 0, width: 390, height: 350 } });
    await page.waitForTimeout(150);
  }
  await pending.click();
  await page.locator('.stage-won').waitFor();
  await page.getByRole('button', { name: '开心收下', exact: true }).click();
  assert.equal(await page.locator('button.checkin-done').isDisabled(), true);
  await page.screenshot({ path: path.join(output, 'live-checkin-after.png') });
  await page.reload();
  await page.locator('button.checkin-done').waitFor();
  assert.equal(await page.locator('button.checkin-done').isDisabled(), true);
  assert.equal(await page.locator('.welcome-screen').count(), 0);
  assert.deepEqual(errors, []);
  fs.writeFileSync(path.join(output, 'welcome-checkin-live.json'), JSON.stringify({ passed: true, firstWelcome: true, passwordToggle: true, invitationGateKept: true, checkInDisabledAfterSuccess: true, checkInRestoredAfterReload: true, runtimeErrors: errors }, null, 2));
  console.log('PASS live welcome, login, invitation gate, check-in and persisted state');
  await browser.close();
})().catch(async error => { console.error(error); if (browser) await browser.close(); process.exitCode = 1; });
