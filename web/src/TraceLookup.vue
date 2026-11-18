<script setup>
import { ref, nextTick, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized", "open-user"]);
const query = ref(""),
  result = ref(null),
  open = ref(false),
  loading = ref(false),
  error = ref(""),
  liftReason = ref(""),
  lifting = ref(false),
  notice = ref(""),
  panel = ref(null);
let controller = new AbortController();

// DeviceTrust / behaviour reason codes → plain wording for support staff.
const REASONS = {
  adb_enabled: "开启了 USB 调试",
  developer_options: "开启了开发者选项",
  root_mounts: "检测到 Root 挂载痕迹",
  root_detected: "设备已 Root",
  su_binary: "存在 su 提权程序",
  boot_state_spoofed: "系统启动状态被篡改（解锁/伪装）",
  bootloader_unlocked: "Bootloader 已解锁",
  emulator: "疑似模拟器",
  hook_framework: "检测到 Hook 框架",
  xposed: "检测到 Xposed",
  frida: "检测到 Frida",
  virtual_app: "疑似多开/分身",
  accessibility_automation: "无障碍自动化服务",
  automation_tool: "检测到自动化脚本工具",
  mock_location: "虚拟定位",
  proxy_enabled: "设置了网络代理",
  vpn_active: "正在使用 VPN",
  network_interface_mismatch: "网卡信息异常",
  exposed_listening_ports: "本机开放调试端口",
  lan_adb_peers: "同网段有开放无线调试的设备",
  account_device_changes: "账号频繁更换设备",
  device_account_changes: "同一设备登录多个账号",
  sim_network_mismatch: "SIM 卡与网络运营商不一致",
  ip_registrations: "同一 IP 注册过多",
  attestation_failed: "硬件证明未通过",
  injected_touches: "疑似模拟点击",
};
const reasonLabel = (code) => REASONS[code] || code;
// "DeviceTrust verify: [\"adb_enabled\",…]" → "设备核验需复核：开启了 USB 调试、…"
function prettyReason(text) {
  if (!text) return "—";
  const m = text.match(/^(DeviceTrust|行为审计评估)\s*([a-z]*)\s*[:：]\s*(.*)$/s);
  if (!m) return text;
  let codes = [];
  try {
    codes = JSON.parse(m[3]);
  } catch {
    codes = m[3].split(/[、,，]/).map((v) => v.trim()).filter(Boolean);
  }
  const head = m[1] === "DeviceTrust" ? `设备核验${ACTION[m[2]] || ""}` : "行为审计";
  return `${head}：${(Array.isArray(codes) ? codes : [String(codes)]).map(reasonLabel).join("、")}`;
}
const reasonList = (risk) => (Array.isArray(risk?.reasons) ? risk.reasons : []);
const STATUS = { active: "正常", review: "复核中（限制领取）", restricted: "已限制" };
const DECISION = { allow: "通过", review: "复核", deny: "拒绝", reject: "拒绝", block: "拦截" };
const ACTION = { observe: "仅记录", verify: "需复核", hold: "限制", block: "拦截" };
const OPERATION = { login: "登录", reward: "领红包", check_in: "签到", withdraw: "提现", rain: "红包雨", behavior: "行为审计" };
const AD_EVENT = { plan: "服务器分配", risk_block: "风控拦截", empty: "无可用平台", request: "发起请求", fill: "填充", no_fill: "无填充", show: "展示", click: "点击", close: "关闭", reward: "客户端奖励", skip: "跳过", error: "出错", timeout: "超时", callback_ok: "回调通过", callback_retry: "回调重试", callback_reject: "回调拒绝" };
const NETWORK = { csj: "穿山甲", kuaishou: "快手", baidu: "百度", gdt: "优量汇", "-": "—" };
const time = (iso) => (iso ? new Date(iso).toLocaleString("zh-CN", { hour12: false }) : "—");

function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function search() {
  // Users paste the whole line, e.g. "参考号：xxxx" or "奖励编号：xxxx".
  const ref = query.value.replace(/^.*[:：]\s*/, "").trim();
  if (!ref) return;
  query.value = ref;
  open.value = true;
  loading.value = true;
  error.value = "";
  notice.value = "";
  result.value = null;
  liftReason.value = "";
  controller.abort();
  controller = new AbortController();
  try {
    result.value = await props.api(`/admin/trace/${encodeURIComponent(ref)}`, { signal: controller.signal });
    await nextTick();
    panel.value?.focus();
  } catch (e) {
    fail(e);
  } finally {
    loading.value = false;
  }
}
async function lift() {
  const user = result.value?.user;
  if (!user || !liftReason.value.trim()) return;
  lifting.value = true;
  error.value = "";
  try {
    await props.api(`/admin/users/${user.id}/restriction`, {
      method: "PUT",
      body: { status: "active", reason: liftReason.value.trim() },
      signal: controller.signal,
    });
    notice.value = "已解除限制，用户可立即正常领取";
    const keep = notice.value;
    await search();
    notice.value = keep;
  } catch (e) {
    fail(e);
  } finally {
    lifting.value = false;
  }
}
function viewUser() {
  emit("open-user", result.value.user.id);
  open.value = false;
}
onUnmounted(() => controller.abort());
</script>

<template>
  <form class="trace-search" role="search" @submit.prevent="search">
    <Icon name="search" :size="15" />
    <input v-model="query" maxlength="120" placeholder="参考号 / 奖励编号" aria-label="按参考号或奖励编号查询" />
  </form>

  <div v-if="open" class="trace-backdrop" @click.self="open = false">
    <aside ref="panel" class="trace-panel" tabindex="-1" role="dialog" aria-label="参考号查询结果" @keydown.esc="open = false">
      <header class="trace-head">
        <div>
          <h2>参考号查询</h2>
          <code>{{ query }}</code>
        </div>
        <button type="button" class="trace-close" aria-label="关闭" @click="open = false"><Icon name="close" :size="18" /></button>
      </header>

      <div v-if="error" class="form-error" role="alert">{{ error }}</div>
      <div v-if="notice" class="trace-notice"><Icon name="check" :size="15" />{{ notice }}</div>
      <p v-if="loading" class="trace-muted">正在查询…</p>
      <p v-else-if="result && !result.found" class="trace-empty">没有找到这个编号。请确认是用户界面上“参考号”或“奖励编号”后面的完整内容。</p>

      <template v-if="result?.found">
        <section v-if="result.user" class="trace-card">
          <div class="trace-user">
            <div>
              <b>{{ result.user.nickname || "用户" }}</b>
              <span>ID {{ result.user.id }} · {{ result.user.phone || result.user.username || "—" }}</span>
              <span v-if="result.user.review_account" class="pill blue">审核账号（自动风控豁免）</span>
            </div>
            <button type="button" class="outline small" @click="viewUser"><Icon name="user" :size="14" />查看用户</button>
          </div>
          <div class="trace-status" :class="result.user.restriction.status">
            <div>
              <span>账号状态</span>
              <b>{{ STATUS[result.user.restriction.status] || result.user.restriction.status }}</b>
            </div>
            <p v-if="result.user.restriction.status !== 'active'">
              {{ prettyReason(result.user.restriction.reason) }}<br />
              <small>由 {{ result.user.restriction.updated_by }} 于 {{ time(result.user.restriction.updated_at) }} 设置</small>
            </p>
          </div>
          <form v-if="result.user.restriction.status !== 'active'" class="trace-lift" @submit.prevent="lift">
            <input v-model="liftReason" maxlength="120" placeholder="解除理由（必填），如：已人工核实为正常用户" aria-label="解除理由" />
            <button class="primary" :disabled="lifting || !liftReason.trim()">{{ lifting ? "处理中…" : "解除限制" }}</button>
          </form>
        </section>

        <template v-for="(m, i) in result.matches" :key="i">
          <section v-if="m.type === 'risk_event'" class="trace-card">
            <h3>设备核验 · {{ OPERATION[m.operation] || m.operation }}</h3>
            <dl class="trace-kv">
              <div><dt>时间</dt><dd>{{ time(m.time) }}</dd></div>
              <div><dt>结论</dt><dd>{{ DECISION[m.decision] || m.decision || "—" }} / {{ ACTION[m.action] || m.action || "—" }}</dd></div>
              <div><dt>风险分</dt><dd>{{ m.score ?? "—" }}</dd></div>
              <div><dt>IP</dt><dd>{{ m.client_ip || "—" }}<small v-if="m.risk.ip_location"> · {{ m.risk.ip_location }}</small></dd></div>
            </dl>
            <div v-if="reasonList(m.risk).length" class="trace-reasons">
              <span v-for="r in reasonList(m.risk)" :key="r" class="pill warn" :title="r">{{ reasonLabel(r) }}</span>
            </div>
            <details v-if="m.risk.hits?.length" class="trace-more">
              <summary>命中规则明细（{{ m.risk.hits.length }}）</summary>
              <ul>
                <li v-for="(h, j) in m.risk.hits" :key="j">
                  <b>{{ h.label || reasonLabel(h.code) || "规则" }}</b>
                  <code v-if="h.code">{{ h.code }}</code>
                  <span v-if="h.score != null">+{{ h.score }}</span>
                  <small v-if="h.detail != null">{{ typeof h.detail === "string" ? h.detail : JSON.stringify(h.detail) }}</small>
                </li>
              </ul>
            </details>
            <p v-if="m.error" class="trace-muted">接口错误：{{ m.error }}</p>
          </section>

          <section v-else-if="m.type === 'restriction'" class="trace-card">
            <h3>这是一次账号限制的编号</h3>
            <p class="trace-text">{{ STATUS[m.status] || m.status }} · {{ prettyReason(m.reason) }}<br /><small>由 {{ m.updated_by }} 于 {{ time(m.time) }} 设置</small></p>
          </section>

          <section v-else-if="m.type === 'reward'" class="trace-card">
            <h3>激励红包 · {{ m.coins }} 金币 · {{ m.status }}</h3>
            <dl class="trace-kv">
              <div><dt>开始</dt><dd>{{ time(m.started_at) }}</dd></div>
              <div><dt>平台</dt><dd>{{ NETWORK[m.network] || m.network || "—" }}</dd></div>
              <div><dt>平台确认</dt><dd>{{ time(m.verified_at) }}</dd></div>
              <div><dt>到账</dt><dd>{{ time(m.completed_at) }}</dd></div>
            </dl>
            <ol v-if="m.ad_trail.length" class="trace-trail">
              <li v-for="(e, j) in m.ad_trail" :key="j" :class="{ bad: ['risk_block', 'callback_reject', 'error', 'no_fill', 'timeout'].includes(e.event) }">
                <span>{{ time(e.time).slice(-8) }}</span>
                <b>{{ NETWORK[e.network] || e.network }}</b>
                <em>{{ AD_EVENT[e.event] || e.event }}</em>
                <small>{{ [e.code, e.message].filter(Boolean).join(" ") }}</small>
              </li>
            </ol>
            <p v-if="m.callbacks.length" class="trace-muted">平台回调：{{ m.callbacks.map((c) => `${c.status === "accepted" ? "通过" : "拒绝"}（${c.reason}）`).join("，") }}</p>
          </section>

          <section v-else-if="m.type === 'withdrawal'" class="trace-card">
            <h3>提现申请 · {{ m.amount_yuan }} 元</h3>
            <p class="trace-text">{{ m.status }} · {{ m.channel }} · {{ time(m.time) }}</p>
          </section>

          <section v-else-if="m.type === 'audit'" class="trace-card">
            <h3>处置记录</h3>
            <ul class="trace-list">
              <li v-for="(a, j) in m.items" :key="j">{{ time(a.time) }} · {{ a.actor }} · {{ a.action }}<small v-if="a.reason"> · {{ a.reason }}</small></li>
            </ul>
          </section>

          <section v-else-if="m.type === 'behavior'" class="trace-card">
            <h3>业务记录 · {{ m.kind }}</h3>
            <p class="trace-text">{{ m.status }} · {{ time(m.time) }}</p>
          </section>
        </template>

        <section v-if="result.devices.length" class="trace-card">
          <h3>账号设备（{{ result.devices.length }}）</h3>
          <ul class="trace-devices">
            <li v-for="d in result.devices" :key="d.device_id" :class="{ hit: d.this_event }">
              <div>
                <b>{{ d.model || "未上报型号" }}</b>
                <span v-if="d.this_event" class="pill warn">本次事件设备</span>
                <small>{{ d.platform }} {{ d.os_version }}</small>
              </div>
              <code :title="d.device_id">{{ d.device_id.replace(/^device:/, "").slice(0, 16) }}…</code>
              <small>最近 {{ time(d.last_seen) }}</small>
            </li>
          </ul>
        </section>
      </template>
    </aside>
  </div>
</template>

<style scoped>
.trace-search { display: flex; align-items: center; gap: 8px; background: #fffdfa; border: 1px solid #ecdccd; border-radius: 10px; padding: 0 12px; height: 36px; color: #b49777; width: 260px; }
.trace-search input { border: 0; background: transparent; padding: 0; margin: 0; height: 100%; font-size: 14px; box-shadow: none; }
.trace-search input:focus { box-shadow: none; }
.trace-search:focus-within { border-color: #ef916b; box-shadow: 0 0 0 3px #ef58360c; }
.trace-backdrop { position: fixed; inset: 0; background: rgba(60, 40, 28, 0.28); z-index: 60; display: flex; justify-content: flex-end; }
.trace-panel { width: min(560px, 100%); height: 100%; overflow-y: auto; background: #faf7f2; padding: 22px 22px 40px; box-shadow: -12px 0 40px rgba(80, 50, 30, 0.18); outline: none; }
.trace-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; margin-bottom: 14px; }
.trace-head h2 { font-size: 18px; color: #4f3627; margin: 0 0 6px; }
.trace-head code { font-size: 13.5px; color: #8a6f61; background: #fff; border: 1px solid #f0e2d6; border-radius: 8px; padding: 3px 8px; word-break: break-all; }
.trace-close { border: 0; background: #fff; border-radius: 10px; width: 34px; height: 34px; display: grid; place-items: center; color: #8a6f61; cursor: pointer; }
.trace-card { background: #fffdfa; border: 0; border-radius: 14px; padding: 14px 16px; margin-bottom: 10px; }
.trace-card h3 { font-size: 14px; color: #5d4030; margin: 0 0 10px; }
.trace-muted, .trace-empty { color: #a8907f; font-size: 14px; line-height: 1.7; }
.trace-empty { background: #fffdfa; border: 0; border-radius: 14px; padding: 18px; }
.trace-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 10px 12px; margin-bottom: 12px; font-size: 14px; }
.trace-user { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
.trace-user > div { display: flex; flex-wrap: wrap; gap: 6px 10px; align-items: center; }
.trace-user b { font-size: 15px; color: #4f3627; }
.trace-user span { font-size: 14px; color: #8a6f61; }
.trace-status { margin-top: 12px; border-radius: 12px; padding: 12px 14px; background: #eefaf1; }
.trace-status > div { display: flex; justify-content: space-between; font-size: 14px; }
.trace-status > div span { color: #6f8b77; }
.trace-status b { color: #2f8a4c; }
.trace-status.review, .trace-status.restricted { background: #fff4ec; }
.trace-status.review > div span, .trace-status.restricted > div span { color: #a5735a; }
.trace-status.review b, .trace-status.restricted b { color: #c2481f; }
.trace-status p { margin: 8px 0 0; font-size: 14px; color: #6d5547; line-height: 1.6; }
.trace-status small, .trace-text small { color: #a8907f; }
.trace-lift { display: flex; gap: 8px; margin-top: 12px; }
.trace-lift input { flex: 1; margin: 0; padding: 9px 12px; font-size: 14px; border-radius: 10px; }
.trace-lift .primary { width: auto; margin: 0; padding: 9px 16px; white-space: nowrap; }
.trace-kv { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px 14px; margin: 0; }
.trace-kv div { display: grid; gap: 2px; }
.trace-kv dt { font-size: 13px; color: #a8907f; }
.trace-kv dd { margin: 0; font-size: 14px; color: #4f3627; word-break: break-all; }
.trace-kv small { color: #8a6f61; }
.trace-reasons { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 12px; }
.pill { font-size: 13.5px; border-radius: 999px; padding: 3px 10px; background: #f7efe8; color: #8a6f61; white-space: nowrap; }
.pill.warn { background: #fff0e8; color: #c2481f; }
.pill.blue { background: #eef4ff; color: #3b6fc4; }
.trace-more { margin-top: 10px; font-size: 14px; color: #6d5547; }
.trace-more summary { cursor: pointer; color: #b0623f; }
.trace-more ul, .trace-list { margin: 8px 0 0; padding-left: 18px; display: grid; gap: 6px; font-size: 14px; color: #6d5547; }
.trace-more code { font-size: 13px; color: #a8907f; margin: 0 6px; }
.trace-more small { display: block; color: #a8907f; word-break: break-all; }
.trace-text { margin: 0; font-size: 14px; color: #6d5547; line-height: 1.7; }
.trace-trail { list-style: none; margin: 12px 0 0; padding: 0; display: grid; gap: 4px; font-size: 13.5px; }
.trace-trail li { display: grid; grid-template-columns: 62px 52px 72px 1fr; gap: 6px; padding: 5px 8px; border-radius: 8px; background: #fbf6f1; color: #6d5547; }
.trace-trail li.bad { background: #fff3ee; }
.trace-trail em { font-style: normal; }
.trace-trail small { color: #a8907f; word-break: break-all; }
.trace-devices { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
.trace-devices li { display: grid; gap: 4px; padding: 10px 12px; border-radius: 10px; background: #fbf6f1; font-size: 14px; }
.trace-devices li.hit { background: #fff4ec; border: 0; }
.trace-devices li > div { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.trace-devices b { color: #4f3627; }
.trace-devices small { color: #a8907f; }
.trace-devices code { font-size: 13px; color: #8a6f61; }
button.small { padding: 6px 10px; font-size: 13.5px; display: inline-flex; gap: 5px; align-items: center; white-space: nowrap; }
@media (max-width: 900px) { .trace-search { width: 170px; } }
@media (max-width: 640px) { .trace-search { width: 130px; } .trace-kv { grid-template-columns: 1fr; } }
</style>
