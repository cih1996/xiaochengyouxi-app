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
// Share of first attempts each network gets under the current sliders (only usable networks count).
function usable(id, format) {
  const n = cfg.value.networks[id];
  const code = format === "feed" ? n.feed : n.rewarded;
  if (!n.enabled || !n.app_id || !code) return false;
  if (format === "rewarded" && !cfg.value.secret_source[id] && !secrets.value[id]) return false;
  return true;
}
function share(format, id) {
  const rule = cfg.value[format];
  if (!usable(id, format)) return 0;
  const total = Object.keys(cfg.value.networks)
    .filter((n) => usable(n, format))
    .reduce((sum, n) => sum + (rule.weights[n] || 0), 0);
  return total ? Math.round(((rule.weights[id] || 0) * 1000) / total) / 10 : 0;
}
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
  ({ admin: "已在后台设置", env: "使用服务器环境变量" })[cfg.value.secret_source[id]] || "未设置";
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <section class="admin-card ad-admin">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="ad-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <p v-if="!cfg && !error" class="ad-muted">正在读取广告配置…</p>
    <form v-if="cfg" @submit.prevent="save">
      <header class="ad-head">
        <div>
                    <p>
            每次拉广告前，App 先向服务器要“本次可以试哪几家”。按下面的比例选第一家，没填充时依次换下一家。
            改完保存即生效，不用发新版。
          </p>
        </div>
      </header>

      <div class="ad-switches">
        <label class="ad-switch">
          <input v-model="cfg.enabled" type="checkbox" />
          <span><b>广告总开关</b><small>关闭后所有用户都拿不到广告（红包和信息流都会暂停）</small></span>
        </label>
        <label class="ad-switch">
          <input v-model="cfg.risk_block" type="checkbox" />
          <span>
            <b>命中风控不返回广告</b>
            <small>账户被限制、或近期设备风控为“复核/拒绝”时，直接按“暂无广告”处理；用户端没有任何提示，金币不变。</small>
          </span>
        </label>
        <label v-if="cfg.risk_block" class="ad-inline">
          风控判断时间范围
          <input v-model.number="cfg.risk_window_hours" type="number" min="1" max="720" />
          小时内
        </label>
      </div>
      <p v-if="cfg.ad_mode !== 'csj'" class="ad-warn">
        当前服务器是演示广告模式（ORANGE_AD_MODE={{ cfg.ad_mode }}），平台回调不会入账。
      </p>

      <h3>广告平台</h3>
      <div class="ad-networks">
        <article v-for="id in ids" :key="id" class="ad-net" :class="{ off: !cfg.networks[id].enabled }">
          <header>
            <label class="ad-switch compact">
              <input v-model="cfg.networks[id].enabled" type="checkbox" />
              <b>{{ labels[id] }}</b>
            </label>
            <span class="ad-tag" :class="cfg.secret_source[id] ? 'ok' : ''">回调密钥：{{ secretLabel(id) }}</span>
          </header>
          <div class="ad-fields">
            <label>AppID<input v-model="cfg.networks[id].app_id" maxlength="32" placeholder="平台后台的应用 ID" /></label>
            <label>激励视频代码位<input v-model="cfg.networks[id].rewarded" maxlength="32" placeholder="不填则不参与激励视频" /></label>
            <label>信息流代码位<input v-model="cfg.networks[id].feed" maxlength="32" placeholder="模板信息流，不填则不参与" /></label>
            <label>
              回调密钥
              <input
                v-model="secrets[id]"
                type="password"
                autocomplete="new-password"
                maxlength="128"
                :placeholder="cfg.secret_source[id] ? '已设置，留空保持不变' : '平台生成的安全密钥'"
              />
            </label>
          </div>
          <div class="ad-callback">
            <div class="ad-callback-head">
              <b>服务器回调地址</b>
              <button type="button" class="outline small" @click="copy(id, cfg.callbacks[id].url)">
                <Icon :name="copied === id ? 'check' : 'list'" :size="13" />{{ copied === id ? "已复制" : "复制" }}
              </button>
            </div>
            <code>{{ cfg.callbacks[id].url }}</code>
            <p>{{ cfg.callbacks[id].how }}</p>
          </div>
          <label v-if="cfg.secret_source[id] === 'admin'" class="ad-clear">
            <input v-model="clear" type="checkbox" :value="id" />清除后台保存的回调密钥
          </label>
        </article>
      </div>
      <p class="ad-muted">
        激励视频必须配置回调密钥，否则该平台不会被分配（金币只在平台服务器回调验签通过后到账）。修改 AppID 后，用户需要重新打开 App 才会用新的 AppID 初始化。
      </p>

      <h3>流量分配</h3>
      <div class="ad-rules">
        <article v-for="f in formats" :key="f.id" class="ad-rule">
          <header>
            <div><b>{{ f.label }}</b><small>{{ f.hint }}</small></div>
            <div class="ad-mode" role="radiogroup" :aria-label="`${f.label}分配方式`">
              <label><input v-model="cfg[f.id].mode" type="radio" value="weighted" />按概率</label>
              <label><input v-model="cfg[f.id].mode" type="radio" value="ordered" />按顺序</label>
            </div>
          </header>
          <template v-if="cfg[f.id].mode === 'weighted'">
            <div v-for="id in ids" :key="id" class="ad-slider" :class="{ dim: !usable(id, f.id) }">
              <span>{{ labels[id] }}</span>
              <input v-model.number="cfg[f.id].weights[id]" type="range" min="0" max="100" step="5" :aria-label="`${labels[id]} ${f.label}权重`" />
              <output>{{ cfg[f.id].weights[id] || 0 }}</output>
              <em>{{ usable(id, f.id) ? `${share(f.id, id)}%` : "未就绪" }}</em>
            </div>
            <p class="ad-muted">右侧百分比是“首选该平台”的实际概率；权重为 0 的平台不参与。首选没填充时按权重从高到低换下一家。</p>
          </template>
          <ol v-else class="ad-order">
            <li v-for="(id, index) in cfg[f.id].order" :key="id" :class="{ dim: !usable(id, f.id) }">
              <span>{{ index + 1 }}. {{ labels[id] }}<small v-if="!usable(id, f.id)">（未就绪，跳过）</small></span>
              <span>
                <button type="button" class="outline small" :disabled="index === 0" @click="move(f.id, index, -1)">上移</button>
                <button type="button" class="outline small" :disabled="index === cfg[f.id].order.length - 1" @click="move(f.id, index, 1)">下移</button>
              </span>
            </li>
          </ol>
        </article>
      </div>
      <button class="primary" :disabled="saving"><Icon name="check" :size="17" />保存广告配置</button>
    </form>
  </section>
</template>

<style scoped>
.ad-admin h2 { margin: 0 0 6px; }
.ad-admin h3 { margin: 22px 0 10px; font-size: 15px; }
.ad-head p, .ad-muted { color: #9a8274; font-size: 13px; line-height: 1.7; margin: 0; }
.ad-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 10px; padding: 10px 12px; margin-bottom: 12px; font-size: 13px; }
.ad-warn { background: #fff4e5; color: #a5520b; border-radius: 10px; padding: 10px 12px; font-size: 13px; }
.ad-switches { display: grid; gap: 10px; margin: 16px 0 6px; }
.ad-switch { display: flex; gap: 10px; align-items: flex-start; cursor: pointer; }
.ad-switch input { margin-top: 3px; }
.ad-switch span { display: grid; gap: 2px; }
.ad-switch small { color: #9a8274; font-size: 12px; line-height: 1.6; }
.ad-switch.compact { align-items: center; }
.ad-switch.compact input { margin: 0; }
.ad-inline { display: flex; gap: 8px; align-items: center; font-size: 13px; color: #6d5547; padding-left: 26px; }
.ad-inline input { width: 80px; }
.ad-networks { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 12px; }
.ad-net { border: 1px solid #f1e3d8; border-radius: 14px; padding: 14px; display: grid; gap: 12px; background: #fff; }
.ad-net.off { background: #fcf9f6; }
.ad-net > header { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
.ad-tag { font-size: 12px; color: #a08779; background: #f7efe8; border-radius: 999px; padding: 3px 9px; white-space: nowrap; }
.ad-tag.ok { color: #2f8a4c; background: #eefaf1; }
.ad-fields { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.ad-fields label { display: grid; gap: 4px; font-size: 12px; color: #8a6f61; }
.ad-callback { background: #fbf6f1; border-radius: 10px; padding: 10px; display: grid; gap: 6px; }
.ad-callback-head { display: flex; justify-content: space-between; align-items: center; font-size: 12px; }
.ad-callback code { font-size: 12px; word-break: break-all; color: #3d2b22; background: #fff; border-radius: 8px; padding: 8px; user-select: all; }
.ad-callback p { margin: 0; font-size: 12px; color: #9a8274; line-height: 1.6; }
.ad-clear { font-size: 12px; color: #b0533a; display: flex; gap: 6px; align-items: center; }
.ad-rules { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 12px; margin-bottom: 16px; }
.ad-rule { border: 1px solid #f1e3d8; border-radius: 14px; padding: 14px; display: grid; gap: 10px; }
.ad-rule > header { display: flex; justify-content: space-between; gap: 10px; align-items: center; }
.ad-rule > header > div:first-child { display: grid; gap: 2px; }
.ad-rule small { color: #9a8274; font-size: 12px; }
.ad-mode { display: flex; gap: 10px; font-size: 13px; }
.ad-mode label { display: flex; gap: 4px; align-items: center; cursor: pointer; }
.ad-slider { display: grid; grid-template-columns: 56px 1fr 34px 56px; gap: 8px; align-items: center; font-size: 13px; }
.ad-slider input { accent-color: #f65331; width: 100%; }
.ad-slider output { text-align: right; color: #6d5547; }
.ad-slider em { font-style: normal; text-align: right; font-weight: 600; color: #f65331; }
.ad-slider.dim em { color: #c4b2a6; font-weight: 400; }
.ad-slider.dim span { color: #c4b2a6; }
.ad-order { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
.ad-order li { display: flex; justify-content: space-between; align-items: center; background: #fbf6f1; border-radius: 10px; padding: 8px 10px; font-size: 13px; }
.ad-order li.dim { color: #b9a497; }
.ad-order li span:last-child { display: flex; gap: 6px; }
button.small { padding: 4px 8px; font-size: 12px; display: inline-flex; gap: 4px; align-items: center; }
@media (max-width: 560px) {
  .ad-fields { grid-template-columns: 1fr; }
  .ad-networks, .ad-rules { grid-template-columns: 1fr; }
}
/* Global form styles stretch inputs to full width; switches and sliders here stay compact. */
.ad-admin label { margin-bottom: 0; }
.ad-admin input[type="checkbox"],
.ad-admin input[type="radio"] { width: auto; flex: none; margin: 3px 0 0; padding: 0; accent-color: #f65331; }
.ad-admin input[type="range"] { padding: 0; margin: 0; border: 0; background: transparent; box-shadow: none; }
.ad-fields input { margin-top: 0; padding: 9px 10px; font-size: 13px; }
.ad-inline input { margin: 0; padding: 7px 10px; }
.ad-switch b { white-space: nowrap; }
.ad-mode input { margin: 0; }
</style>
