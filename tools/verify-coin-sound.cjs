const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
let browser;
const base = 'http://127.0.0.1:38180';
async function post(endpoint, body) {
  const response = await fetch(base + '/api' + endpoint, {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(JSON.stringify(result));
  return result;
}
(async () => {
  const phone = '137' + String(Date.now()).slice(-8);
  await post('/auth/sms', { phone, purpose: 'register' });
  const auth = await post('/auth/register', { phone, password: 'CoinSound123!', code: '123456' });
  browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(token => {
    localStorage.setItem('orange-token', token);
    window.coinPlays = [];
    const createSource = AudioContext.prototype.createBufferSource;
    AudioContext.prototype.createBufferSource = function () {
      const source = createSource.call(this), context = this;
      const start = source.start;
      source.start = function (...args) {
        const samples = this.buffer.getChannelData(0);
        let sum = 0, count = 0;
        for (let i = 0; i < samples.length; i += 32) { sum += samples[i] ** 2; count++; }
        window.coinPlays.push({ duration: this.buffer.duration, channels: this.buffer.numberOfChannels, state: context.state, rms: Math.sqrt(sum / count) });
        return start.apply(this, args);
      };
      return source;
    };
  }, auth.token);
  await page.goto(base);
  await page.locator('[data-event-kind="red_packet"]').first().waitFor();
  assert.equal(await page.evaluate(() => coinPlays.length), 0, 'initial balance/load stays silent');
  await page.locator('.hall-tools button').filter({ hasText: '每日签到' }).click();
  await page.locator('.stage-won').waitFor();
  assert.equal(await page.evaluate(() => coinPlays.length), 1, 'check-in plays once');
  await page.getByRole('button', { name: '开心收下', exact: true }).click();
  await page.locator('.hall-tools button').filter({ hasText: '今日已签到' }).click();
  await page.locator('.app-toast').filter({ hasText: '今天已经签到啦' }).waitFor();
  assert.equal(await page.evaluate(() => coinPlays.length), 1, 'failed duplicate check-in stays silent');
  await page.locator('.chat-packet').first().click();
  await page.getByRole('button', { name: '看视频领金币', exact: true }).click();
  await page.locator('.stage-won').waitFor({ timeout: 12000 });
  const plays = await page.evaluate(() => coinPlays);
  assert.equal(plays.length, 2, 'delayed video reward plays once');
  for (const play of plays) {
    assert.ok(Math.abs(play.duration - 1.85) < .01);
    assert.equal(play.channels, 2);
    assert.equal(play.state, 'running');
    assert.ok(play.rms > .01, 'decoded sound contains audible signal');
  }
  await page.getByRole('button', { name: '开心收下', exact: true }).click();
  assert.equal(await page.evaluate(() => coinPlays.length), 2, 'closing reward does not replay');
  assert.deepEqual(errors, []);
  const result = { passed: true, checkIn: 'once', duplicateCheckIn: 'silent', videoReward: 'once after 5 seconds', closingReward: 'silent', plays, runtimeErrors: errors };
  fs.writeFileSync(path.resolve(__dirname, '../output/coin-sound-verification.json'), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result));
  await browser.close();
})().catch(async error => { console.error(error); if (browser) await browser.close(); process.exitCode = 1; });
