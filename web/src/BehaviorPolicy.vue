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
  {
    title: "核查分数",
    fields: [
      { key: "review_score", label: "待核查分数", min: 0, max: 100 },
      { key: "hold_score", label: "暂缓分数", min: 0, max: 100 },
    ],
  },
];
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
            <option value="observe">观察</option>
            <option value="review">待核查</option>
            <option value="hold">暂缓</option>
          </select></label
        ><span
          title="证据达到阈值后按所选方式处理，样本不足时保持待评估。"
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
  padding: 25px 28px;
  max-width: 1050px;
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
  font-size: 10px;
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
  font-size: 11px;
  color: #b09676;
  font-weight: 400;
}
.behavior-policy-mode select {
  width: 136px;
  padding: 10px 12px;
  border-radius: 8px;
  font-size: 11px;
  margin: 0;
}
.behavior-policy-mode > span {
  font-size: 12px;
  color: #baa482;
  cursor: help;
}
.behavior-policy-group {
  border-top: 1px solid #f0e4d5;
  padding: 23px 0 5px;
}
.behavior-policy-group h3 {
  font-size: 13px;
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
  font-size: 10px;
  color: #b09677;
  font-weight: 400;
  margin-bottom: 20px;
}
.behavior-policy-group input {
  font-size: 12px;
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
  font-size: 11px;
  border-radius: 9px;
  padding: 11px 17px;
}
.behavior-policy-notice {
  display: flex;
  align-items: center;
  gap: 7px;
  background: #edf5e8;
  border: 1px solid #d9e8cf;
  color: #91a27b;
  padding: 12px;
  border-radius: 9px;
  font-size: 11px;
  margin-bottom: 18px;
}
.behavior-policy-loading {
  padding: 50px;
  text-align: center;
  font-size: 12px;
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
    font-size: 9px;
  }
  .behavior-policy-group input {
    font-size: 11px;
    padding: 10px;
  }
  .behavior-policy-enable {
    font-size: 13px;
  }
}
</style>
