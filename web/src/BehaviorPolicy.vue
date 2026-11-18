<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const policy = ref(null),
  error = ref(""),
  notice = ref(""),
  saving = ref(false);
const controller = new AbortController();
const groups = [
  {
    title: "采集设置",
    fields: [
      { key: "session_ttl_seconds", label: "会话有效期（秒）", min: 60 },
      { key: "batch_interval_seconds", label: "上报间隔（秒）", min: 1 },
      { key: "touch_sample_percent", label: "触点采样（%）", min: 0, max: 100 },
    ],
  },
  {
    title: "检测阈值",
    fields: [
      { key: "click_window_seconds", label: "点击观察窗口（秒）", min: 1 },
      { key: "click_count_threshold", label: "窗口点击次数", min: 1 },
      { key: "min_interval_samples", label: "点击间隔最少样本", min: 2 },
      { key: "interval_tolerance_ms", label: "间隔偏差（毫秒）", min: 0 },
      {
        key: "repeated_interval_ratio",
        label: "规律间隔比例",
        min: 0,
        max: 1,
        step: 0.01,
      },
      { key: "short_dwell_ms", label: "短停留时长（毫秒）", min: 1 },
      { key: "short_dwell_samples", label: "短停留最少样本", min: 1 },
      { key: "business_mismatch_threshold", label: "业务不匹配次数", min: 1 },
      {
        key: "session_anomaly_threshold",
        label: "会话异常最少样本",
        min: 2,
        max: 100,
      },
    ],
  },
];
const synced = () => policy.value?.score_source?.source === "DeviceTrust";
const syncedAt = () =>
  policy.value?.score_source?.synced_at
    ? new Date(policy.value.score_source.synced_at).toLocaleString("zh-CN", { hour12: false })
    : "";
const weights = [
  { key: "click_frequency", label: "点击密集" },
  { key: "regular_intervals", label: "间隔规律" },
  { key: "short_dwell", label: "短停留" },
  { key: "business_mismatch", label: "业务不匹配" },
  { key: "session_anomaly", label: "会话异常" },
];
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function refresh() {
  error.value = "";
  try {
    policy.value = await props.api("/admin/behavior/policy", {
      signal: controller.signal,
    });
  } catch (failure) {
    fail(failure);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    const { revision, ...body } = policy.value;
    policy.value = await props.api("/admin/behavior/policy", {
      method: "PUT",
      body,
      signal: controller.signal,
    });
    notice.value = "策略已保存";
  } catch (failure) {
    fail(failure);
  } finally {
    saving.value = false;
  }
}
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>
<template>
  <section class="admin-card behavior-policy">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="behavior-policy-notice">
      <Icon name="check" :size="15" />{{ notice }}
    </div>
    <form v-if="policy" @submit.prevent="save">
      <header>
        <label class="behavior-policy-enable"
          ><input v-model="policy.enabled" type="checkbox" />开启行为审计</label
        ><span>V{{ policy.revision }}</span>
      </header>
      <div class="behavior-policy-mode">
        <label
          >处置方式<select v-model="policy.mode" aria-label="行为处置方式">
            <option value="observe">仅观察（只记录告警）</option>
            <option value="review">待核查（达到复核线限制领取）</option>
            <option value="hold">暂缓（达到限制线直接限制）</option>
          </select></label
        ><span
          title="行为分数达到下方同步的复核线/限制线后按所选方式处理，样本不足时保持待评估。人工放行的账号和审核账号不会被自动限制。"
          tabindex="0"
          >ⓘ</span
        >
      </div>
      <section
        v-for="group in groups"
        :key="group.title"
        class="behavior-policy-group"
      >
        <h3>{{ group.title }}</h3>
        <div>
          <label v-for="field in group.fields" :key="field.key"
            >{{ field.label
            }}<input
              v-model.number="policy[field.key]"
              :aria-label="field.label"
              type="number"
              :min="field.min"
              :max="field.max"
              :step="field.step || 1"
              required
          /></label>
        </div>
      </section>
      <section class="behavior-policy-group">
        <h3>证据权重</h3>
        <div>
          <label v-for="field in weights" :key="field.key"
            >{{ field.label
            }}<input
              v-model.number="policy.weights[field.key]"
              :aria-label="field.label + '权重'"
              type="number"
              min="0"
              max="100"
              required
          /></label>
        </div>
      </section>
      <section class="behavior-policy-group behavior-score-sync">
        <h3>风险分级线 <small>同步自 DeviceTrust，无需在这里设置</small></h3>
        <div class="behavior-score-lines">
          <div>
            <span>复核线</span>
            <b>{{ policy.review_score }}<small> 分</small></b>
            <em>行为分数达到即标“中风险”；“待核查”模式下限制领取、不出广告</em>
          </div>
          <div>
            <span>限制线</span>
            <b>{{ policy.hold_score }}<small> 分</small></b>
            <em>达到即标“高风险”；“暂缓”模式下直接限制账号</em>
          </div>
        </div>
        <p v-if="synced()" class="behavior-score-note">
          与设备核验使用同一套分数线，跟随 DeviceTrust 平台策略自动更新（最近同步 {{ syncedAt() }}）。如需调整，请在 DeviceTrust 平台修改。
        </p>
        <p v-else class="behavior-score-note warn">
          暂未取得 DeviceTrust 核验结果，先使用默认分数线；有用户完成设备核验后自动同步平台设置。
        </p>
      </section>
      <footer>
        <button class="primary" :disabled="saving">
          <Icon name="check" :size="15" />{{ saving ? "保存中…" : "保存策略" }}
        </button>
      </footer>
    </form>
    <p v-else-if="!error" class="behavior-policy-loading">正在读取策略…</p>
  </section>
</template>
<style scoped>
.behavior-policy {
  padding: 18px 20px;
  max-width: none;
}
.behavior-policy form > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #eddfcd;
  padding-bottom: 22px;
}
.behavior-policy-enable {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
  color: #9b744c;
  margin: 0;
}
.behavior-policy-enable input {
  width: 16px;
  height: 16px;
  margin: 0;
  accent-color: #dda167;
}
.behavior-policy header > span {
  font-size: 12.5px;
  color: #baa180;
}
.behavior-policy-mode {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 24px 0;
}
.behavior-policy-mode > label {
  display: flex;
  align-items: center;
  gap: 13px;
  margin: 0;
  font-size: 13px;
  color: #b09676;
  font-weight: 400;
}
.behavior-policy-mode select {
  width: 260px;
  padding: 10px 12px;
  border-radius: 8px;
  font-size: 13px;
  margin: 0;
}
.behavior-policy-mode > span {
  font-size: 13.5px;
  color: #baa482;
  cursor: help;
}
.behavior-policy-group {
  border-top: 1px solid #f0e4d5;
  padding: 23px 0 5px;
}
.behavior-policy-group h3 {
  font-size: 14px;
  color: #9a754d;
  margin-bottom: 20px;
  font-weight: 600;
}
.behavior-policy-group > div {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 0 23px;
}
.behavior-policy-group label {
  font-size: 12.5px;
  color: #b09677;
  font-weight: 400;
  margin-bottom: 20px;
}
.behavior-policy-group input {
  font-size: 13.5px;
  padding: 10px 12px;
  margin-top: 8px;
  border-radius: 9px;
  color: #96714b;
  background: #fffcf7;
}
.behavior-policy footer {
  border-top: 1px solid #efe2d0;
  padding-top: 20px;
  display: flex;
  justify-content: flex-end;
}
.behavior-policy footer .primary {
  font-size: 13px;
  border-radius: 9px;
  padding: 11px 17px;
}
.behavior-policy-notice {
  display: flex;
  align-items: center;
  gap: 7px;
  background: #edf5e8;
  border: 0;
  color: #91a27b;
  padding: 12px;
  border-radius: 9px;
  font-size: 13px;
  margin-bottom: 18px;
}
.behavior-policy-loading {
  padding: 50px;
  text-align: center;
  font-size: 13.5px;
  color: #baa184;
}
@media (max-width: 650px) {
  .behavior-policy {
    padding: 21px 17px;
  }
  .behavior-policy-group > div {
    grid-template-columns: 1fr 1fr;
    gap: 0 15px;
  }
  .behavior-policy-group label {
    font-size: 12px;
  }
  .behavior-policy-group input {
    font-size: 13px;
    padding: 10px;
  }
  .behavior-policy-enable {
    font-size: 14px;
  }
}
.behavior-score-sync h3 small { font-size: 13.5px; font-weight: 400; color: #a8907f; margin-left: 8px; }
.behavior-score-lines { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
.behavior-score-lines > div { display: grid; gap: 4px; background: #fbf6f1; border: 0; border-radius: 12px; padding: 12px 14px; }
.behavior-score-lines span { font-size: 13.5px; color: #a8907f; }
.behavior-score-lines b { font-size: 22px; color: #4f3627; font-variant-numeric: tabular-nums; }
.behavior-score-lines b small { font-size: 13.5px; color: #a8907f; font-weight: 400; }
.behavior-score-lines em { font-style: normal; font-size: 13.5px; color: #8a6f61; line-height: 1.6; }
.behavior-score-note { margin: 10px 0 0; font-size: 14px; color: #8a6f61; line-height: 1.7; }
.behavior-score-note.warn { color: #a5520b; }
@media (max-width: 640px) { .behavior-score-lines { grid-template-columns: 1fr; } }
</style>
