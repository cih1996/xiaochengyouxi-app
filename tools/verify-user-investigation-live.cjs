const {chromium}=require('./node_modules/playwright');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),base='https://orange.lingzidev.com';
const env=Object.fromEntries(fs.readFileSync(path.join(root,'private/server.env'),'utf8').trim().split(/\r?\n/).filter(s=>s&&!s.startsWith('#')).map(s=>{const i=s.indexOf('=');return [s.slice(0,i),s.slice(i+1)]}));
const sample=JSON.parse(fs.readFileSync(path.join(root,'private/risk-test-users.json'),'utf8'));
(async()=>{
 const browser=await chromium.launch({executablePath:'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true});
 try {
  const res=await fetch(`${base}/api/admin/users/${sample.user_id}/records?section=risk&page_size=100`,{headers:{'x-admin-token':env.ORANGE_ADMIN_TOKEN}});
  assert.ok(res.ok); const data=await res.json(),row=data.items.filter(r=>r.status==='reported').sort((a,b)=>b.score-a.score)[0];assert.ok(row.score>=30);
  const page=await browser.newPage({viewport:{width:1512,height:1120}}),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(token=>sessionStorage.setItem('orange-admin-token',token),env.ORANGE_ADMIN_TOKEN);
  await page.goto(base+'/admin');
  await page.getByRole('button',{name:'用户管理',exact:true}).click();
  await page.getByLabel('搜索用户',{exact:true}).fill(sample.phone);
  await page.getByRole('button',{name:'查询',exact:true}).click();
  await page.getByRole('button',{name:'查看用户 '+sample.phone,exact:true}).click();
  await page.locator('.ud-name h1').waitFor();
  await page.screenshot({path:path.join(root,'output/admin-user-live.png'),fullPage:true});
  await page.locator('.ud-tabs').getByRole('button',{name:'风险证据',exact:true}).click();
  await page.getByLabel('关联编号',{exact:true}).fill(row.report_id);
  await page.locator('.ud-filter-actions').getByRole('button',{name:'筛选',exact:true}).click();
  await page.locator('.ud-table-scroll tbody tr').first().getByRole('button',{name:'查看证据',exact:true}).click();
  await page.locator('.ud-record-detail').waitFor();
  assert.ok((await page.locator('.ud-proof-summary').textContent()).includes('持钥签名'));
  assert.ok(await page.locator('.ud-rule-hit').count()>0);
  await page.screenshot({path:path.join(root,'output/admin-user-risk-live.png'),fullPage:true});
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({passed:true,live:true,adminDetail:true,actualRuleHits:row.detail.hits.length,score:row.score}));
 } finally {await browser.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
