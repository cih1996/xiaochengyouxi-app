<script setup>
import { ref } from "vue";
import Icon from "./Icon.vue";
const api = "https://orange-api.lingzidev.com/api/integration/test-accounts";
const copied = ref("");
async function copy(key, text) {
  try { await navigator.clipboard.writeText(text); copied.value = key; setTimeout(() => (copied.value = ""), 1500); } catch { /* manual copy */ }
}
const curl = `curl -X POST ${api} \\
  -H "Authorization: Bearer $ORANGE_API_TOKEN" \\
  -H "Content-Type: application/json" \\
  -d '{"phone":"13800000000","password":"Test12345","nickname":"测试小王"}'`;
const request = `{
  "phone": "13800000000",
  "password": "Test12345",
  "nickname": "测试小王"
}`;
const response = `{
  "user_id": 123,
  "phone": "13800000000",
  "created": true,
  "whitelisted": true,
  "inviter_code": "JZ8F6A",
  "invite_code": "G2CCNB",
  "message": "账号已创建并加入白名单"
}`;
const python = `# 批量创建：accounts.csv 每行 "手机号,密码[,昵称]"，令牌放在环境变量 ORANGE_API_TOKEN
import csv, json, os, urllib.request, urllib.error

API = "${api}"
TOKEN = os.environ["ORANGE_API_TOKEN"]

with open("accounts.csv", encoding="utf-8") as f:
    for row in csv.reader(f):
        if not row or row[0].startswith("#"):
            continue
        body = {"phone": row[0].strip(), "password": row[1].strip()}
        if len(row) > 2 and row[2].strip():
            body["nickname"] = row[2].strip()
        req = urllib.request.Request(API, data=json.dumps(body).encode(), method="POST", headers={
            "Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=20) as r:
                print(body["phone"], json.load(r)["message"])
        except urllib.error.HTTPError as e:
            print(body["phone"], e.code, json.load(e).get("error"))`;
</script>
<template>
  <div class="ta-page">
    <header class="ta-header">
      <a href="/" class="brand"><span class="brand-symbol">橙</span><b>小橙有喜</b></a>
      <a href="/admin?tab=api-tokens">返回接口令牌<Icon name="arrow" :size="14" /></a>
    </header>
    <main>
      <section class="ta-hero">
        <h1>测试账号接口</h1>
        <p>传入手机号和登录密码，一次调用得到一个可直接登录 App 的账号：自动绑定平台邀请码，并加入白名单（人工放行）。用于测试分发。</p>
      </section>

      <div class="ta-steps">
        <article><span>01</span><h2>生成令牌</h2><p>后台「系统设置 · 接口令牌」生成令牌，只显示一次，请立即保存。</p></article>
        <article><span>02</span><h2>调用接口</h2><p>请求头 <code>Authorization: Bearer 令牌</code>，提交手机号和密码。</p></article>
        <article><span>03</span><h2>直接登录</h2><p>测试人员用该手机号和密码登录 App，无需短信验证码、无需填邀请码。</p></article>
      </div>

      <section class="ta-section">
        <div class="ta-head"><h2>请求</h2><button @click="copy('curl', curl)"><Icon name="list" :size="14" />{{ copied === "curl" ? "已复制" : "复制 curl" }}</button></div>
        <div class="ta-endpoint"><b>POST</b><code>{{ api }}</code></div>
        <pre>{{ curl }}</pre>
        <table class="ta-table">
          <thead><tr><th>字段</th><th>必填</th><th>说明</th></tr></thead>
          <tbody>
            <tr><td><code>phone</code></td><td>是</td><td>11 位手机号</td></tr>
            <tr><td><code>password</code></td><td>是</td><td>登录密码，至少 8 位</td></tr>
            <tr><td><code>nickname</code></td><td>否</td><td>昵称，最多 20 字；默认“测试”+手机尾号 4 位</td></tr>
          </tbody>
        </table>
        <p class="ta-caption">令牌也可以放在 <code>X-Api-Token</code> 请求头。请求体为 JSON。</p>
        <pre class="ta-small">{{ request }}</pre>
      </section>

      <section class="ta-section">
        <div class="ta-head"><h2>响应</h2></div>
        <pre class="ta-small">{{ response }}</pre>
        <table class="ta-table">
          <thead><tr><th>字段</th><th>说明</th></tr></thead>
          <tbody>
            <tr><td><code>created</code></td><td><code>true</code> 新建了账号；<code>false</code> 手机号已注册，本次只加入白名单，<b>密码未修改</b></td></tr>
            <tr><td><code>whitelisted</code></td><td>始终为 <code>true</code></td></tr>
            <tr><td><code>inviter_code</code></td><td>绑定的上级邀请码（平台邀请码）</td></tr>
            <tr><td><code>invite_code</code></td><td>该账号自己的邀请码</td></tr>
          </tbody>
        </table>
      </section>

      <section class="ta-section">
        <div class="ta-head"><h2>错误</h2></div>
        <table class="ta-table">
          <thead><tr><th>状态码</th><th>含义</th></tr></thead>
          <tbody>
            <tr><td>400</td><td>手机号不是 11 位，或密码少于 8 位</td></tr>
            <tr><td>401</td><td>令牌无效或已停用</td></tr>
            <tr><td>409</td><td>该账号已被后台人工限制，接口不会放行，请在后台处理</td></tr>
            <tr><td>429</td><td>该令牌今天已新建 500 个账号</td></tr>
          </tbody>
        </table>
        <p class="ta-caption">错误响应格式：<code>{"error": "说明文字"}</code></p>
      </section>

      <section class="ta-section">
        <div class="ta-head"><h2>批量创建（Python 标准库）</h2><button @click="copy('py', python)"><Icon name="list" :size="14" />{{ copied === "py" ? "已复制" : "复制代码" }}</button></div>
        <pre>{{ python }}</pre>
      </section>

      <section class="ta-rules">
        <article><Icon name="shield" :size="22" /><div><h2>白名单是什么</h2><p>与后台“人工放行”同一机制：设备风控、行为审计命中时自动放行，不限制账号；看广告不受风控拦截。风险记录照常保留，提现仍需后台审核，奖励金额规则不变。</p></div></article>
        <article><Icon name="user" :size="22" /><div><h2>已注册的手机号</h2><p>只加入白名单，不改密码、不改邀请关系。后台人工限制过的账号会返回 409。</p></div></article>
        <article><Icon name="settings" :size="22" /><div><h2>令牌管理</h2><p>令牌只存摘要、只显示一次；可随时停用。后台可查看每个令牌的调用次数和最近处理的账号。</p></div></article>
      </section>
    </main>
  </div>
</template>
<style scoped>
.ta-page { min-height: 100vh; background: #fff8ef; color: #4f3627; }
.ta-header { display: flex; justify-content: space-between; align-items: center; max-width: 1040px; margin: 0 auto; padding: 20px 24px; }
.ta-header a { color: #b0623f; text-decoration: none; display: inline-flex; gap: 6px; align-items: center; font-size: 14px; }
.ta-header .brand { color: #4f3627; gap: 10px; font-size: 17px; }
.brand-symbol { width: 34px; height: 34px; border-radius: 11px; background: linear-gradient(135deg, #ffad6b, #f0650c); color: #fff; display: inline-flex; align-items: center; justify-content: center; font-weight: 900; }
main { max-width: 1040px; margin: 0 auto; padding: 0 24px 60px; }
.ta-hero h1 { font-size: 30px; margin: 18px 0 8px; }
.ta-hero p { color: #8a7060; font-size: 15px; line-height: 1.8; max-width: 760px; margin: 0; }
.ta-steps { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; margin: 26px 0; }
.ta-steps article { background: #fff; border-radius: 18px; padding: 18px; box-shadow: 0 2px 0 #f1e3cf; }
.ta-steps span { color: #f0650c; font-weight: 900; font-size: 13px; }
.ta-steps h2 { font-size: 16px; margin: 6px 0; }
.ta-steps p { color: #8a7060; font-size: 13.5px; line-height: 1.7; margin: 0; }
.ta-section { background: #fff; border-radius: 20px; padding: 20px 22px; margin-bottom: 16px; box-shadow: 0 2px 0 #f1e3cf; }
.ta-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.ta-head h2 { font-size: 18px; margin: 0; }
.ta-head button { display: inline-flex; gap: 5px; align-items: center; border: 1px solid #ecdccd; background: #fff; color: #b0623f; border-radius: 9px; padding: 6px 12px; font-size: 13px; cursor: pointer; }
.ta-endpoint { display: flex; gap: 10px; align-items: center; margin-bottom: 10px; font-size: 14px; flex-wrap: wrap; }
.ta-endpoint b { background: #eefaf1; color: #2f8a4c; border-radius: 7px; padding: 2px 8px; font-size: 12.5px; }
pre { background: #3a2a20; color: #ffe9d2; border-radius: 14px; padding: 14px 16px; font-size: 12.8px; line-height: 1.6; overflow-x: auto; margin: 0; }
.ta-small { margin-top: 10px; }
code { background: #fbf3e8; color: #8a4f26; border-radius: 6px; padding: 1px 6px; font-size: 12.8px; }
pre code { background: none; color: inherit; padding: 0; }
.ta-table { width: 100%; border-collapse: collapse; font-size: 13.5px; margin-top: 12px; }
.ta-table th { text-align: left; color: #a8907f; font-weight: 600; padding: 8px; border-bottom: 1px solid #f2e8d9; }
.ta-table td { padding: 9px 8px; border-bottom: 1px solid #f7efe4; line-height: 1.6; }
.ta-caption { color: #9a8274; font-size: 13px; margin: 12px 0 0; }
.ta-rules { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; }
.ta-rules article { display: flex; gap: 12px; background: #fff; border-radius: 18px; padding: 18px; box-shadow: 0 2px 0 #f1e3cf; color: #f0650c; }
.ta-rules h2 { font-size: 15px; margin: 0 0 6px; color: #4f3627; }
.ta-rules p { font-size: 13px; line-height: 1.7; color: #8a7060; margin: 0; }
@media (max-width: 760px) { .ta-steps, .ta-rules { grid-template-columns: 1fr; } .ta-hero h1 { font-size: 24px; } }
</style>
