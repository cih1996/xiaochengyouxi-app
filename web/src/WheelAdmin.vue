<script setup>
import { ref, computed, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const data = ref(null), cfg = ref(null), error = ref(""), notice = ref(""), saving = ref(false);
const controller = new AbortController();
onUnmounted(() => controller.abort());
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
  try { apply(await props.api("/admin/wheel", { signal: controller.signal })); } catch (e) { fail(e); }
}
async function save(newRound = false) {
  if (newRound && !confirm(`确定开启第 ${cfg.value.round + 1} 期？\n\n新一期开始后，邀请得到的次数重新累计，获奖播报只展示新一期。`)) return;
  saving.value = true; error.value = ""; notice.value = "";
  try {
    const { round, round_started_at, revision, ...config } = cfg.value;
    apply(await props.api("/admin/wheel", { method: "PUT", body: { ...config, new_round: newRound }, signal: controller.signal }));
    notice.value = newRound ? `已开启第 ${data.value.config.round} 期` : "配置已保存，App 约 45 秒内生效";
  } catch (e) { fail(e); } finally { saving.value = false; }
}
const totalWeight = computed(() => (cfg.value?.slots || []).reduce((a, s) => a + (Number(s.weight) || 0), 0));
const pct = (w) => (totalWeight.value ? ((Number(w) || 0) / totalWeight.value * 100).toFixed(1) : "0.0") + "%";
const expected = computed(() => {
  const t = totalWeight.value; if (!t) return 0;
  return Math.round(cfg.value.slots.reduce((a, s) => a + s.coins * (Number(s.weight) || 0), 0) / t);
});
function addSlot() { if (cfg.value.slots.length < 12) cfg.value.slots.push({ label: "100金币", coins: 100, weight: 10 }); }
function removeSlot(i) { if (cfg.value.slots.length > 6) cfg.value.slots.splice(i, 1); }
function addCap() { const c = cfg.value.caps, last = c[c.length - 1]; c.splice(c.length - 1, 0, { max_score: Math.max(0, last.max_score - 10), max_coins: last.max_coins }); }
function removeCap(i) { const c = cfg.value.caps; if (c.length > 1) c.splice(i, 1); c[c.length - 1].max_score = 100; }
const lower = (list, i) => (i === 0 ? 0 : list[i - 1].max_score + 1);
const fmt = (n) => Number(n || 0).toLocaleString("zh-CN");
const yuan = (n) => (Number(n || 0) / 1000).toFixed(2);
const SOURCE = { free: "免费", video: "看视频", invite: "邀请" };
const time = (v) => (v ? new Date(v).toLocaleString("zh-CN", { hour12: false, month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }) : "");
defineExpose({ refresh });
refresh();
</script>
<template>
  <div class="wa-page">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="wa-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <section v-if="!data && !error" class="admin-card wa-card"><p class="wa-muted">正在读取幸运转盘配置…</p></section>
    <template v-if="data && cfg">
      <section class="wa-stats">
        <div class="wa-stat"><span>本期</span><b>第 {{ data.config.round }} 期</b><small>{{ time(data.config.round_started_at) }} 开始</small></div>
        <div class="wa-stat"><span>参与人数</span><b>{{ fmt(data.stats.players) }}</b><small>共抽 {{ fmt(data.stats.spins) }} 次</small></div>
        <div class="wa-stat"><span>看视频 / 邀请</span><b>{{ fmt(data.stats.video_spins) }} / {{ fmt(data.stats.invite_spins) }}</b><small>次</small></div>
        <div class="wa-stat"><span>发放金币</span><b>{{ fmt(data.stats.coins) }}</b><small>≈ ¥{{ yuan(data.stats.coins) }}</small></div>
      </section>
      <form class="admin-card wa-card" @submit.prevent="save(false)">
        <header class="wa-head">
          <div>
            <h2>转盘设置</h2>
            <p>大厅右上角显示“幸运转盘”悬浮入口，点开为网页活动 /activity/wheel.html。转到哪一格由服务端决定：先按用户设备风险分得到可中金币上限，再在不超过上限的奖项里按权重抽取。看视频抽奖需广告平台回调验证通过才到账，不占大厅每日红包次数。</p>
          </div>
          <span class="wa-state" :class="{ on: cfg.enabled }">{{ cfg.enabled ? "进行中" : "未开启" }}</span>
        </header>
        <label class="wa-toggle"><div><b>开启活动</b><p>关闭后悬浮入口隐藏。</p></div><span class="wa-switch"><input v-model="cfg.enabled" type="checkbox" aria-label="开启活动" /><span></span></span></label>
        <div class="wa-fields">
          <label>活动名称<input v-model="cfg.title" maxlength="12" required /></label>
          <label>入口文字（≤6 字）<input v-model="cfg.entry_label" maxlength="6" required /></label>
          <label>无评分时按分数<input v-model.number="cfg.unscored_score" type="number" min="0" max="100" required /></label>
          <label>每日免费次数<input v-model.number="cfg.free_per_day" type="number" min="0" max="10" required /></label>
          <label>每日看视频次数<input v-model.number="cfg.video_per_day" type="number" min="0" max="20" required /></label>
          <label>每邀请 1 人得次数（本期内）<input v-model.number="cfg.invite_per_invitee" type="number" min="0" max="10" required /></label>
        </div>
        <div class="wa-block">
          <div class="wa-block-head"><h3>奖项（转盘上的格子，按顺时针）</h3><p>理论平均每次 {{ fmt(expected) }} 金币（未计风险封顶，实际更低）</p></div>
          <table>
            <thead><tr><th>#</th><th>名称</th><th>金币</th><th>权重</th><th>概率</th><th></th></tr></thead>
            <tbody>
              <tr v-for="(s, i) in cfg.slots" :key="i">
                <td class="wa-muted">{{ i + 1 }}</td>
                <td><input v-model="s.label" maxlength="8" /></td>
                <td><input v-model.number="s.coins" type="number" min="1" max="100000" /></td>
                <td><input v-model.number="s.weight" type="number" min="0" max="10000" /></td>
                <td class="wa-muted">{{ pct(s.weight) }}</td>
                <td><button v-if="cfg.slots.length > 6" type="button" class="linkish" @click="removeSlot(i)">删除</button></td>
              </tr>
            </tbody>
          </table>
          <button v-if="cfg.slots.length < 12" type="button" class="linkish" @click="addSlot">+ 增加奖项</button>
        </div>
        <div class="wa-block">
          <div class="wa-block-head"><h3>风险分封顶</h3><p>用户只会转到不超过该档金币上限的奖项；全黑设备落在最后一档。</p></div>
          <table>
            <thead><tr><th>设备风险分</th><th>可中金币上限</th><th></th></tr></thead>
            <tbody>
              <tr v-for="(t, i) in cfg.caps" :key="i">
                <td class="score"><span>{{ lower(cfg.caps, i) }} –</span><input v-if="i < cfg.caps.length - 1" v-model.number="t.max_score" type="number" min="0" max="99" /><b v-else>100</b></td>
                <td><input v-model.number="t.max_coins" type="number" min="1" max="100000" /></td>
                <td><button v-if="cfg.caps.length > 1" type="button" class="linkish" @click="removeCap(i)">删除</button></td>
              </tr>
            </tbody>
          </table>
          <button v-if="cfg.caps.length < 10" type="button" class="linkish" @click="addCap">+ 增加一档</button>
        </div>
        <div class="wa-actions">
          <button class="primary" :disabled="saving"><Icon name="check" :size="16" />{{ saving ? "保存中…" : "保存配置" }}</button>
          <button type="button" class="outline" :disabled="saving" @click="save(true)"><Icon name="spark" :size="15" />开启新一期</button>
        </div>
      </form>
      <section class="admin-card wa-card">
        <h2>最近中奖</h2>
        <p v-if="!data.recent.length" class="wa-muted">本期还没有人抽奖。</p>
        <table v-else class="wa-recent">
          <thead><tr><th>时间</th><th>用户</th><th>来源</th><th>风险分</th><th>金币</th></tr></thead>
          <tbody>
            <tr v-for="(r, i) in data.recent" :key="i">
              <td>{{ time(r.done_at) }}</td>
              <td>{{ r.username || "用户 " + r.user_id }}<small>{{ r.phone }}</small></td>
              <td>{{ SOURCE[r.source] || r.source }}</td>
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
.wa-card { padding: 18px 20px; margin-bottom: 12px; }
.wa-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.wa-card h3 { font-size: 15px; color: #4f3627; margin: 0; }
.wa-head { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; }
.wa-head p, .wa-muted { color: #9a8274; font-size: 13.5px; line-height: 1.7; margin: 0; max-width: 860px; }
.wa-state { font-size: 13px; border-radius: 999px; padding: 5px 12px; background: #f7efe8; color: #8a6f61; white-space: nowrap; }
.wa-state.on { background: #fff0ec; color: #e2321f; }
.wa-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 10px 14px; margin-bottom: 12px; font-size: 13.5px; }
.wa-stats { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-bottom: 12px; }
.wa-stat { background: linear-gradient(160deg, #fff, #fff6ee); border-radius: 16px; padding: 14px 16px; display: grid; gap: 4px; box-shadow: 0 1px 0 #f2e8d9; }
.wa-stat span { font-size: 13px; color: #a8907f; }
.wa-stat b { font-size: 22px; color: #e2321f; }
.wa-stat small { font-size: 12.5px; color: #9a8274; }
.wa-toggle { display: flex; justify-content: space-between; align-items: center; gap: 16px; padding: 12px 0; margin: 12px 0 0; border-top: 1px solid #f2e8d9; cursor: pointer; }
.wa-toggle b { font-size: 14px; color: #6f4b33; }
.wa-toggle p { font-size: 12.5px; color: #a8907f; margin: 3px 0 0; }
.wa-switch { position: relative; display: block; width: 42px; height: 24px; flex-shrink: 0; }
.wa-switch input { opacity: 0; width: 100%; height: 100%; position: absolute; inset: 0; margin: 0; padding: 0; cursor: pointer; }
.wa-switch > span { pointer-events: none; display: block; height: 24px; border-radius: 30px; background: #dfd4c3; transition: background 0.2s; }
.wa-switch > span::after { content: ""; position: absolute; height: 18px; width: 18px; top: 3px; left: 3px; background: #fff; border-radius: 50%; transition: transform 0.2s; }
.wa-switch input:checked + span { background: #ec985b; }
.wa-switch input:checked + span::after { transform: translateX(18px); }
.wa-fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px 14px; margin-top: 4px; }
.wa-fields label { margin: 0; display: grid; gap: 6px; font-size: 13px; }
.wa-fields input { margin: 0; padding: 9px 11px; font-size: 13.5px; border-radius: 10px; }
.wa-block { margin-top: 18px; border-top: 1px solid #f2e8d9; padding-top: 14px; }
.wa-block-head { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; margin-bottom: 8px; }
.wa-block-head p { margin: 0; font-size: 12.5px; color: #a8907f; }
.wa-block table, .wa-recent { width: 100%; border-collapse: collapse; font-size: 13.5px; }
.wa-block th, .wa-recent th { text-align: left; font-weight: 600; color: #a8907f; font-size: 12.5px; padding: 6px 8px; }
.wa-block td, .wa-recent td { padding: 5px 8px; border-top: 1px solid #f7efe6; }
.wa-block input { width: 120px; margin: 0; padding: 7px 9px; font-size: 13.5px; border-radius: 9px; }
.wa-block td.score { display: flex; align-items: center; gap: 6px; }
.wa-block td.score input { width: 70px; }
.wa-block td.score b { width: 70px; padding-left: 10px; color: #4f3627; }
.wa-actions { margin-top: 16px; display: flex; gap: 10px; }
.wa-actions .primary, .wa-actions .outline { width: auto; padding: 10px 22px; margin: 0; display: inline-flex; gap: 6px; align-items: center; }
.wa-recent small { display: block; color: #a8907f; font-size: 12px; }
.wa-recent .coins { font-weight: 700; color: #e2321f; }
.linkish { border: 0; background: none; color: #f65331; cursor: pointer; padding: 4px; font: inherit; font-size: 13px; }
@media (max-width: 1000px) { .wa-stats { grid-template-columns: 1fr 1fr; } .wa-fields { grid-template-columns: 1fr; } }
</style>
