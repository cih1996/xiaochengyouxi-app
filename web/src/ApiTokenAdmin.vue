<script setup>
import { ref, computed, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const data = ref(null), error = ref(""), name = ref(""), busy = ref(false), created = ref(null), copied = ref("");
const controller = new AbortController();
onUnmounted(() => controller.abort());
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try { data.value = await props.api("/admin/api-tokens", { signal: controller.signal }); } catch (e) { fail(e); }
}
async function create() {
  if (!name.value.trim()) { error.value = "请填写令牌名称，例如“测试分发”"; return; }
  busy.value = true; error.value = "";
  try {
    const result = await props.api("/admin/api-tokens", { method: "POST", body: { name: name.value.trim() }, signal: controller.signal });
    created.value = result.created_token; data.value = result; name.value = "";
  } catch (e) { fail(e); } finally { busy.value = false; }
}
async function revoke(t) {
  if (!confirm(`停用令牌“${t.name}”后，使用它的脚本将无法再创建账号，且不能恢复。确定停用？`)) return;
  try { data.value = await props.api(`/admin/api-tokens/${t.id}/revoke`, { method: "POST", signal: controller.signal }); } catch (e) { fail(e); }
}
async function copy(key, text) {
  try { await navigator.clipboard.writeText(text); copied.value = key; setTimeout(() => (copied.value = ""), 1500); }
  catch { error.value = "浏览器不允许自动复制，请手动选中复制"; }
}
const example = computed(() => {
  const token = created.value?.token || "<你的接口令牌>";
  const url = data.value?.endpoint || "/api/integration/test-accounts";
  return `curl -X POST ${url} \\\n  -H "Authorization: Bearer ${token}" \\\n  -H "Content-Type: application/json" \\\n  -d '{"phone":"13800000000","password":"Test12345"}'`;
});
const fmt = (s) => (s ? new Date(s).toLocaleString("zh-CN", { hour12: false }) : "—");
defineExpose({ refresh });
refresh();
</script>
<template>
  <div v-if="data" class="tk-wrap">
    <section class="admin-card tk-card">
      <p class="tk-desc">
        用于测试分发：调用接口传入<strong>手机号和登录密码</strong>，即可创建可直接登录的账号。账号自动绑定平台邀请码
        <code>{{ data.platform_code }}</code>，并设为<strong>人工放行（白名单）</strong>：风控命中时自动放行、不会被限制，看广告不受风控拦截；风险记录照常保留，提现仍需后台审核。
        手机号已注册时只加入白名单，<strong>不会修改密码</strong>。每个令牌每天最多创建 {{ data.daily_limit }} 个账号。
      </p>
      <div v-if="error" class="form-error" role="alert">{{ error }}</div>
      <form class="tk-new" @submit.prevent="create">
        <input v-model="name" maxlength="30" placeholder="令牌名称，例如：测试分发 / 渠道A" aria-label="令牌名称" />
        <button class="primary" :disabled="busy"><Icon name="check" :size="16" />{{ busy ? "生成中…" : "生成新令牌" }}</button>
      </form>
      <div v-if="created" class="tk-created">
        <div class="tk-created-head"><Icon name="shield" :size="16" />令牌“{{ created.name }}”已生成。<strong>只显示这一次</strong>，关闭后无法再查看，请立即复制保存。</div>
        <div class="tk-token"><code>{{ created.token }}</code>
          <button type="button" class="tk-copy" @click="copy('token', created.token)">{{ copied === "token" ? "已复制" : "复制" }}</button></div>
        <button type="button" class="tk-close" @click="created = null">我已保存，关闭</button>
      </div>
    </section>

    <section class="admin-card tk-card">
      <div class="tk-head"><h3>调用方式</h3><span class="tk-links"><a class="tk-copy" href="/admin?doc=test-accounts">完整接口文档</a><button type="button" class="tk-copy" @click="copy('curl', example)">{{ copied === "curl" ? "已复制" : "复制示例" }}</button></span></div>
      <pre class="tk-pre">{{ example }}</pre>
      <ul class="tk-notes">
        <li>请求头也可以用 <code>X-Api-Token: &lt;令牌&gt;</code>。</li>
        <li><code>phone</code>：11 位手机号；<code>password</code>：至少 8 位；可选 <code>nickname</code>（默认“测试”+手机尾号）。</li>
        <li>成功返回 <code>{"user_id", "created", "whitelisted": true, "inviter_code", "invite_code", "message"}</code>；<code>created=false</code> 表示手机号已存在，仅加入白名单。</li>
        <li>失败：401 令牌无效或已停用；400 参数错误；409 账号已被后台人工限制；429 超出当日上限。</li>
      </ul>
    </section>

    <section class="admin-card tk-card">
      <h3>令牌列表</h3>
      <table v-if="data.tokens.length" class="tk-table">
        <thead><tr><th>名称</th><th>令牌开头</th><th>创建</th><th>最近使用</th><th>调用次数</th><th>新建账号</th><th>状态</th><th></th></tr></thead>
        <tbody>
          <tr v-for="t in data.tokens" :key="t.id" :class="{ off: t.revoked_at }">
            <td>{{ t.name }}</td><td><code>{{ t.prefix }}…</code></td>
            <td>{{ fmt(t.created_at) }}<small>{{ t.created_by }}</small></td>
            <td>{{ fmt(t.last_used_at) }}</td><td>{{ t.use_count }}</td><td>{{ t.accounts }}</td>
            <td><span :class="['tk-badge', t.revoked_at ? 'off' : 'on']">{{ t.revoked_at ? "已停用" : "使用中" }}</span></td>
            <td><button v-if="!t.revoked_at" type="button" class="tk-revoke" @click="revoke(t)">停用</button></td>
          </tr>
        </tbody>
      </table>
      <p v-else class="tk-empty">还没有令牌，在上方生成一个。</p>
    </section>

    <section v-if="data.recent_accounts.length" class="admin-card tk-card">
      <h3>最近通过接口处理的账号</h3>
      <table class="tk-table">
        <thead><tr><th>时间</th><th>用户 ID</th><th>手机号</th><th>昵称</th><th>结果</th><th>令牌</th></tr></thead>
        <tbody>
          <tr v-for="a in data.recent_accounts" :key="a.at + a.user_id">
            <td>{{ fmt(a.at) }}</td><td>{{ a.user_id }}</td><td>{{ a.phone }}</td><td>{{ a.nickname }}</td>
            <td>{{ a.created ? "新建 + 白名单" : "已存在，加白名单" }}</td><td>{{ a.token }}</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
  <p v-else-if="error" class="form-error">{{ error }}</p>
</template>
<style scoped>
.tk-wrap { display: grid; gap: 14px; }
.tk-card { padding: 18px 20px; }
.tk-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.tk-card h3 { font-size: 15px; color: #4f3627; margin: 0 0 10px; }
.tk-desc { color: #9a8274; font-size: 13px; line-height: 1.8; margin: 0 0 12px; max-width: 900px; }
.tk-desc strong { color: #6f4b33; }
code { background: #fbf3e8; border-radius: 6px; padding: 1px 6px; font-size: 12.5px; color: #8a4f26; }
.tk-new { display: flex; gap: 10px; max-width: 560px; }
.tk-new input { flex: 1; margin: 0; padding: 9px 11px; font-size: 13.5px; border-radius: 10px; }
.tk-new .primary { width: auto; margin: 0; padding: 9px 18px; display: inline-flex; gap: 6px; align-items: center; white-space: nowrap; }
.tk-created { margin-top: 14px; border: 1.5px solid #f2c48f; background: #fff8ee; border-radius: 14px; padding: 12px 14px; max-width: 760px; }
.tk-created-head { display: flex; gap: 6px; align-items: center; font-size: 13.5px; color: #8a4f26; }
.tk-token { display: flex; gap: 10px; align-items: center; margin-top: 10px; }
.tk-token code { flex: 1; font-size: 14px; padding: 8px 10px; word-break: break-all; background: #fff; border: 1px solid #f0dcc4; }
.tk-copy, .tk-close, .tk-revoke { border: 1px solid #ecdccd; background: #fff; color: #b0623f; border-radius: 8px; padding: 5px 12px; font-size: 13px; cursor: pointer; white-space: nowrap; }
.tk-copy:hover, .tk-close:hover { border-color: #ef916b; }
.tk-close { margin-top: 10px; }
.tk-revoke { color: #c0392b; }
.tk-revoke:hover { border-color: #e57373; }
.tk-links { display: inline-flex; gap: 8px; }
a.tk-copy { text-decoration: none; }
.tk-head { display: flex; justify-content: space-between; align-items: center; }
.tk-pre { background: #3a2a20; color: #ffe9d2; border-radius: 12px; padding: 12px 14px; font-size: 12.5px; overflow-x: auto; margin: 0; }
.tk-notes { margin: 10px 0 0; padding-left: 18px; color: #9a8274; font-size: 12.5px; line-height: 1.9; }
.tk-table { width: 100%; border-collapse: collapse; font-size: 13px; }
.tk-table th { text-align: left; color: #a8907f; font-weight: 600; padding: 7px 8px; border-bottom: 1px solid #f2e8d9; white-space: nowrap; }
.tk-table td { padding: 8px; border-bottom: 1px solid #f7efe4; color: #5a3a24; vertical-align: top; }
.tk-table td small { display: block; color: #b9a28c; font-size: 11.5px; }
.tk-table tr.off td { color: #bfae9c; }
.tk-badge { border-radius: 10px; padding: 2px 8px; font-size: 12px; font-weight: 700; }
.tk-badge.on { background: #eefaf1; color: #2f8a4c; }
.tk-badge.off { background: #f4efe9; color: #a8907a; }
.tk-empty { color: #a8907f; font-size: 13px; margin: 0; }
@media (max-width: 900px) { .tk-new { flex-direction: column; } .tk-table { display: block; overflow-x: auto; } }
</style>
