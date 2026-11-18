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
  <section class="admin-card sms-admin">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="sms-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <p v-if="!cfg && !error" class="sms-muted">正在读取短信配置…</p>
    <template v-if="cfg">
      <header class="sms-head">
        <div>
                    <p>
            使用腾讯云短信（SendSms 2021-01-11）发送注册、登录、找回密码验证码。开启后 App 无需更新，
            获取验证码时由服务器直接下发；关闭时验证码不会发出。
          </p>
        </div>
        <div class="sms-state" :class="{ on: cfg.active }">
          <b>{{ cfg.active ? "已开启" : cfg.ready ? "已配置，未开启" : "未配置" }}</b>
          <small>今日 已发送 {{ cfg.today.sent }} · 失败 {{ cfg.today.failed }} · 未发出 {{ cfg.today.unsent }}</small>
        </div>
      </header>

      <form @submit.prevent="save">
        <label class="sms-switch">
          <input v-model="cfg.enabled" type="checkbox" />
          <span><b>开启短信通道</b><small>需要先填写下面带 * 的项目；建议先用“发送测试短信”确认能收到。</small></span>
        </label>

        <h3>腾讯云账号</h3>
        <div class="sms-grid">
          <label>SecretId *<input v-model="cfg.secret_id" maxlength="64" autocomplete="off" placeholder="访问管理 → API 密钥" /></label>
          <label>
            SecretKey *
            <input v-model="secretKey" type="password" maxlength="64" autocomplete="new-password" :placeholder="cfg.secret_key_set ? '已设置，留空保持不变' : '与 SecretId 配对的密钥'" />
          </label>
          <label>
            地域
            <select v-model="cfg.region"><option v-for="r in cfg.regions" :key="r" :value="r">{{ REGION_LABELS[r] || r }}</option></select>
          </label>
          <label>SdkAppId *<input v-model="cfg.sdk_app_id" maxlength="12" inputmode="numeric" placeholder="如 1400006666" /></label>
        </div>

        <h3>签名与模板</h3>
        <div class="sms-grid">
          <label>短信签名 *<input v-model="cfg.sign_name" maxlength="12" placeholder="已审核通过的签名内容，如：小橙有喜" /></label>
          <label>默认模板 ID *<input v-model="cfg.template_id" maxlength="12" inputmode="numeric" placeholder="验证码模板" /></label>
          <label>注册模板 ID<input v-model="cfg.register_template_id" maxlength="12" inputmode="numeric" placeholder="不填用默认模板" /></label>
          <label>登录模板 ID<input v-model="cfg.login_template_id" maxlength="12" inputmode="numeric" placeholder="不填用默认模板" /></label>
          <label>找回密码模板 ID<input v-model="cfg.reset_template_id" maxlength="12" inputmode="numeric" placeholder="不填用默认模板" /></label>
          <label>
            模板参数
            <input v-model="cfg.template_params" maxlength="80" placeholder="{code}" />
            <small>按模板变量顺序用英文逗号分隔：{code}=验证码，{minutes}=有效分钟。例如模板“验证码{1}，{2}分钟内有效”填 <code>{code},{minutes}</code></small>
          </label>
        </div>

        <h3>有效期与防刷</h3>
        <div class="sms-grid four">
          <label>有效期（分钟）<input v-model.number="cfg.expire_minutes" type="number" min="1" max="30" /></label>
          <label>同号码间隔（秒）<input v-model.number="cfg.cooldown_seconds" type="number" min="30" max="600" /></label>
          <label>同号码每日上限<input v-model.number="cfg.phone_daily_limit" type="number" min="0" max="100" /></label>
          <label>同 IP 每小时上限<input v-model.number="cfg.ip_hourly_limit" type="number" min="0" max="1000" /></label>
        </div>
        <p class="sms-muted">上限填 0 表示不限。请同时在腾讯云短信控制台设置频率限制，双重保护短信费用。</p>
        <button class="primary" :disabled="saving"><Icon name="check" :size="17" />保存短信配置</button>
      </form>

      <div class="sms-test">
        <h3>发送测试短信</h3>
        <p class="sms-muted">使用已保存的配置向指定手机号发一条随机验证码（未开启通道也可以测试）。先保存再测试。</p>
        <form class="sms-test-row" @submit.prevent="test">
          <input v-model="testPhone" maxlength="11" inputmode="numeric" placeholder="11 位手机号" aria-label="测试手机号" />
          <button class="outline" :disabled="testing || testPhone.trim().length !== 11"><Icon name="send" :size="14" />{{ testing ? "发送中…" : "发送测试" }}</button>
        </form>
        <p v-if="testResult" :class="testResult.ok ? 'sms-ok' : 'sms-bad'">
          {{ testResult.ok ? "发送成功，请查收短信" : "发送失败" }}<template v-if="!testResult.ok">：{{ testResult.code }} {{ testResult.message }}</template>
        </p>
      </div>
    </template>

    <div v-if="logs" class="sms-logs">
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
              <td>{{ STATUS_LABELS[row.status] || row.status }}</td><td class="msg">{{ [row.code, row.message].filter(Boolean).join(" ") || "—" }}</td>
              <td>{{ row.client_ip }}</td><td class="mono">{{ row.request_id || "—" }}</td>
            </tr>
          </tbody>
        </table>
        <p v-if="!logs.items.length" class="sms-muted">暂无记录。</p>
      </div>
      <div class="sms-pager">
        <button type="button" class="outline" :disabled="logs.page <= 1" @click="loadLogs(logs.page - 1).catch(fail)">上一页</button>
        <span>第 {{ logs.page }} / {{ Math.max(1, Math.ceil(logs.total / logs.page_size)) }} 页</span>
        <button type="button" class="outline" :disabled="logs.page * logs.page_size >= logs.total" @click="loadLogs(logs.page + 1).catch(fail)">下一页</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.sms-admin h2 { margin: 0 0 6px; }
.sms-admin h3 { margin: 20px 0 10px; font-size: 15px; }
.sms-head { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; flex-wrap: wrap; }
.sms-head p, .sms-muted { color: #9a8274; font-size: 13px; line-height: 1.7; margin: 0; max-width: 680px; }
.sms-state { border-radius: 12px; padding: 10px 14px; background: #f7efe8; display: grid; gap: 2px; }
.sms-state.on { background: #eefaf1; color: #2f8a4c; }
.sms-state small { font-size: 12px; color: #9a8274; }
.sms-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 10px; padding: 10px 12px; margin-bottom: 12px; font-size: 13px; }
.sms-switch { display: flex; gap: 10px; align-items: flex-start; margin-top: 16px; cursor: pointer; }
.sms-switch input { margin-top: 3px; }
.sms-switch span { display: grid; gap: 2px; }
.sms-switch small { color: #9a8274; font-size: 12px; }
.sms-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 10px; }
.sms-grid label { display: grid; gap: 4px; font-size: 12px; color: #8a6f61; }
.sms-grid small { color: #a8907f; line-height: 1.6; }
.sms-grid.four { grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); }
.sms-admin form > .primary { margin-top: 14px; }
.sms-test { margin-top: 18px; border-top: 1px dashed #f1e3d8; padding-top: 4px; }
.sms-test-row { display: flex; gap: 8px; margin-top: 10px; flex-wrap: wrap; }
.sms-test-row input { width: 180px; }
.sms-ok { color: #2f8a4c; font-size: 13px; }
.sms-bad { color: #c0392b; font-size: 13px; }
.sms-logs { margin-top: 18px; }
.sms-logs header { display: flex; justify-content: space-between; align-items: center; }
.sms-table-wrap { overflow-x: auto; }
.sms-logs table { width: 100%; border-collapse: collapse; font-size: 12.5px; white-space: nowrap; }
.sms-logs th { text-align: left; color: #9a8274; font-weight: 500; padding: 8px 10px; border-bottom: 1px solid #f1e3d8; }
.sms-logs td { padding: 8px 10px; border-bottom: 1px solid #f8f0ea; }
.sms-logs td.msg { white-space: normal; min-width: 160px; }
.sms-logs td.mono { font-family: ui-monospace, monospace; font-size: 11px; color: #9a8274; }
.sms-logs tr.failed td { background: #fff6f3; }
.sms-pager { display: flex; gap: 10px; align-items: center; justify-content: flex-end; margin-top: 10px; font-size: 13px; color: #8a6f61; }
.sms-admin label { margin-bottom: 0; }
.sms-admin input[type="checkbox"] { width: auto; flex: none; margin: 3px 0 0; padding: 0; accent-color: #f65331; }
.sms-grid input, .sms-grid select { margin-top: 0; padding: 9px 10px; font-size: 13px; }
.sms-test-row input { padding: 9px 10px; }
.sms-logs select { width: auto; padding: 7px 10px; }
</style>
