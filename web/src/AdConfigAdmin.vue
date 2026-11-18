<script setup>
import { computed, ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const cfg = ref(null),
  secrets = ref({}),
  clear = ref([]),
  error = ref(""),
  notice = ref(""),
  saving = ref(false),
  copied = ref("");
const controller = new AbortController();
const formats = [
  { id: "rewarded", label: "激励视频", hint: "大厅红包、红包雨大奖" },
  { id: "feed", label: "信息流", hint: "大厅消息流里的广告卡片" },
];
const COLORS = { csj: "#f65331", kuaishou: "#f2a51a", baidu: "#3b7bd8", gdt: "#2e9e62" };
const ids = computed(() => (cfg.value?.network_labels || []).map((n) => n.id));
const labels = computed(() =>
  Object.fromEntries((cfg.value?.network_labels || []).map((n) => [n.id, n.label])),
);
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try {
    cfg.value = await props.api("/admin/ad-config", { signal: controller.signal });
    secrets.value = {};
    clear.value = [];
  } catch (e) {
    fail(e);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    const c = cfg.value;
    cfg.value = await props.api("/admin/ad-config", {
      method: "PUT",
      body: {
        enabled: c.enabled,
        risk_block: c.risk_block,
        risk_window_hours: c.risk_window_hours,
        networks: c.networks,
        rewarded: c.rewarded,
        feed: c.feed,
        secrets: secrets.value,
        clear_secrets: clear.value,
      },
      signal: controller.signal,
    });
    secrets.value = {};
    clear.value = [];
    notice.value = "广告配置已保存，新的广告请求立即按此分配";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
const hasSecret = (id) => !!cfg.value.secret_source[id] || !!secrets.value[id];
// A network takes part in a format only when it is on, configured, and (rewarded) can verify callbacks.
function usable(id, format) {
  const n = cfg.value.networks[id];
  const code = format === "feed" ? n.feed : n.rewarded;
  if (!n.enabled || !n.app_id || !code) return false;
  if (format === "rewarded" && !hasSecret(id)) return false;
  return true;
}
function reason(id, format) {
  const n = cfg.value.networks[id];
  if (!n.enabled) return "未开启";
  if (!n.app_id) return "缺 AppID";
  if (!(format === "feed" ? n.feed : n.rewarded)) return "缺代码位";
  if (format === "rewarded" && !hasSecret(id)) return "缺回调密钥";
  return "";
}
function share(format, id) {
  const rule = cfg.value[format];
  if (!usable(id, format)) return 0;
  const total = ids.value
    .filter((n) => usable(n, format))
    .reduce((sum, n) => sum + (rule.weights[n] || 0), 0);
  return total ? Math.round(((rule.weights[id] || 0) * 1000) / total) / 10 : 0;
}
const readyCount = computed(() =>
  cfg.value ? ids.value.filter((id) => usable(id, "rewarded") || usable(id, "feed")).length : 0,
);
function move(format, index, step) {
  const order = cfg.value[format].order;
  const target = index + step;
  if (target < 0 || target >= order.length) return;
  [order[index], order[target]] = [order[target], order[index]];
}
async function copy(id, text) {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = id;
    setTimeout(() => (copied.value = ""), 1600);
  } catch {
    copied.value = "";
  }
}
const secretLabel = (id) =>
  ({ admin: "已设置", env: "服务器环境变量" })[cfg.value.secret_source[id]] || "未设置";
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <div class="ad-page">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="ad-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <section v-if="!cfg && !error" class="admin-card ad-card"><p class="ad-muted">正在读取广告配置…</p></section>

    <form v-if="cfg" @submit.prevent="save">
      <section class="admin-card ad-card">
        <header class="ad-card-head">
          <div>
            <h2>投放总控</h2>
            <p>每次拉广告前 App 先向服务器要“本次可以试哪几家”，按下方比例选首选，没填充时依次换下一家。保存即生效，不用发新版。</p>
          </div>
          <div class="ad-summary">
            <span><b>{{ readyCount }}</b> / {{ ids.length }} 家已就绪</span>
            <span :class="cfg.ad_mode === 'csj' ? 'ok' : 'warn'">{{ cfg.ad_mode === "csj" ? "正式广告模式" : "演示模式" }}</span>
          </div>
        </header>
        <div class="ad-toggles">
          <div class="ad-toggle-row">
            <div>
              <b>广告总开关</b>
              <p>关闭后所有用户都拿不到广告，红包视频和信息流都会暂停。</p>
            </div>
            <label class="ad-switch"><input v-model="cfg.enabled" type="checkbox" aria-label="广告总开关" /><span></span></label>
          </div>
          <div class="ad-toggle-row">
            <div>
              <b>命中风控不返回广告</b>
              <p>账户被限制，或近期设备核验结果为“复核/拒绝”时，按“暂无广告”处理；用户端只看到普通的无广告提示，金币不变。</p>
            </div>
            <div class="ad-toggle-side">
              <label v-if="cfg.risk_block" class="ad-window">
                近 <input v-model.number="cfg.risk_window_hours" type="number" min="1" max="720" aria-label="风控判断时间范围（小时）" /> 小时
              </label>
              <label class="ad-switch"><input v-model="cfg.risk_block" type="checkbox" aria-label="命中风控不返回广告" /><span></span></label>
            </div>
          </div>
        </div>
        <p v-if="cfg.ad_mode !== 'csj'" class="ad-warn">
          当前服务器是演示模式（ORANGE_AD_MODE={{ cfg.ad_mode }}），平台回调不会入账。
        </p>
      </section>

      <div class="ad-section-title">
        <h3>广告平台</h3>
        <p>激励视频必须填回调密钥才会被分配（金币只在平台服务器回调验签通过后到账）。修改 AppID 后，用户重新打开 App 才会生效。</p>
      </div>
      <div class="ad-networks">
        <section v-for="id in ids" :key="id" class="admin-card ad-card ad-net" :class="{ off: !cfg.networks[id].enabled }">
          <header class="ad-net-head">
            <div class="ad-net-name">
              <i :style="{ background: COLORS[id] }"></i>
              <b>{{ labels[id] }}</b>
              <span class="chip" :class="usable(id, 'rewarded') ? 'ok' : ''">激励视频{{ usable(id, "rewarded") ? "就绪" : "：" + reason(id, "rewarded") }}</span>
              <span class="chip" :class="usable(id, 'feed') ? 'ok' : ''">信息流{{ usable(id, "feed") ? "就绪" : "：" + reason(id, "feed") }}</span>
            </div>
            <label class="ad-switch"><input v-model="cfg.networks[id].enabled" type="checkbox" :aria-label="`开启${labels[id]}`" /><span></span></label>
          </header>
          <div class="ad-fields">
            <label>AppID<input v-model="cfg.networks[id].app_id" maxlength="32" placeholder="平台后台的应用 ID" /></label>
            <label>
              回调密钥
              <input
                v-model="secrets[id]"
                type="password"
                autocomplete="new-password"
                maxlength="128"
                :placeholder="cfg.secret_source[id] ? `${secretLabel(id)}，留空保持不变` : '平台生成的安全密钥'"
              />
            </label>
            <label>激励视频代码位<input v-model="cfg.networks[id].rewarded" maxlength="32" placeholder="不填则不参与激励视频" /></label>
            <label>信息流代码位<input v-model="cfg.networks[id].feed" maxlength="32" placeholder="模板信息流，不填则不参与" /></label>
          </div>
          <div class="ad-callback">
            <div class="ad-callback-head">
              <b>服务器回调地址</b>
              <button type="button" class="ad-copy" @click="copy(id, cfg.callbacks[id].url)">
                <Icon :name="copied === id ? 'check' : 'list'" :size="13" />{{ copied === id ? "已复制" : "复制地址" }}
              </button>
            </div>
            <code>{{ cfg.callbacks[id].url }}</code>
            <p>{{ cfg.callbacks[id].how }}</p>
          </div>
          <label v-if="cfg.secret_source[id] === 'admin'" class="ad-clear">
            <input v-model="clear" type="checkbox" :value="id" />保存时清除后台保存的回调密钥
          </label>
        </section>
      </div>

      <div class="ad-section-title">
        <h3>流量分配</h3>
        <p>按概率：右侧百分比是“首选该平台”的实际概率，权重 0 的平台不参与，首选没填充时按权重从高到低换下一家。按顺序：始终从第一家开始依次尝试。</p>
      </div>
      <div class="ad-rules">
        <section v-for="f in formats" :key="f.id" class="admin-card ad-card">
          <header class="ad-rule-head">
            <div><b>{{ f.label }}</b><small>{{ f.hint }}</small></div>
            <div class="ad-seg" role="radiogroup" :aria-label="`${f.label}分配方式`">
              <button type="button" role="radio" :aria-checked="cfg[f.id].mode === 'weighted'" :class="{ on: cfg[f.id].mode === 'weighted' }" @click="cfg[f.id].mode = 'weighted'">按概率</button>
              <button type="button" role="radio" :aria-checked="cfg[f.id].mode === 'ordered'" :class="{ on: cfg[f.id].mode === 'ordered' }" @click="cfg[f.id].mode = 'ordered'">按顺序</button>
            </div>
          </header>
          <div v-if="cfg[f.id].mode === 'weighted'" class="ad-sliders">
            <div v-for="id in ids" :key="id" class="ad-slider" :class="{ dim: !usable(id, f.id) }">
              <span class="ad-slider-name"><i :style="{ background: COLORS[id] }"></i>{{ labels[id] }}</span>
              <input
                v-model.number="cfg[f.id].weights[id]"
                type="range"
                min="0"
                max="100"
                step="5"
                :style="{ '--fill': `${cfg[f.id].weights[id] || 0}%`, '--tint': COLORS[id] }"
                :aria-label="`${labels[id]} ${f.label}权重`"
              />
              <output>{{ cfg[f.id].weights[id] || 0 }}</output>
              <em>{{ usable(id, f.id) ? `${share(f.id, id)}%` : reason(id, f.id) }}</em>
            </div>
            <div class="ad-share-bar" aria-hidden="true">
              <i v-for="id in ids" v-show="share(f.id, id) > 0" :key="id" :style="{ width: `${share(f.id, id)}%`, background: COLORS[id] }"></i>
            </div>
          </div>
          <ol v-else class="ad-order">
            <li v-for="(id, index) in cfg[f.id].order" :key="id" :class="{ dim: !usable(id, f.id) }">
              <span class="ad-slider-name">
                <em>{{ index + 1 }}</em><i :style="{ background: COLORS[id] }"></i>{{ labels[id] }}
                <small v-if="!usable(id, f.id)">（{{ reason(id, f.id) }}，跳过）</small>
              </span>
              <span class="ad-order-btns">
                <button type="button" :disabled="index === 0" :aria-label="`${labels[id]}上移`" @click="move(f.id, index, -1)">↑</button>
                <button type="button" :disabled="index === cfg[f.id].order.length - 1" :aria-label="`${labels[id]}下移`" @click="move(f.id, index, 1)">↓</button>
              </span>
            </li>
          </ol>
        </section>
      </div>

      <div class="ad-savebar">
        <span>修改后点击保存，新的广告请求立即生效</span>
        <button class="primary" :disabled="saving"><Icon name="check" :size="17" />{{ saving ? "保存中…" : "保存广告配置" }}</button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.ad-card { padding: 18px 20px; margin-bottom: 12px; }
.ad-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.ad-card-head { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 8px; }
.ad-card-head p, .ad-muted { color: #9a8274; font-size: 14px; line-height: 1.75; margin: 0; max-width: 760px; }
.ad-summary { display: flex; gap: 8px; flex-shrink: 0; }
.ad-summary span { font-size: 13.5px; color: #8a6f61; background: #f7efe8; border-radius: 999px; padding: 6px 12px; white-space: nowrap; }
.ad-summary b { color: #f65331; font-size: 14px; }
.ad-summary .ok { background: #eefaf1; color: #2f8a4c; }
.ad-summary .warn { background: #fff4e5; color: #a5520b; }
.ad-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 12px 14px; margin-bottom: 14px; font-size: 14px; }
.ad-warn { background: #fff4e5; color: #a5520b; border-radius: 10px; padding: 10px 14px; font-size: 14px; margin: 14px 0 0; }

.ad-toggle-row { display: flex; justify-content: space-between; align-items: center; gap: 24px; border-top: 1px solid #f2e8d9; padding: 18px 0; }
.ad-toggle-row:last-child { padding-bottom: 4px; }
.ad-toggle-row b { font-size: 14px; color: #6f4b33; }
.ad-toggle-row p { font-size: 14px; color: #a8907f; margin: 6px 0 0; line-height: 1.65; max-width: 680px; }
.ad-toggle-side { display: flex; align-items: center; gap: 18px; flex-shrink: 0; }
.ad-window { display: flex; align-items: center; gap: 6px; font-size: 14px; color: #7d685c; margin: 0; font-weight: 400; white-space: nowrap; }
.ad-window input { width: 72px; margin: 0; padding: 7px 10px; text-align: center; }

.ad-switch { position: relative; display: block; width: 42px; height: 24px; flex-shrink: 0; margin: 0; }
.ad-switch input { opacity: 0; width: 100%; height: 100%; position: absolute; inset: 0; margin: 0; padding: 0; cursor: pointer; }
.ad-switch > span { pointer-events: none; display: block; height: 24px; border-radius: 30px; background: #dfd4c3; transition: background 0.2s; }
.ad-switch > span::after { content: ""; position: absolute; height: 18px; width: 18px; top: 3px; left: 3px; background: #fff; border-radius: 50%; transition: transform 0.2s; box-shadow: 0 1px 3px #98714824; }
.ad-switch input:checked + span { background: #ec985b; }
.ad-switch input:checked + span::after { transform: translateX(18px); }
.ad-switch input:focus-visible + span { outline: 2px solid #d18448; outline-offset: 3px; }

.ad-section-title { display: flex; align-items: baseline; gap: 14px; margin: 6px 2px 8px; flex-wrap: wrap; }
.ad-section-title h3 { font-size: 16px; color: #5d4030; margin: 0; }
.ad-section-title p { font-size: 14px; color: #a8907f; margin: 0; line-height: 1.6; flex: 1; min-width: 280px; }

.ad-networks, .ad-rules { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0 12px; }
.ad-net { display: grid; gap: 16px; align-content: start; }
.ad-net.off { background: #fcf9f5; }
.ad-net.off .ad-fields, .ad-net.off .ad-callback { opacity: 0.72; }
.ad-net-head { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
.ad-net-name { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; min-width: 0; }
.ad-net-name > i, .ad-slider-name i { width: 10px; height: 10px; border-radius: 3px; display: inline-block; flex-shrink: 0; }
.ad-net-name b { font-size: 16px; color: #4f3627; margin-right: 4px; }
.chip { font-size: 13px; color: #a08779; background: #f7efe8; border-radius: 999px; padding: 3px 9px; white-space: nowrap; }
.chip.ok { color: #2f8a4c; background: #eefaf1; }
.ad-fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px 14px; }
.ad-fields label { margin: 0; display: grid; gap: 6px; font-size: 13.5px; }
.ad-fields input { margin: 0; padding: 10px 12px; font-size: 14px; border-radius: 10px; }
.ad-callback { background: #fbf6f1; border: 0; border-radius: 12px; padding: 12px 14px; display: grid; gap: 8px; }
.ad-callback-head { display: flex; justify-content: space-between; align-items: center; font-size: 14px; color: #6f4b33; }
.ad-copy { display: inline-flex; gap: 5px; align-items: center; border: 1px solid #ecdccd; background: #fff; color: #b0623f; border-radius: 8px; padding: 4px 10px; font-size: 13.5px; cursor: pointer; }
.ad-copy:hover { border-color: #ef916b; }
.ad-callback code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 13.5px; line-height: 1.6; word-break: break-all; color: #3d2b22; background: #fff; border-radius: 8px; padding: 9px 11px; user-select: all; }
.ad-callback p { margin: 0; font-size: 13.5px; color: #9a8274; line-height: 1.65; }
.ad-clear { display: flex; gap: 6px; align-items: center; margin: 0; font-size: 13.5px; color: #b0533a; font-weight: 400; }
.ad-clear input { width: auto; margin: 0; padding: 0; }

.ad-rule-head { display: flex; justify-content: space-between; gap: 12px; align-items: center; margin-bottom: 18px; }
.ad-rule-head > div:first-child { display: grid; gap: 3px; }
.ad-rule-head b { font-size: 15px; color: #4f3627; }
.ad-rule-head small { color: #a8907f; font-size: 13.5px; }
.ad-seg { display: flex; background: #f7efe8; border-radius: 999px; padding: 3px; flex-shrink: 0; }
.ad-seg button { border: 0; background: transparent; border-radius: 999px; padding: 6px 14px; font-size: 14px; color: #8a6f61; cursor: pointer; }
.ad-seg button.on { background: #fff; color: #f65331; font-weight: 600; box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08); }
.ad-sliders { display: grid; gap: 14px; }
.ad-slider { display: grid; grid-template-columns: 78px minmax(0, 1fr) 36px 92px; gap: 14px; align-items: center; font-size: 14px; }
.ad-slider-name { display: flex; align-items: center; gap: 7px; color: #5d4030; }
.ad-slider input[type="range"] {
  -webkit-appearance: none; appearance: none; width: 100%; height: 6px; margin: 0; padding: 0; border: 0; border-radius: 3px; box-shadow: none;
  background: linear-gradient(90deg, var(--tint) var(--fill), #efe5dc var(--fill));
}
.ad-slider input[type="range"]::-webkit-slider-thumb { -webkit-appearance: none; width: 18px; height: 18px; border-radius: 50%; background: #fff; border: 2px solid var(--tint); box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15); cursor: pointer; }
.ad-slider input[type="range"]::-moz-range-thumb { width: 16px; height: 16px; border-radius: 50%; background: #fff; border: 2px solid var(--tint); cursor: pointer; }
.ad-slider output { text-align: right; color: #7d685c; font-variant-numeric: tabular-nums; }
.ad-slider em { font-style: normal; text-align: right; font-weight: 600; color: #4f3627; font-variant-numeric: tabular-nums; white-space: nowrap; }
.ad-slider.dim .ad-slider-name, .ad-slider.dim output { color: #c4b2a6; }
.ad-slider.dim em { color: #c4b2a6; font-weight: 400; font-size: 13.5px; }
.ad-slider.dim input[type="range"] { filter: grayscale(0.85); opacity: 0.55; }
.ad-share-bar { display: flex; height: 8px; border-radius: 4px; overflow: hidden; background: #f3e9df; gap: 2px; margin-top: 4px; }
.ad-share-bar i { display: block; height: 100%; }
.ad-order { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
.ad-order li { display: flex; justify-content: space-between; align-items: center; background: #fbf6f1; border-radius: 12px; padding: 10px 14px; font-size: 14px; }
.ad-order li em { font-style: normal; width: 20px; height: 20px; border-radius: 50%; background: #fff; color: #b0623f; display: inline-grid; place-items: center; font-size: 13.5px; font-weight: 600; }
.ad-order li.dim { color: #b9a497; }
.ad-order small { color: #b9a497; }
.ad-order-btns { display: flex; gap: 6px; }
.ad-order-btns button { width: 30px; height: 28px; border: 1px solid #ecdccd; background: #fff; border-radius: 8px; cursor: pointer; color: #8a6f61; }
.ad-order-btns button:disabled { opacity: 0.35; cursor: default; }

.ad-savebar {
  position: sticky; bottom: 16px; z-index: 5; display: flex; justify-content: space-between; align-items: center; gap: 16px;
  background: #fffdfaf2; backdrop-filter: blur(6px); border: 1px solid #eee4d7; border-radius: 16px; padding: 12px 14px 12px 22px;
  box-shadow: 0 8px 24px rgba(120, 80, 50, 0.08);
}
.ad-savebar span { font-size: 14px; color: #a8907f; }
.ad-savebar .primary { width: auto; margin: 0; padding: 11px 22px; }

@media (max-width: 1180px) {
  .ad-networks, .ad-rules { grid-template-columns: 1fr; }
}
@media (max-width: 640px) {
  .ad-card { padding: 18px 16px; }
  .ad-card-head { flex-direction: column; }
  .ad-toggle-row { align-items: flex-start; }
  .ad-toggle-side { flex-direction: column-reverse; align-items: flex-end; gap: 10px; }
  .ad-fields { grid-template-columns: 1fr; }
  .ad-slider { grid-template-columns: 64px minmax(0, 1fr) 30px 70px; gap: 10px; }
  .ad-savebar span { display: none; }
  .ad-savebar .primary { width: 100%; justify-content: center; }
}
</style>
