<script setup>
import { ref, computed, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const data = ref(null),
  cfg = ref(null),
  error = ref(""),
  notice = ref(""),
  saving = ref(false);
const controller = new AbortController();
onUnmounted(() => controller.abort());
const GROUPS = [
  { key: "welcome_tiers", title: "新人红包", hint: "新注册或本期受邀用户拆自己的新人红包，按本人设备风险分。" },
  { key: "inviter_new_tiers", title: "邀请新用户红包", hint: "邀请人每邀请 1 位新用户（完成首签或首个红包）得 1 次，按被邀请人的风险分。" },
  { key: "inviter_old_tiers", title: "邀请老用户红包", hint: "已注册用户扫码参与本期活动时，邀请人得 1 次，金额应明显低于新用户。" },
];
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
function apply(result) {
  data.value = result;
  cfg.value = JSON.parse(JSON.stringify(result.config));
}
async function refresh() {
  error.value = "";
  try {
    apply(await props.api("/admin/newbie", { signal: controller.signal }));
  } catch (e) {
    fail(e);
  }
}
async function save(newRound = false) {
  if (newRound && !confirm(`确定开启第 ${cfg.value.round + 1} 期？\n\n新一期开始后：本期之后注册的用户可领新人红包；老用户可再次被邀请参与；上一期未拆的红包将不再显示。`)) return;
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    const { round, round_started_at, revision, ...config } = cfg.value;
    apply(await props.api("/admin/newbie", { method: "PUT", body: { ...config, new_round: newRound }, signal: controller.signal }));
    notice.value = newRound ? `已开启第 ${data.value.config.round} 期` : "配置已保存，App 约 45 秒内生效";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
function addTier(key) {
  const list = cfg.value[key];
  const last = list[list.length - 1];
  list.splice(list.length - 1, 0, { max_score: Math.max(0, last.max_score - 10), min_coins: last.min_coins, max_coins: last.max_coins });
}
function removeTier(key, i) {
  const list = cfg.value[key];
  if (list.length > 1) list.splice(i, 1);
  list[list.length - 1].max_score = 100;
}
function lower(list, i) {
  return i === 0 ? 0 : list[i - 1].max_score + 1;
}
const fmt = (n) => Number(n || 0).toLocaleString("zh-CN");
const yuan = (n) => (Number(n || 0) / 1000).toFixed(2);
const s = computed(() => data.value?.stats || {});
const KIND = { welcome: "新人红包", invite: "邀请红包" };
function time(v) {
  return v ? new Date(v).toLocaleString("zh-CN", { hour12: false, month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }) : "";
}
defineExpose({ refresh });
refresh();
</script>
<template>
  <div class="nb-page">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="nb-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <section v-if="!data && !error" class="admin-card nb-card"><p class="nb-muted">正在读取新人礼包配置…</p></section>

    <template v-if="data && cfg">
      <section class="nb-stats">
        <div class="nb-stat"><span>本期</span><b>第 {{ data.config.round }} 期</b><small>{{ time(data.config.round_started_at) }} 开始</small></div>
        <div class="nb-stat"><span>受邀用户</span><b>{{ fmt(s.members) }}</b><small>新 {{ fmt(s.members_new) }} · 老 {{ fmt(s.members_old) }} · 达标 {{ fmt(s.qualified) }}</small></div>
        <div class="nb-stat"><span>新人红包</span><b>{{ fmt(s.welcome_opened) }} 个</b><small>{{ fmt(s.welcome_coins) }} 金币 ≈ ¥{{ yuan(s.welcome_coins) }}</small></div>
        <div class="nb-stat"><span>邀请红包</span><b>{{ fmt(s.invite_opened) }} 个</b><small>待拆 {{ fmt(s.invite_ready) }} · {{ fmt(s.invite_coins) }} 金币 ≈ ¥{{ yuan(s.invite_coins) }}</small></div>
      </section>

      <form class="admin-card nb-card" @submit.prevent="save(false)">
        <header class="nb-head">
          <div>
            <h2>活动设置</h2>
            <p>新用户首次进入大厅自动弹出大红包；大厅右上角显示悬浮礼包入口，点击进入活动页（网页 /activity/newbie.html）。金币全部由服务端按设备风险分计算。</p>
          </div>
          <span class="nb-state" :class="{ on: cfg.enabled }">{{ cfg.enabled ? "进行中" : "未开启" }}</span>
        </header>
        <div class="nb-toggles">
          <label class="nb-toggle"><div><b>开启活动</b><p>关闭后入口和弹窗隐藏，未拆红包暂不可拆。</p></div><span class="nb-switch"><input v-model="cfg.enabled" type="checkbox" aria-label="开启活动" /><span></span></span></label>
          <label class="nb-toggle"><div><b>首次进入自动弹出</b><p>每个账号每期只弹一次（新人红包未拆时）。</p></div><span class="nb-switch"><input v-model="cfg.auto_popup" type="checkbox" aria-label="自动弹出" /><span></span></span></label>
        </div>
        <div class="nb-fields">
          <label>活动名称<input v-model="cfg.title" maxlength="12" required /></label>
          <label>入口文字（≤6 字）<input v-model="cfg.entry_label" maxlength="6" required /></label>
          <label>副标题<input v-model="cfg.subtitle" maxlength="40" required /></label>
          <label class="wide">分享文案（随海报一起发送，≤120 字）<input v-model="cfg.share_text" maxlength="120" /></label>
          <label>受邀新用户加成 %<input v-model.number="cfg.invited_bonus_percent" type="number" min="0" max="300" required /></label>
          <label>无评分时按分数<input v-model.number="cfg.unscored_score" type="number" min="0" max="100" required /></label>
          <label>每人邀请红包上限（0 不限）<input v-model.number="cfg.max_invite_packets" type="number" min="0" max="10000" required /></label>
        </div>

        <div v-for="g in GROUPS" :key="g.key" class="nb-tiers">
          <div class="nb-tiers-head"><h3>{{ g.title }}</h3><p>{{ g.hint }}</p></div>
          <table>
            <thead><tr><th>设备风险分</th><th>最少金币</th><th>最多金币</th><th>约合</th><th></th></tr></thead>
            <tbody>
              <tr v-for="(t, i) in cfg[g.key]" :key="i">
                <td class="score">
                  <span class="range">{{ lower(cfg[g.key], i) }} –</span>
                  <input v-if="i < cfg[g.key].length - 1" v-model.number="t.max_score" type="number" min="0" max="99" />
                  <b v-else>100</b>
                  <i class="dot" :style="{ background: `hsl(${120 - (t.max_score * 1.2)},70%,45%)` }"></i>
                </td>
                <td><input v-model.number="t.min_coins" type="number" min="1" max="100000" /></td>
                <td><input v-model.number="t.max_coins" type="number" min="1" max="100000" /></td>
                <td class="nb-muted">¥{{ yuan(t.min_coins) }} – ¥{{ yuan(t.max_coins) }}</td>
                <td><button v-if="cfg[g.key].length > 1" type="button" class="linkish" @click="removeTier(g.key, i)">删除</button></td>
              </tr>
            </tbody>
          </table>
          <button v-if="cfg[g.key].length < 10" type="button" class="linkish" @click="addTier(g.key)">+ 增加一档</button>
        </div>
        <p class="nb-hint">风险分来自 DeviceTrust（0 最干净，100 最差）；账号被限制时按 100 计。全黑设备落在最后一档（例如 10 金币），干净设备拿高额随机。新人红包对受邀新用户再乘以加成。</p>
        <div class="nb-actions">
          <button class="primary" :disabled="saving"><Icon name="check" :size="16" />{{ saving ? "保存中…" : "保存配置" }}</button>
          <button type="button" class="outline" :disabled="saving" @click="save(true)"><Icon name="spark" :size="15" />开启新一期</button>
        </div>
      </form>

      <section class="admin-card nb-card">
        <h2>最近拆开的红包</h2>
        <p v-if="!data.recent.length" class="nb-muted">本期还没有人拆红包。</p>
        <table v-else class="nb-recent">
          <thead><tr><th>时间</th><th>用户</th><th>类型</th><th>被邀请人</th><th>风险分</th><th>金币</th></tr></thead>
          <tbody>
            <tr v-for="(r, i) in data.recent" :key="i">
              <td>{{ time(r.opened_at) }}</td>
              <td>{{ r.username || "用户 " + r.user_id }}<small>{{ r.phone }}</small></td>
              <td><span class="kind" :class="r.kind">{{ KIND[r.kind] }}</span></td>
              <td>{{ r.invitee_phone || "—" }}</td>
              <td>{{ r.score ?? "—" }}</td>
              <td class="coins">{{ fmt(r.coins) }}</td>
            </tr>
          </tbody>
        </table>
      </section>
    </template>
  </div>
</template>
<style scoped>
.nb-card { padding: 18px 20px; margin-bottom: 12px; }
.nb-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.nb-card h3 { font-size: 15px; color: #4f3627; margin: 0; }
.nb-head { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; }
.nb-head p, .nb-muted { color: #9a8274; font-size: 13.5px; line-height: 1.7; margin: 0; max-width: 820px; }
.nb-state { font-size: 13px; border-radius: 999px; padding: 5px 12px; background: #f7efe8; color: #8a6f61; white-space: nowrap; }
.nb-state.on { background: #fff0ec; color: #e2321f; }
.nb-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 10px 14px; margin-bottom: 12px; font-size: 13.5px; }
.nb-stats { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-bottom: 12px; }
.nb-stat { background: linear-gradient(160deg, #fff, #fff6ee); border-radius: 16px; padding: 14px 16px; display: grid; gap: 4px; box-shadow: 0 1px 0 #f2e8d9; }
.nb-stat span { font-size: 13px; color: #a8907f; }
.nb-stat b { font-size: 22px; color: #e2321f; }
.nb-stat small { font-size: 12.5px; color: #9a8274; }
.nb-toggles { display: grid; grid-template-columns: 1fr 1fr; gap: 0 24px; border-top: 1px solid #f2e8d9; margin-top: 12px; }
.nb-toggle { display: flex; justify-content: space-between; align-items: center; gap: 16px; padding: 12px 0; margin: 0; cursor: pointer; }
.nb-toggle b { font-size: 14px; color: #6f4b33; }
.nb-toggle p { font-size: 12.5px; color: #a8907f; margin: 3px 0 0; }
.nb-switch { position: relative; display: block; width: 42px; height: 24px; flex-shrink: 0; }
.nb-switch input { opacity: 0; width: 100%; height: 100%; position: absolute; inset: 0; margin: 0; padding: 0; cursor: pointer; }
.nb-switch > span { pointer-events: none; display: block; height: 24px; border-radius: 30px; background: #dfd4c3; transition: background 0.2s; }
.nb-switch > span::after { content: ""; position: absolute; height: 18px; width: 18px; top: 3px; left: 3px; background: #fff; border-radius: 50%; transition: transform 0.2s; }
.nb-switch input:checked + span { background: #ec985b; }
.nb-switch input:checked + span::after { transform: translateX(18px); }
.nb-fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px 14px; margin-top: 8px; }
.nb-fields label { margin: 0; display: grid; gap: 6px; font-size: 13px; }
.nb-fields label.wide { grid-column: span 3; }
.nb-fields input { margin: 0; padding: 9px 11px; font-size: 13.5px; border-radius: 10px; }
.nb-tiers { margin-top: 18px; border-top: 1px solid #f2e8d9; padding-top: 14px; }
.nb-tiers-head { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; margin-bottom: 8px; }
.nb-tiers-head p { margin: 0; font-size: 12.5px; color: #a8907f; }
.nb-tiers table, .nb-recent { width: 100%; border-collapse: collapse; font-size: 13.5px; }
.nb-tiers th, .nb-recent th { text-align: left; font-weight: 600; color: #a8907f; font-size: 12.5px; padding: 6px 8px; }
.nb-tiers td, .nb-recent td { padding: 5px 8px; border-top: 1px solid #f7efe6; }
.nb-tiers input { width: 110px; margin: 0; padding: 7px 9px; font-size: 13.5px; border-radius: 9px; }
.nb-tiers td.score { display: flex; align-items: center; gap: 6px; white-space: nowrap; }
.nb-tiers td.score input { width: 70px; }
.nb-tiers td.score b { display: inline-block; width: 70px; padding-left: 10px; color: #4f3627; }
.range { color: #8a6f61; min-width: 38px; }
.dot { width: 9px; height: 9px; border-radius: 50%; display: inline-block; }
.nb-hint { font-size: 13px; color: #8a6f61; background: #fbf6f1; border-radius: 10px; padding: 10px 12px; line-height: 1.7; margin: 16px 0 0; }
.nb-actions { margin-top: 14px; display: flex; gap: 10px; }
.nb-actions .primary, .nb-actions .outline { width: auto; padding: 10px 22px; margin: 0; display: inline-flex; gap: 6px; align-items: center; }
.nb-recent small { display: block; color: #a8907f; font-size: 12px; }
.nb-recent .coins { font-weight: 700; color: #e2321f; }
.kind { font-size: 12px; padding: 2px 8px; border-radius: 8px; }
.kind.welcome { background: #fff0ec; color: #e2321f; }
.kind.invite { background: #fff6e0; color: #c47a00; }
.linkish { border: 0; background: none; color: #f65331; cursor: pointer; padding: 4px; font: inherit; font-size: 13px; }
@media (max-width: 1000px) { .nb-stats { grid-template-columns: 1fr 1fr; } .nb-fields { grid-template-columns: 1fr; } .nb-fields label.wide { grid-column: auto; } .nb-toggles { grid-template-columns: 1fr; } }
</style>
