<script setup>
import { ref, computed, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const rain = ref(null),
  status = ref(null),
  error = ref(""),
  notice = ref(""),
  busy = ref(false),
  previewKey = ref(0);
const controller = new AbortController();
const grandPercent = computed({
  get: () => Math.round(rain.value.grand_probability * 10000) / 100,
  set: (value) => (rain.value.grand_probability = Number(value) / 100),
});
const opacityPercent = computed({
  get: () => Math.round(rain.value.overlay_opacity * 100),
  set: (value) => (rain.value.overlay_opacity = Number(value) / 100),
});
const previewCount = computed(() =>
  Math.min(rain.value.max_visible, rain.value.packet_count),
);
const nextTime = computed(() =>
  status.value?.next_start_at
    ? new Date(status.value.next_start_at).toLocaleString("zh-CN", {
        timeZone: "Asia/Shanghai",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
      })
    : "尚未安排",
);
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function refresh() {
  error.value = "";
  try {
    [rain.value, status.value] = await Promise.all([
      props.api("/admin/rain-config", { signal: controller.signal }),
      props.api("/rain/status", { signal: controller.signal }),
    ]);
  } catch (failure) {
    fail(failure);
  }
}
async function saveRain() {
  busy.value = true;
  error.value = "";
  notice.value = "";
  try {
    rain.value = await props.api("/admin/rain-config", {
      method: "PUT",
      body: rain.value,
    });
    status.value = await props.api("/rain/status");
    notice.value = "红包雨设置已保存";
  } catch (failure) {
    fail(failure);
  } finally {
    busy.value = false;
  }
}
async function startRain() {
  busy.value = true;
  error.value = "";
  notice.value = "";
  try {
    rain.value = await props.api("/admin/rain-config", {
      method: "PUT",
      body: rain.value,
    });
    status.value = await props.api("/admin/rain/start", {
      method: "POST",
      body: { countdown_seconds: 15 },
    });
    notice.value = "已安排开场，倒计时即将开始";
  } catch (failure) {
    fail(failure);
  } finally {
    busy.value = false;
  }
}
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <div class="rain-admin">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="rain-admin-notice">
      <Icon name="check" :size="16" />{{ notice }}
    </div>
    <div v-if="!rain" class="rain-admin-loading">正在读取活动设置…</div>
    <template v-else
      ><div class="rain-admin-status">
        <Icon name="calendar" :size="17" />
        <p>
          {{
            status?.phase === "active"
              ? "红包雨正在进行"
              : status?.enabled
                ? "下场开场 " + nextTime
                : "当前活动未开启"
          }}
        </p>
      </div>
      <div class="rain-admin-grid">
        <section class="rain-config-card">
          <form @submit.prevent="saveRain">
            <header>
              <div>
                <h2>红包雨设置</h2>
              </div>
              <label class="rain-toggle"
                ><input
                  v-model="rain.enabled"
                  type="checkbox"
                  aria-label="开启红包雨"
                /><i></i>{{ rain.enabled ? "已开启" : "已关闭" }}</label
              >
            </header>
            <div class="rain-form-group">
              <h3>开场时间</h3>
              <div class="rain-form-fields">
                <label
                  >每日首场<select
                    v-model.number="rain.start_hour"
                    aria-label="每日首场"
                  >
                    <option v-for="hour in 24" :key="hour" :value="hour - 1">
                      {{ String(hour - 1).padStart(2, "0") }}:00
                    </option>
                  </select></label
                ><label
                  >每隔几小时<input
                    v-model.number="rain.every_hours"
                    type="number"
                    min="1"
                    max="24"
                    required /></label
                ><label
                  >每场时长（秒）<input
                    v-model.number="rain.duration_seconds"
                    type="number"
                    min="10"
                    max="180"
                    required /></label
                ><label
                  >每人红包数<input
                    v-model.number="rain.packet_count"
                    type="number"
                    min="5"
                    max="60"
                    required
                /></label>
              </div>
            </div>
            <div class="rain-form-group">
              <h3>红包与奖励</h3>
              <div class="rain-form-fields">
                <label
                  >小包最低金币<input
                    v-model.number="rain.small_min_coins"
                    type="number"
                    min="1"
                    max="10"
                    required /></label
                ><label
                  >小包最高金币<input
                    v-model.number="rain.small_max_coins"
                    type="number"
                    min="1"
                    max="10"
                    required /></label
                ><label
                  >大奖概率（%）<input
                    v-model="grandPercent"
                    type="number"
                    min="0"
                    max="100"
                    step="0.1"
                    required /></label
                ><label
                  >大奖金币<input
                    v-model.number="rain.grand_coins"
                    type="number"
                    min="1"
                    max="10000"
                    required /></label
                ><label class="rain-form-wide"
                  >每人本场金币上限<input
                    v-model.number="rain.max_total_coins"
                    type="number"
                    min="1"
                    max="600000"
                    required
                /></label>
              </div>
            </div>
            <div class="rain-form-group">
              <h3>下落效果</h3>
              <div class="rain-form-fields">
                <label
                  >下落速度（倍）<input
                    v-model.number="rain.fall_speed"
                    type="number"
                    min="0.4"
                    max="3"
                    step="0.1"
                    required /></label
                ><label
                  >同时可见红包数<input
                    v-model.number="rain.max_visible"
                    type="number"
                    min="1"
                    max="20"
                    required /></label
                ><label class="rain-form-wide rain-opacity-control"
                  ><span
                    >遮罩浓度<b>{{ opacityPercent }}%</b></span
                  ><input
                    v-model="opacityPercent"
                    type="range"
                    min="0"
                    max="90"
                    aria-label="遮罩浓度"
                /></label>
              </div>
            </div>
            <footer>
              <button class="primary" :disabled="busy">
                <Icon name="check" :size="16" />保存设置</button
              ><button
                type="button"
                class="outline"
                :disabled="busy"
                @click="startRain"
              >
                <Icon name="spark" :size="15" />立即开一场
              </button>
            </footer>
          </form>
        </section>
        <aside class="rain-preview-card">
          <header>
            <h2>效果预览</h2>
            <button @click="previewKey++">
              <Icon name="refresh" :size="14" />重播
            </button>
          </header>
          <div
            :key="previewKey"
            class="rain-admin-preview"
            :style="{ '--preview-opacity': rain.overlay_opacity }"
          >
            <div class="rain-preview-backdrop"></div>
            <img src="/assets/rain-sky.png" class="rain-preview-sky" alt="" />
            <div class="rain-preview-field">
              <img
                v-for="index in previewCount"
                :key="index"
                :class="{ large: index === 1 && rain.grand_probability > 0 }"
                :src="
                  index === 1 && rain.grand_probability > 0
                    ? '/assets/rain-large.png'
                    : '/assets/rain-small.png'
                "
                :style="{
                  left: 5 + ((index * 37) % 73) + '%',
                  '--preview-duration': 5 / rain.fall_speed + 's',
                  '--preview-delay':
                    (-index / previewCount) * (5 / rain.fall_speed) + 's',
                  '--preview-turn': -9 + (index % 5) * 4 + 'deg',
                  '--preview-rest': 10 + (index % 4) * 19 + '%',
                }"
                alt="红包样式"
              />
            </div>
            <div class="rain-preview-legend">
              <span><img src="/assets/rain-small.png" alt="" />小红包</span
              ><span><img src="/assets/rain-large.png" alt="" />大奖红包</span>
            </div>
          </div>
          <div class="rain-preview-summary">
            <span
              >每人 <b>{{ rain.packet_count }}</b> 个</span
            ><span
              >同时 <b>{{ rain.max_visible }}</b> 个</span
            ><span
              >时长 <b>{{ rain.duration_seconds }}</b> 秒</span
            >
          </div>
          <div class="rain-preview-next">
            <span>服务端下场安排</span><b>{{ nextTime }}</b>
          </div>
        </aside>
      </div>
    </template>
  </div>
</template>

<style scoped>
.rain-admin {
  min-width: 0;
}
.rain-admin-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  background: #fff1d9;
  border: 1px solid #ebd3ad;
  color: #bd8b4e;
  font-size: 11px;
  line-height: 1.8;
  padding: 12px 16px;
  border-radius: 11px;
  margin-bottom: 18px;
}
.rain-admin-loading {
  padding: 70px 15px;
  text-align: center;
  color: #bb9d7c;
  font-size: 12px;
}
.rain-admin-status {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 13px 17px;
  margin-bottom: 19px;
  border: 1px solid #efdcc3;
  border-radius: 11px;
  background: #fff6e8;
  color: #b18b62;
  font-size: 11px;
}
.rain-admin-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.5fr) minmax(275px, 0.85fr);
  gap: 24px;
  align-items: start;
}
.rain-config-card,
.rain-preview-card {
  background: #fffdfa;
  border: 1px solid #ecddc9;
  border-radius: 17px;
  min-width: 0;
  overflow: hidden;
}
.rain-config-card form > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 24px 25px 22px;
  border-bottom: 1px solid #f1e6d6;
  gap: 15px;
}
.rain-config-card h2 {
  font-size: 16px;
  color: #916a44;
  font-weight: 600;
  margin-top: 0;
}
.rain-toggle {
  display: flex;
  align-items: center;
  gap: 7px;
  white-space: nowrap;
  margin: 0;
  font-size: 10px;
  color: #b8946c;
  font-weight: 400;
  cursor: pointer;
}
.rain-toggle > input {
  position: absolute;
  opacity: 0;
  width: 1px;
  height: 1px;
}
.rain-toggle > i {
  position: relative;
  display: block;
  width: 34px;
  height: 20px;
  border-radius: 100px;
  background: #dfcdb4;
  transition: background 0.2s;
}
.rain-toggle > i:after {
  content: "";
  position: absolute;
  left: 3px;
  top: 3px;
  width: 14px;
  height: 14px;
  background: #fffaf3;
  border-radius: 50%;
  transition: transform 0.2s;
}
.rain-toggle > input:checked + i {
  background: #eaa065;
}
.rain-toggle > input:checked + i:after {
  transform: translateX(14px);
}
.rain-toggle > input:focus-visible + i {
  outline: 3px solid #f1bf86;
  outline-offset: 3px;
}
.rain-form-group {
  padding: 22px 25px 2px;
  border-bottom: 1px solid #f3e9db;
}
.rain-form-group:last-of-type {
  border: 0;
}
.rain-form-group h3 {
  font-size: 12px;
  font-weight: 500;
  color: #a27d54;
  margin-bottom: 19px;
}
.rain-form-fields {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px 20px;
}
.rain-form-fields > label {
  font-size: 10px;
  color: #b29572;
  margin: 0 0 14px;
  min-width: 0;
  font-weight: 400;
}
.rain-form-fields input,
.rain-form-fields select {
  width: 100%;
  font-size: 12px;
  border: 1px solid #e9d7bd;
  background: #fffaf3;
  color: #a17b4e;
  padding: 11px 12px;
  border-radius: 9px;
  margin-top: 0;
  min-width: 0;
}
.rain-form-fields .rain-form-wide {
  grid-column: span 2;
}
.rain-opacity-control > span {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 15px;
}
.rain-opacity-control b {
  font-size: 11px;
  color: #c2955a;
  font-weight: 500;
}
.rain-opacity-control input {
  padding: 4px 0;
  accent-color: #e5a464;
  height: 26px;
  background: transparent;
  border: 0;
  box-shadow: none;
  margin-top: 10px;
  cursor: pointer;
}
.rain-config-card form > footer {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 22px 25px;
  border-top: 1px solid #f0e2cf;
}
.rain-config-card .primary,
.rain-config-card .outline {
  font-size: 11px;
  border-radius: 9px;
  padding: 12px 17px;
}
.rain-config-card .outline {
  color: #bd8e55;
  background: #fff7e7;
  border-color: #e7cfaa;
}
.rain-preview-card {
  position: sticky;
  top: 22px;
}
.rain-preview-card > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 21px;
  border-bottom: 1px solid #eee0cc;
}
.rain-preview-card h2 {
  font-size: 13px;
  color: #9d7951;
  font-weight: 500;
}
.rain-preview-card > header > button {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 10px;
  color: #bb9a71;
}
.rain-admin-preview {
  height: 480px;
  position: relative;
  overflow: hidden;
  background: #fff2db;
  margin: 17px;
  border-radius: 21px;
  border: 4px solid #fffaf0;
  box-shadow: 0 7px 20px #98713f13;
}
.rain-preview-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(87, 43, 22, var(--preview-opacity));
}
.rain-preview-sky {
  position: absolute;
  left: 0;
  top: -8px;
  width: 100%;
  height: 220px;
  object-fit: cover;
  object-position: top;
}
.rain-preview-field {
  position: absolute;
  inset: 60px 0 58px;
  overflow: hidden;
}
.rain-preview-field > img {
  position: absolute;
  width: 64px;
  height: 72px;
  object-fit: contain;
  top: -85px;
  filter: drop-shadow(0 4px 5px #a45a2b32);
  animation: previewFall var(--preview-duration) var(--preview-delay) linear
    infinite both;
  pointer-events: none;
}
.rain-preview-field > img.large {
  width: 85px;
  height: 94px;
  filter: drop-shadow(0 0 10px #ffe0836a);
}
.rain-preview-legend {
  position: absolute;
  bottom: 14px;
  left: 14px;
  right: 14px;
  display: flex;
  justify-content: center;
  gap: 18px;
  font-size: 9px;
  color: #ffe4b7;
}
.rain-preview-legend > span {
  display: flex;
  align-items: center;
  gap: 5px;
}
.rain-preview-legend img {
  width: 27px;
  height: 30px;
  object-fit: contain;
}
.rain-preview-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin: 0 21px;
  padding: 5px 0 22px;
  font-size: 9px;
  color: #bca17e;
  border-bottom: 1px solid #eee0c9;
}
.rain-preview-summary b {
  color: #bd8f54;
  font-size: 12px;
  font-weight: 500;
  margin: 0 2px;
}
.rain-preview-next {
  padding: 17px 21px 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.rain-preview-next > span {
  font-size: 9px;
  color: #bda480;
}
.rain-preview-next > b {
  font-size: 11px;
  font-weight: 500;
  color: #b18856;
}
@keyframes previewFall {
  0% {
    transform: translateY(0) rotate(var(--preview-turn));
    opacity: 0;
  }
  8%,
  90% {
    opacity: 1;
  }
  100% {
    transform: translateY(420px) rotate(calc(var(--preview-turn) * -1));
    opacity: 0;
  }
}
@media (max-width: 1120px) {
  .rain-admin-grid {
    grid-template-columns: minmax(0, 1.3fr) minmax(250px, 1fr);
    gap: 18px;
  }
  .rain-form-fields {
    gap: 14px;
  }
  .rain-config-card form > header {
    padding: 22px;
  }
  .rain-form-group {
    padding: 21px 22px 2px;
  }
  .rain-config-card form > footer {
    padding: 21px 22px;
    gap: 9px;
  }
  .rain-config-card .primary,
  .rain-config-card .outline {
    font-size: 10px;
    padding: 11px 13px;
  }
  .rain-admin-preview {
    height: 445px;
    margin: 14px 12px;
  }
  .rain-preview-summary {
    margin: 0 17px;
    font-size: 8px;
    gap: 6px;
  }
  .rain-preview-next {
    padding: 17px;
    font-size: 9px;
  }
}
@media (max-width: 950px) {
  .rain-admin-grid {
    grid-template-columns: 1fr;
  }
  .rain-preview-card {
    position: static;
  }
  .rain-admin-preview {
    height: 440px;
    max-width: 320px;
    margin: 18px auto;
    border-width: 5px;
  }
  .rain-preview-summary,
  .rain-preview-next {
    max-width: 360px;
    margin: 0 auto;
    width: 100%;
    padding-left: 20px;
    padding-right: 20px;
  }
  .rain-preview-summary {
    padding-bottom: 20px;
  }
}
@media (max-width: 650px) {
  .rain-config-card form > header {
    padding: 21px 18px;
  }
  .rain-config-card h2 {
    font-size: 14px;
  }
  .rain-form-group {
    padding: 20px 18px 3px;
  }
  .rain-form-fields {
    gap: 10px 13px;
  }
  .rain-form-fields > label {
    font-size: 9px;
  }
  .rain-form-fields input,
  .rain-form-fields select {
    font-size: 11px;
    padding: 10px;
    margin-top: 6px;
  }
  .rain-form-group h3 {
    font-size: 11px;
    margin-bottom: 16px;
  }
  .rain-config-card form > footer {
    padding: 20px 18px;
    flex-wrap: wrap;
  }
  .rain-config-card .primary,
  .rain-config-card .outline {
    font-size: 10px;
    padding: 11px 12px;
  }
  .rain-toggle {
    font-size: 9px;
  }
  .rain-admin-preview {
    max-width: 290px;
    margin: 17px auto;
    height: 425px;
  }
  .rain-preview-summary {
    font-size: 8px;
    padding: 0 18px 19px;
  }
  .rain-preview-summary b {
    font-size: 11px;
  }
  .rain-preview-next {
    padding: 17px 18px;
  }
  .rain-preview-next > b {
    font-size: 10px;
  }
  .rain-preview-field > img {
    width: 57px;
    height: 64px;
  }
  .rain-preview-field > img.large {
    width: 75px;
    height: 83px;
  }
}
@media (prefers-reduced-motion: reduce) {
  .rain-preview-field > img {
    animation: none !important;
    opacity: 1;
    top: var(--preview-rest);
    transform: rotate(var(--preview-turn));
  }
  .rain-toggle > i,
  .rain-toggle > i:after {
    transition: none;
  }
}
</style>
