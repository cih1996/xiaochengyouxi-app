<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const cfg = ref(null),
  secretKey = ref(""),
  logs = ref(null),
  status = ref(""),
  testPhone = ref(""),
  testResult = ref(null),
  testing = ref(false),
  error = ref(""),
  notice = ref(""),
  saving = ref(false);
const controller = new AbortController();
const REGION_LABELS = { "ap-guangzhou": "华南地区（广州）", "ap-beijing": "华北地区（北京）", "ap-nanjing": "华东地区（南京）" };
const STATUS_LABELS = { sent: "已发送", failed: "发送失败", unsent: "未开启通道" };
const PURPOSE_LABELS = { register: "注册", login: "登录", reset: "找回密码", test: "后台测试" };
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function loadLogs(page = 1) {
  const q = new URLSearchParams({ page });
  if (status.value) q.set("status", status.value);
  logs.value = await props.api(`/admin/sms-logs?${q}`, { signal: controller.signal });
}
async function refresh() {
  error.value = "";
  try {
    [cfg.value] = await Promise.all([
      props.api("/admin/sms-config", { signal: controller.signal }),
      loadLogs(1),
    ]);
    secretKey.value = "";
  } catch (e) {
    fail(e);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    const { secret_key_set, ready, active, revision, updated_at, regions, today, ...config } = cfg.value;
    cfg.value = await props.api("/admin/sms-config", {
      method: "PUT",
      body: { ...config, secret_key: secretKey.value },
      signal: controller.signal,
    });
    secretKey.value = "";
    notice.value = cfg.value.active
      ? "已保存，短信通道已开启：App 获取验证码将真实下发短信"
      : "已保存，短信通道未开启：验证码不会真实下发";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
async function test() {
  testing.value = true;
  testResult.value = null;
  error.value = "";
  try {
    testResult.value = await props.api("/admin/sms-config/test", {
      method: "POST",
      body: { phone: testPhone.value.trim() },
      signal: controller.signal,
    });
    await loadLogs(1);
  } catch (e) {
    fail(e);
  } finally {
    testing.value = false;
  }
}
const time = (iso) => new Date(iso).toLocaleString("zh-CN", { hour12: false });
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <div class="sms-page">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="sms-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <section v-if="!cfg && !error" class="admin-card sms-card"><p class="sms-muted">正在读取短信配置…</p></section>

    <form v-if="cfg" @submit.prevent="save">
      <section class="admin-card sms-card">
        <header class="sms-card-head">
          <div>
            <h2>腾讯云短信</h2>
            <p>发送注册、登录、找回密码验证码（SendSms 2021-01-11）。开启后 App 无需更新，获取验证码时由服务器直接下发；关闭时验证码不会发出。</p>
          </div>
          <div class="sms-stats">
            <span :class="cfg.active ? 'ok' : cfg.ready ? 'warn' : ''">{{ cfg.active ? "已开启" : cfg.ready ? "已配置，未开启" : "未配置" }}</span>
            <span>今日已发送 <b>{{ cfg.today.sent }}</b></span>
            <span>失败 <b>{{ cfg.today.failed }}</b></span>
            <span>未发出 <b>{{ cfg.today.unsent }}</b></span>
          </div>
        </header>
        <div class="sms-toggle-row">
          <div>
            <b>开启短信通道</b>
            <p>先填写下方带 * 的项目并保存，用“发送测试短信”确认能收到后再开启。</p>
          </div>
          <label class="sms-switch"><input v-model="cfg.enabled" type="checkbox" aria-label="开启短信通道" /><span></span></label>
        </div>
      </section>

      <div class="sms-grid2">
        <section class="admin-card sms-card">
          <h3>腾讯云账号</h3>
          <div class="sms-fields">
            <label>SecretId *<input v-model="cfg.secret_id" maxlength="64" autocomplete="off" placeholder="访问管理 → API 密钥" /></label>
            <label>
              SecretKey *
              <input v-model="secretKey" type="password" maxlength="64" autocomplete="new-password" :placeholder="cfg.secret_key_set ? '已设置，留空保持不变' : '与 SecretId 配对的密钥'" />
            </label>
            <label>SdkAppId *<input v-model="cfg.sdk_app_id" maxlength="12" inputmode="numeric" placeholder="如 1400006666" /></label>
            <label>
              地域
              <select v-model="cfg.region"><option v-for="r in cfg.regions" :key="r" :value="r">{{ REGION_LABELS[r] || r }}</option></select>
            </label>
          </div>
        </section>
        <section class="admin-card sms-card">
          <h3>签名与模板</h3>
          <div class="sms-fields">
            <label>短信签名 *<input v-model="cfg.sign_name" maxlength="12" placeholder="已审核通过的签名，如：小橙有喜" /></label>
            <label>默认模板 ID *<input v-model="cfg.template_id" maxlength="12" inputmode="numeric" placeholder="验证码模板" /></label>
            <label>注册模板 ID<input v-model="cfg.register_template_id" maxlength="12" inputmode="numeric" placeholder="不填用默认模板" /></label>
            <label>登录模板 ID<input v-model="cfg.login_template_id" maxlength="12" inputmode="numeric" placeholder="不填用默认模板" /></label>
            <label>找回密码模板 ID<input v-model="cfg.reset_template_id" maxlength="12" inputmode="numeric" placeholder="不填用默认模板" /></label>
            <label>模板参数<input v-model="cfg.template_params" maxlength="80" placeholder="{code}" /></label>
          </div>
          <p class="sms-hint">模板参数按模板变量顺序用英文逗号分隔：<code>{code}</code> 为验证码，<code>{minutes}</code> 为有效分钟。例如模板“验证码{1}，{2}分钟内有效”填 <code>{code},{minutes}</code>。</p>
        </section>
      </div>

      <div class="sms-grid2">
        <section class="admin-card sms-card">
          <h3>有效期与防刷</h3>
          <div class="sms-fields">
            <label>有效期（分钟）<input v-model.number="cfg.expire_minutes" type="number" min="1" max="30" /></label>
            <label>同号码间隔（秒）<input v-model.number="cfg.cooldown_seconds" type="number" min="30" max="600" /></label>
            <label>同号码每日上限<input v-model.number="cfg.phone_daily_limit" type="number" min="0" max="100" /></label>
            <label>同 IP 每小时上限<input v-model.number="cfg.ip_hourly_limit" type="number" min="0" max="1000" /></label>
          </div>
          <p class="sms-hint">上限填 0 表示不限。建议同时在腾讯云短信控制台设置频率限制，双重保护短信费用。</p>
        </section>
        <section class="admin-card sms-card">
          <h3>发送测试短信</h3>
          <p class="sms-hint top">用已保存的配置向指定手机号发一条随机验证码，未开启通道也可以测试。修改配置后请先保存再测试。</p>
          <div class="sms-test-row">
            <input v-model="testPhone" maxlength="11" inputmode="numeric" placeholder="11 位手机号" aria-label="测试手机号" />
            <button type="button" class="outline" :disabled="testing || testPhone.trim().length !== 11" @click="test">
              <Icon name="send" :size="14" />{{ testing ? "发送中…" : "发送测试" }}
            </button>
          </div>
          <p v-if="testResult" class="sms-result" :class="testResult.ok ? 'ok' : 'bad'">
            {{ testResult.ok ? "发送成功，请查收短信" : "发送失败" }}<template v-if="!testResult.ok">：{{ testResult.code }} {{ testResult.message }}</template>
          </p>
        </section>
      </div>

      <div class="sms-savebar">
        <span>保存后立即生效</span>
        <button class="primary" :disabled="saving"><Icon name="check" :size="17" />{{ saving ? "保存中…" : "保存短信配置" }}</button>
      </div>
    </form>

    <section v-if="logs" class="admin-card sms-card sms-logs">
      <header>
        <h3>发送记录</h3>
        <select v-model="status" aria-label="状态" @change="loadLogs(1).catch(fail)">
          <option value="">全部状态</option><option value="sent">已发送</option><option value="failed">发送失败</option><option value="unsent">未开启通道</option>
        </select>
      </header>
      <div class="sms-table-wrap">
        <table>
          <thead><tr><th>时间</th><th>手机号</th><th>用途</th><th>状态</th><th>平台返回</th><th>IP</th><th>RequestId</th></tr></thead>
          <tbody>
            <tr v-for="(row, i) in logs.items" :key="i" :class="row.status">
              <td>{{ time(row.created_at) }}</td><td>{{ row.phone }}</td><td>{{ PURPOSE_LABELS[row.purpose] || row.purpose }}</td>
              <td><span class="sms-status" :class="row.status">{{ STATUS_LABELS[row.status] || row.status }}</span></td>
              <td class="msg">{{ [row.code, row.message].filter(Boolean).join(" ") || "—" }}</td>
              <td>{{ row.client_ip }}</td><td class="mono">{{ row.request_id || "—" }}</td>
            </tr>
          </tbody>
        </table>
        <p v-if="!logs.items.length" class="sms-muted empty">暂无记录。</p>
      </div>
      <div class="sms-pager">
        <button type="button" class="outline" :disabled="logs.page <= 1" @click="loadLogs(logs.page - 1).catch(fail)">上一页</button>
        <span>第 {{ logs.page }} / {{ Math.max(1, Math.ceil(logs.total / logs.page_size)) }} 页</span>
        <button type="button" class="outline" :disabled="logs.page * logs.page_size >= logs.total" @click="loadLogs(logs.page + 1).catch(fail)">下一页</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.sms-card { padding: 18px 20px; margin-bottom: 12px; }
.sms-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.sms-card h3 { font-size: 15px; color: #4f3627; margin: 0 0 16px; }
.sms-card-head { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 8px; }
.sms-card-head p, .sms-muted { color: #9a8274; font-size: 14px; line-height: 1.75; margin: 0; max-width: 720px; }
.sms-stats { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
.sms-stats span { font-size: 13.5px; color: #8a6f61; background: #f7efe8; border-radius: 999px; padding: 6px 12px; white-space: nowrap; }
.sms-stats span.ok { background: #eefaf1; color: #2f8a4c; font-weight: 600; }
.sms-stats span.warn { background: #fff4e5; color: #a5520b; font-weight: 600; }
.sms-stats b { color: #4f3627; }
.sms-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 12px 14px; margin-bottom: 14px; font-size: 14px; }

.sms-toggle-row { display: flex; justify-content: space-between; align-items: center; gap: 24px; border-top: 1px solid #f2e8d9; padding: 18px 0 4px; margin-top: 10px; }
.sms-toggle-row b { font-size: 14px; color: #6f4b33; }
.sms-toggle-row p { font-size: 14px; color: #a8907f; margin: 6px 0 0; line-height: 1.65; }
.sms-switch { position: relative; display: block; width: 42px; height: 24px; flex-shrink: 0; margin: 0; }
.sms-switch input { opacity: 0; width: 100%; height: 100%; position: absolute; inset: 0; margin: 0; padding: 0; cursor: pointer; }
.sms-switch > span { pointer-events: none; display: block; height: 24px; border-radius: 30px; background: #dfd4c3; transition: background 0.2s; }
.sms-switch > span::after { content: ""; position: absolute; height: 18px; width: 18px; top: 3px; left: 3px; background: #fff; border-radius: 50%; transition: transform 0.2s; box-shadow: 0 1px 3px #98714824; }
.sms-switch input:checked + span { background: #ec985b; }
.sms-switch input:checked + span::after { transform: translateX(18px); }
.sms-switch input:focus-visible + span { outline: 2px solid #d18448; outline-offset: 3px; }

.sms-grid2 { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0 12px; }
.sms-grid2 > .sms-card { display: flex; flex-direction: column; }
.sms-fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px 14px; }
.sms-fields label { margin: 0; display: grid; gap: 6px; font-size: 13.5px; }
.sms-fields input, .sms-fields select { margin: 0; padding: 10px 12px; font-size: 14px; border-radius: 10px; }
.sms-hint { font-size: 13.5px; color: #a8907f; line-height: 1.7; margin: 14px 0 0; }
.sms-hint.top { margin: 0 0 14px; }
.sms-hint code { background: #fff2e6; padding: 1px 5px; border-radius: 5px; color: #8a4a2b; }
.sms-test-row { display: flex; gap: 10px; }
.sms-test-row input { flex: 1; max-width: 260px; margin: 0; padding: 10px 12px; font-size: 14px; border-radius: 10px; }
.sms-test-row .outline { display: inline-flex; gap: 6px; align-items: center; white-space: nowrap; }
.sms-result { font-size: 14px; margin: 12px 0 0; padding: 10px 12px; border-radius: 10px; line-height: 1.6; }
.sms-result.ok { color: #2f8a4c; background: #eefaf1; }
.sms-result.bad { color: #b03a2a; background: #fff3ef; }

.sms-savebar {
  position: sticky; bottom: 16px; z-index: 5; display: flex; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 18px;
  background: #fffdfaf2; backdrop-filter: blur(6px); border: 1px solid #eee4d7; border-radius: 16px; padding: 12px 14px 12px 22px;
  box-shadow: 0 8px 24px rgba(120, 80, 50, 0.08);
}
.sms-savebar span { font-size: 14px; color: #a8907f; }
.sms-savebar .primary { width: auto; margin: 0; padding: 11px 22px; }

.sms-logs header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.sms-logs header h3 { margin: 0; }
.sms-logs header select { width: auto; margin: 0; padding: 7px 12px; font-size: 14px; }
.sms-table-wrap { overflow-x: auto; }
.sms-logs table { width: 100%; border-collapse: collapse; font-size: 14px; white-space: nowrap; }
.sms-logs th { text-align: left; color: #9a8274; font-weight: 500; padding: 9px 12px; background: #fbf6f1; }
.sms-logs th:first-child { border-radius: 8px 0 0 8px; }
.sms-logs th:last-child { border-radius: 0 8px 8px 0; }
.sms-logs td { padding: 10px 12px; border-bottom: 1px solid #f6ede5; color: #5d4030; }
.sms-logs td.msg { white-space: normal; min-width: 200px; color: #7d685c; }
.sms-logs td.mono { font-family: ui-monospace, monospace; font-size: 13px; color: #a8907f; }
.sms-status { font-size: 13.5px; border-radius: 999px; padding: 2px 9px; background: #f7efe8; color: #8a6f61; }
.sms-status.sent { background: #eefaf1; color: #2f8a4c; }
.sms-status.failed { background: #fff3ef; color: #b03a2a; }
.sms-muted.empty { padding: 18px 0 4px; }
.sms-pager { display: flex; gap: 10px; align-items: center; justify-content: flex-end; margin-top: 14px; font-size: 14px; color: #8a6f61; }

@media (max-width: 1100px) {
  .sms-grid2 { grid-template-columns: 1fr; }
}
@media (max-width: 640px) {
  .sms-card { padding: 18px 16px; }
  .sms-card-head { flex-direction: column; }
  .sms-stats { justify-content: flex-start; }
  .sms-fields { grid-template-columns: 1fr; }
  .sms-savebar span { display: none; }
  .sms-savebar .primary { width: 100%; justify-content: center; }
}
</style>
