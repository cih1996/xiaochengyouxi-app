const { chromium } = require('playwright');
const fs = require('node:fs');
const path = require('node:path');
let browser;
(async () => {
  const root = path.resolve(__dirname, '..');
  browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  const page = await browser.newPage();
  await page.goto('https://orange.lingzidev.com/');
  const source = fs.readFileSync(path.join(root, 'web/src/invite-poster.js'), 'utf8').replaceAll('export ', '');
  await page.addScriptTag({ content: source + '\nwindow.renderInvitePoster = createInvitePoster;' });
  const qr = await page.evaluate(async () => {
    const response = await fetch('/api/invite/qr/JZ8F6A');
    if (!response.ok) throw new Error(await response.text());
    return response.json();
  });
  const dataUrl = await page.evaluate(async qr => {
    const blob = await window.renderInvitePoster({ invite_code: qr.code }, qr);
    return await new Promise(resolve => {
      const reader = new FileReader(); reader.onload = () => resolve(reader.result); reader.readAsDataURL(blob);
    });
  }, qr);
  const file = path.join(root, 'output/小橙有喜-邀请海报.png');
  fs.writeFileSync(file, Buffer.from(dataUrl.split(',')[1], 'base64'));
  console.log('Exported 1080x1600 platform invitation poster: ' + file);
  await browser.close();
})().catch(async error => { console.error(error); if (browser) await browser.close(); process.exitCode = 1; });
