<script setup>
import { computed, ref, onMounted, onUnmounted, watch } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const stats = ref(null),
  detail = ref(null),
  error = ref(""),
  days = ref(7),
  format = ref(""),
  metric = ref("show"),
  hover = ref(-1),
  filters = ref({ network: "", format: "", event: "", user_id: "", session: "" }),
  page = ref(1);
const controller = new AbortController();
const COLORS = { csj: "#f65331", kuaishou: "#f2a51a", baidu: "#3b7bd8", gdt: "#2e9e62" };
const EVENT_LABELS = {
  plan: "服务器分配", risk_block: "风控拦截", empty: "无可用平台", request: "发起请求", fill: "填充成功",
  no_fill: "无填充", show: "展示", click: "点击", close: "关闭", reward: "客户端奖励", skip: "跳过",
  error: "出错", timeout: "超时", callback_ok: "回调通过", callback_retry: "回调重试", callback_reject: "回调拒绝",
};
const METRICS = [
  { id: "request", label: "请求" }, { id: "fill", label: "填充" }, { id: "show", label: "展示" },
  { id: "click", label: "点击" }, { id: "callback_ok", label: "回调通过" },
];
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
const query = (o) =>
  new URLSearchParams(Object.entries(o).filter(([, v]) => v !== "" && v != null)).toString();
async function loadStats() {
  stats.value = await props.api(`/admin/ad-stats?${query({ days: days.value, format: format.value })}`, {
    signal: controller.signal,
  });
}
async function loadDetail(p = page.value) {
  page.value = p;
  detail.value = await props.api(`/admin/ad-events?${query({ ...filters.value, page: p })}`, {
    signal: controller.signal,
  });
}
async function refresh() {
  error.value = "";
  try {
    await Promise.all([loadStats(), loadDetail(1)]);
  } catch (e) {
    fail(e);
  }
}
watch([days, format], () => loadStats().catch(fail));
const t = (totals, key) => totals?.[key] || 0;
const pct = (v) => (v == null ? "—" : `${v}%`);
const overall = computed(() => stats.value?.overall || { totals: {}, rates: {} });
// Apps before 0.3.21 never report request/fill/show; only server-side numbers exist for them.
const clientSilent = computed(() => {
  const o = overall.value.totals;
  return t(o, "plan") > 0 && t(o, "request") + t(o, "fill") + t(o, "show") === 0;
});
const tiles = computed(() => {
  const o = overall.value;
  return [
    { label: "请求", value: t(o.totals, "request") },
    { label: "填充率", value: pct(o.rates.fill), sub: `${t(o.totals, "fill")} 次填充` },
    { label: "展示", value: t(o.totals, "show"), sub: `${stats.value?.show_users || 0} 人看过` },
    { label: "点击率", value: pct(o.rates.click), sub: `${t(o.totals, "click")} 次点击` },
    { label: "完成率", value: pct(o.rates.complete), sub: `回调通过 ${t(o.totals, "callback_ok")} ÷ 分配 ${t(o.totals, "plan")}` },
    { label: "回调成功率", value: pct(o.rates.callback), sub: `${t(o.totals, "callback_ok")} 次通过 · ${t(o.totals, "callback_reject")} 次拒绝` },
    { label: "风控拦截", value: t(o.totals, "risk_block"), sub: `占分配 ${pct(o.rates.risk)}` },
    { label: "广告发放金币", value: stats.value?.rewarded_coins || 0, sub: "平台回调确认后入账" },
  ];
});
// ---- line chart: one series per network for the chosen metric ----
const W = 720, H = 240, L = 40, R = 14, T = 16, B = 34;
const series = computed(() => {
  if (!stats.value) return [];
  return stats.value.networks.map((n) => ({
    id: n.network,
    label: n.label,
    values: stats.value.series.map((d) => t(d[n.network], metric.value)),
  }));
});
const ceiling = computed(() => {
  const max = Math.max(1, ...series.value.flatMap((s) => s.values));
  const step = Math.pow(10, Math.floor(Math.log10(max)));
  return Math.ceil(max / step) * step;
});
const x = (i) => {
  const n = stats.value?.days.length || 1;
  return n === 1 ? L + (W - L - R) / 2 : L + (i * (W - L - R)) / (n - 1);
};
const y = (v) => T + (H - T - B) * (1 - v / ceiling.value);
const path = (values) => values.map((v, i) => `${i ? "L" : "M"}${x(i)},${y(v)}`).join(" ");
const ticks = computed(() => [0, 0.25, 0.5, 0.75, 1].map((f) => Math.round(ceiling.value * f)));
const labelEvery = computed(() => Math.max(1, Math.ceil((stats.value?.days.length || 1) / 8)));
// ---- share of shows ----
const shares = computed(() => {
  if (!stats.value) return [];
  const total = stats.value.networks.reduce((s, n) => s + t(n.totals, "show"), 0);
  return stats.value.networks.map((n) => ({
    id: n.network,
    label: n.label,
    value: t(n.totals, "show"),
    pct: total ? (t(n.totals, "show") * 100) / total : 0,
  }));
});
// ---- funnel ----
const funnel = computed(() => {
  const o = overall.value.totals;
  const steps = [
    ["服务器分配", t(o, "plan")], ["发起请求", t(o, "request")], ["填充", t(o, "fill")],
    ["展示", t(o, "show")], ["点击", t(o, "click")], ["回调通过", t(o, "callback_ok")],
  ];
  const top = Math.max(1, ...steps.map((s) => s[1]));
  return steps.map(([label, value]) => ({ label, value, width: Math.max(2, (value * 100) / top) }));
});
const time = (iso) => new Date(iso).toLocaleString("zh-CN", { hour12: false });
const label = (id) => stats.value?.networks.find((n) => n.network === id)?.label || (id === "-" ? "—" : id);
function showSession(id) {
  filters.value = { network: "", format: "", event: "", user_id: "", session: id };
  loadDetail(1).catch(fail);
}
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <div class="ad-stats">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <header class="st-head">
      <div>
                <p>“请求/填充/展示/点击”来自 App 上报，“分配/风控拦截/回调”来自服务器。成功率：填充率=填充÷请求，点击率=点击÷展示，回调成功率=回调通过÷展示，完成率=回调通过÷服务器分配（仅激励视频有回调）。</p>
      </div>
      <div class="st-filters">
        <select v-model.number="days" aria-label="时间范围">
          <option :value="1">今天</option><option :value="7">近 7 天</option>
          <option :value="30">近 30 天</option><option :value="90">近 90 天</option>
        </select>
        <select v-model="format" aria-label="广告形式">
          <option value="">全部形式</option><option value="rewarded">激励视频</option><option value="feed">信息流</option>
        </select>
        <button type="button" class="outline" @click="refresh"><Icon name="refresh" :size="14" />刷新</button>
      </div>
    </header>
    <p v-if="!stats && !error" class="st-muted">正在统计…</p>
    <p v-if="clientSilent" class="st-hint">
      所选时段只有服务器数据：用户目前使用的 App 版本（0.3.20 及以前）不上报请求、填充、展示、点击，升级到 0.3.22 后才会有这几项。在此之前请看“完成率”（回调通过 ÷ 服务器分配），它完全由服务器统计。
    </p>
    <template v-if="stats">
      <div class="st-tiles">
        <div v-for="tile in tiles" :key="tile.label" class="st-tile">
          <span>{{ tile.label }}</span><b>{{ tile.value }}</b><small v-if="tile.sub">{{ tile.sub }}</small>
        </div>
      </div>

      <div class="st-grid">
        <article class="st-card wide">
          <header>
            <b>每日趋势</b>
            <div class="st-seg" role="radiogroup" aria-label="趋势指标">
              <button v-for="m in METRICS" :key="m.id" type="button" :class="{ on: metric === m.id }" @click="metric = m.id">{{ m.label }}</button>
            </div>
          </header>
          <svg :viewBox="`0 0 ${W} ${H}`" class="st-chart" role="img" aria-label="各平台每日趋势" @mouseleave="hover = -1">
            <g v-for="v in ticks" :key="v">
              <line :x1="L" :x2="W - R" :y1="y(v)" :y2="y(v)" class="grid" />
              <text :x="L - 6" :y="y(v) + 4" text-anchor="end" class="axis">{{ v }}</text>
            </g>
            <text v-for="(d, i) in stats.days" v-show="i % labelEvery === 0 || i === stats.days.length - 1" :key="d" :x="x(i)" :y="H - 12" text-anchor="middle" class="axis">{{ d.slice(5) }}</text>
            <path v-for="s in series" :key="s.id" :d="path(s.values)" :stroke="COLORS[s.id]" class="line" />
            <g v-for="s in series" :key="`${s.id}-dots`">
              <circle v-for="(v, i) in s.values" :key="i" :cx="x(i)" :cy="y(v)" :r="hover === i ? 4 : 2.5" :fill="COLORS[s.id]" />
            </g>
            <rect v-for="(d, i) in stats.days" :key="`h${d}`" :x="x(i) - (W - L - R) / Math.max(1, stats.days.length) / 2" :y="T" :width="(W - L - R) / Math.max(1, stats.days.length)" :height="H - T - B" fill="transparent" @mouseenter="hover = i" />
            <line v-if="hover >= 0" :x1="x(hover)" :x2="x(hover)" :y1="T" :y2="H - B" class="cursor" />
          </svg>
          <div class="st-legend">
            <span v-for="s in series" :key="s.id"><i :style="{ background: COLORS[s.id] }"></i>{{ s.label }}<b v-if="hover >= 0">{{ s.values[hover] }}</b></span>
            <span v-if="hover >= 0" class="st-muted">{{ stats.days[hover] }}</span>
          </div>
        </article>

        <article class="st-card">
          <header><b>展示占比</b></header>
          <div class="st-stack" role="img" aria-label="各平台展示占比">
            <i v-for="s in shares" v-show="s.pct > 0" :key="s.id" :style="{ width: `${s.pct}%`, background: COLORS[s.id] }" :title="`${s.label} ${s.value}`"></i>
          </div>
          <ul class="st-share">
            <li v-for="s in shares" :key="s.id"><span><i :style="{ background: COLORS[s.id] }"></i>{{ s.label }}</span><b>{{ s.value }}</b><em>{{ s.pct.toFixed(1) }}%</em></li>
          </ul>
          <header class="st-sub"><b>转化漏斗</b></header>
          <div class="st-funnel">
            <div v-for="f in funnel" :key="f.label"><span>{{ f.label }}</span><i :style="{ width: `${f.width}%` }"></i><b>{{ f.value }}</b></div>
          </div>
        </article>
      </div>

      <article class="st-card">
        <header><b>各平台成功率</b></header>
        <div class="st-table-wrap">
          <table class="st-table">
            <thead><tr><th>平台</th><th>分配</th><th>请求</th><th>填充</th><th>填充率</th><th>展示</th><th>展示率</th><th>点击</th><th>点击率</th><th>回调通过</th><th>回调拒绝</th><th>回调成功率</th><th>完成率</th><th>出错</th></tr></thead>
            <tbody>
              <tr v-for="n in stats.networks" :key="n.network">
                <td><i class="dot" :style="{ background: COLORS[n.network] }"></i>{{ n.label }}</td>
                <td>{{ t(n.totals, "plan") }}</td><td>{{ t(n.totals, "request") }}</td><td>{{ t(n.totals, "fill") }}</td>
                <td><span class="bar"><i :style="{ width: `${n.rates.fill || 0}%`, background: COLORS[n.network] }"></i></span>{{ pct(n.rates.fill) }}</td>
                <td>{{ t(n.totals, "show") }}</td><td>{{ pct(n.rates.show) }}</td><td>{{ t(n.totals, "click") }}</td><td>{{ pct(n.rates.click) }}</td>
                <td>{{ t(n.totals, "callback_ok") }}</td><td>{{ t(n.totals, "callback_reject") }}</td>
                <td><span class="bar"><i :style="{ width: `${Math.min(100, n.rates.callback || 0)}%`, background: COLORS[n.network] }"></i></span>{{ pct(n.rates.callback) }}</td>
                <td>{{ pct(n.rates.complete) }}</td>
                <td>{{ t(n.totals, "error") + t(n.totals, "timeout") }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </article>

      <article class="st-card">
        <header><b>失败原因 Top 20</b></header>
        <p v-if="!stats.errors.length" class="st-muted">所选时段没有失败记录。</p>
        <div v-else class="st-table-wrap">
          <table class="st-table">
            <thead><tr><th>平台</th><th>形式</th><th>类型</th><th>错误码</th><th>说明</th><th>次数</th></tr></thead>
            <tbody>
              <tr v-for="(e, i) in stats.errors" :key="i">
                <td>{{ label(e.network) }}</td><td>{{ e.format === "feed" ? "信息流" : "激励视频" }}</td>
                <td>{{ EVENT_LABELS[e.event] || e.event }}</td><td>{{ e.code || "—" }}</td><td class="msg">{{ e.message || "—" }}</td><td>{{ e.count }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </article>
    </template>

    <article v-if="detail" class="st-card">
      <header><b>广告明细</b><small class="st-muted">共 {{ detail.total }} 条，保留 90 天</small></header>
      <form class="st-detail-filters" @submit.prevent="loadDetail(1).catch(fail)">
        <select v-model="filters.network" aria-label="平台"><option value="">全部平台</option><option v-for="n in stats?.networks || []" :key="n.network" :value="n.network">{{ n.label }}</option></select>
        <select v-model="filters.format" aria-label="形式"><option value="">全部形式</option><option value="rewarded">激励视频</option><option value="feed">信息流</option></select>
        <select v-model="filters.event" aria-label="事件"><option value="">全部事件</option><option v-for="e in detail.events" :key="e" :value="e">{{ EVENT_LABELS[e] || e }}</option></select>
        <input v-model="filters.user_id" inputmode="numeric" placeholder="用户 ID" aria-label="用户 ID" />
        <input v-model="filters.session" placeholder="奖励编号" aria-label="奖励编号" />
        <button class="outline"><Icon name="search" :size="14" />筛选</button>
      </form>
      <div class="st-table-wrap">
        <table class="st-table">
          <thead><tr><th>时间</th><th>用户</th><th>平台</th><th>形式</th><th>事件</th><th>来源</th><th>代码位</th><th>错误码 / 说明</th><th>奖励编号</th></tr></thead>
          <tbody>
            <tr v-for="row in detail.items" :key="row.id" :class="{ warn: ['risk_block', 'callback_reject', 'error'].includes(row.event) }">
              <td>{{ time(row.created_at) }}</td><td>{{ row.user_id ?? "—" }}</td><td>{{ label(row.network) }}</td>
              <td>{{ row.format === "feed" ? "信息流" : "激励视频" }}</td><td>{{ EVENT_LABELS[row.event] || row.event }}</td>
              <td>{{ { app: "App", server: "服务器", callback: "平台回调" }[row.source] || row.source }}</td>
              <td>{{ row.placement || "—" }}</td><td class="msg">{{ [row.code, row.message].filter(Boolean).join(" ") || "—" }}</td>
              <td><button v-if="row.session_id" type="button" class="linkish" @click="showSession(row.session_id)">{{ row.session_id.slice(0, 8) }}</button><span v-else>—</span></td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="st-pager">
        <button type="button" class="outline" :disabled="detail.page <= 1" @click="loadDetail(detail.page - 1).catch(fail)">上一页</button>
        <span>第 {{ detail.page }} / {{ Math.max(1, Math.ceil(detail.total / detail.page_size)) }} 页</span>
        <button type="button" class="outline" :disabled="detail.page * detail.page_size >= detail.total" @click="loadDetail(detail.page + 1).catch(fail)">下一页</button>
      </div>
    </article>
  </div>
</template>

<style scoped>
.ad-stats h2 { margin: 0 0 6px; }
.st-head { margin-bottom: 4px; display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; flex-wrap: wrap; }
.st-head p, .st-muted { color: #9a8274; font-size: 14px; line-height: 1.7; margin: 0; max-width: 720px; }
.st-filters { display: flex; gap: 8px; flex-wrap: wrap; }
.st-tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 14px; margin: 18px 0; }
.st-tile { border: 0; border-radius: 16px; padding: 16px 18px; display: grid; gap: 4px; background: #fff; }
.st-tile span { font-size: 13.5px; color: #9a8274; }
.st-tile b { font-size: 22px; color: #3d2b22; font-variant-numeric: tabular-nums; }
.st-tile small { font-size: 13.5px; color: #b49c8e; }
.st-grid { display: grid; grid-template-columns: minmax(0, 2fr) minmax(300px, 1fr); gap: 0 12px; }
.st-card { border: 0; border-radius: 17px; padding: 16px 18px; margin-bottom: 12px; background: #fffdfa; min-width: 0; }
.st-card > header { display: flex; justify-content: space-between; align-items: center; gap: 10px; margin-bottom: 10px; flex-wrap: wrap; }
.st-sub { margin-top: 16px; }
.st-seg { display: flex; gap: 4px; background: #f7efe8; border-radius: 999px; padding: 3px; }
.st-seg button { border: 0; background: transparent; border-radius: 999px; padding: 4px 10px; font-size: 13.5px; color: #8a6f61; cursor: pointer; }
.st-seg button.on { background: #fff; color: #f65331; font-weight: 600; box-shadow: 0 1px 3px rgba(0,0,0,.08); }
.st-chart { width: 100%; height: auto; display: block; }
.st-chart .grid { stroke: #f3e8df; }
.st-chart .axis { fill: #b49c8e; font-size: 13px; }
.st-chart .line { fill: none; stroke-width: 2.2; stroke-linejoin: round; stroke-linecap: round; }
.st-chart .cursor { stroke: #d9c7ba; stroke-dasharray: 3 3; }
.st-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 13.5px; color: #6d5547; margin-top: 6px; }
.st-legend span { display: flex; gap: 5px; align-items: center; }
.st-legend i, .st-share i, .dot { width: 9px; height: 9px; border-radius: 3px; display: inline-block; }
.dot { margin-right: 6px; }
.st-stack { display: flex; height: 14px; border-radius: 7px; overflow: hidden; background: #f7efe8; gap: 2px; }
.st-stack i { display: block; height: 100%; }
.st-share { list-style: none; padding: 0; margin: 10px 0 0; display: grid; gap: 6px; font-size: 14px; }
.st-share li { display: grid; grid-template-columns: 1fr auto 56px; gap: 8px; align-items: center; }
.st-share li span { display: flex; gap: 6px; align-items: center; }
.st-share em { font-style: normal; color: #9a8274; text-align: right; }
.st-funnel { display: grid; gap: 6px; font-size: 13.5px; }
.st-funnel div { display: grid; grid-template-columns: 70px 1fr 48px; gap: 8px; align-items: center; }
.st-funnel i { display: block; height: 12px; border-radius: 6px; background: linear-gradient(90deg, #ff9a62, #f65331); }
.st-funnel b { text-align: right; font-variant-numeric: tabular-nums; }
.st-table-wrap { overflow-x: auto; }
.st-table { width: 100%; border-collapse: collapse; font-size: 14px; white-space: nowrap; }
.st-table th { text-align: left; color: #9a8274; font-weight: 500; padding: 8px 10px; border-bottom: 1px solid #f1e3d8; }
.st-table td { padding: 8px 10px; border-bottom: 1px solid #f8f0ea; font-variant-numeric: tabular-nums; }
.st-table td.msg { white-space: normal; min-width: 160px; max-width: 320px; color: #6d5547; }
.st-table tr.warn td { background: #fff8f3; }
.bar { display: inline-block; width: 46px; height: 6px; border-radius: 3px; background: #f3e8df; margin-right: 6px; vertical-align: middle; overflow: hidden; }
.bar i { display: block; height: 100%; }
.st-detail-filters { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 10px; }
.st-detail-filters input { width: 120px; }
.st-pager { display: flex; gap: 10px; align-items: center; justify-content: flex-end; margin-top: 10px; font-size: 14px; color: #8a6f61; }
.linkish { border: 0; background: none; color: #f65331; cursor: pointer; padding: 0; font: inherit; text-decoration: underline; }
@media (max-width: 860px) { .st-grid { grid-template-columns: 1fr; } }
.st-filters select, .st-detail-filters select { width: auto; padding: 8px 10px; font-size: 14px; }
.st-detail-filters input { padding: 8px 10px; font-size: 14px; }
.st-hint { background: #fff4e5; color: #8a5a1c; border-radius: 10px; padding: 10px 14px; font-size: 13.5px; line-height: 1.7; margin: 12px 0 0; }
</style>
