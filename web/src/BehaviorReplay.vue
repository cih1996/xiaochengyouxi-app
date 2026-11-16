<script setup>
import { ref, computed, watch, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import ReplayPhone from "./ReplayPhone.vue";
import { number, dateTime, duration, label } from "./audit-format";
const props = defineProps({ api: Function, userId: Number, sessionId: String });
const emit = defineEmits(["close", "unauthorized"]);
const result = ref(null),
  events = ref([]),
  error = ref(""),
  loading = ref(false),
  playing = ref(false),
  speed = ref(1),
  position = ref(0),
  eventType = ref("");
let controller, frame, lastFrame;
const endTime = computed(() =>
  Math.max(
    0,
    ...events.value.map((event) => event.time_ms + (event.duration_ms || 0)),
  ),
);
const occurred = computed(() =>
  events.value.filter((event) => event.time_ms <= position.value),
);
const current = computed(() => occurred.value.at(-1));
const screen = computed(() => {
  const lastBackground = occurred.value.findLast((event) =>
    ["foreground", "background"].includes(event.type),
  );
  if (lastBackground?.type === "background") return "background";
  return (
    occurred.value.findLast(
      (event) => event.screen && event.screen !== "unknown",
    )?.screen || "unknown"
  );
});
const touches = computed(() =>
  occurred.value.filter(
    (event) =>
      ["gesture", "click"].includes(event.type) &&
      event.x != null &&
      event.y != null &&
      position.value - event.time_ms < 850,
  ),
);
const credits = computed(() => [
  ...new Map(
    occurred.value
      .filter(
        (event) =>
          event.server_confirmed &&
          event.server_state === "credited" &&
          event.credited_coins > 0,
      )
      .map((event) => [event.operation_ref || event.id, event]),
  ).values(),
]);
const lastCredit = computed(() => credits.value.at(-1)?.credited_coins || 0);
const totalCredit = computed(() =>
  credits.value.reduce((sum, event) => sum + event.credited_coins, 0),
);
const shownEvents = computed(() =>
  events.value.filter(
    (event) => !eventType.value || event.type === eventType.value,
  ),
);
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
function stop() {
  playing.value = false;
  cancelAnimationFrame(frame);
  lastFrame = null;
}
async function load(more = false) {
  if (!more) {
    stop();
    position.value = 0;
    events.value = [];
    result.value = null;
  }
  controller?.abort();
  controller = new AbortController();
  loading.value = true;
  error.value = "";
  try {
    const offset = more ? result.value.next_offset : 0;
    const data = await props.api(
      `/admin/users/${props.userId}/behavior/sessions/${encodeURIComponent(props.sessionId)}?offset=${offset}&limit=5000`,
      { signal: controller.signal },
    );
    events.value = more ? [...events.value, ...data.events] : data.events;
    result.value = data;
  } catch (failure) {
    fail(failure);
  } finally {
    loading.value = false;
  }
}
function tick(now) {
  if (!playing.value) return;
  if (lastFrame != null)
    position.value = Math.min(
      endTime.value,
      position.value + (now - lastFrame) * speed.value,
    );
  lastFrame = now;
  if (position.value >= endTime.value) {
    stop();
    return;
  }
  frame = requestAnimationFrame(tick);
}
function togglePlay() {
  if (playing.value) {
    stop();
    return;
  }
  if (position.value >= endTime.value) position.value = 0;
  playing.value = true;
  lastFrame = null;
  frame = requestAnimationFrame(tick);
}
function seek(value) {
  position.value = Number(value);
  lastFrame = null;
}
async function copy(value) {
  try {
    await navigator.clipboard.writeText(value);
  } catch (failure) {
    fail(failure);
  }
}
watch(
  () => [props.userId, props.sessionId],
  () => load(),
  { immediate: true },
);
onUnmounted(() => {
  stop();
  controller?.abort();
});
</script>
<template>
  <div class="behavior-replay-overlay" @click.self="emit('close')">
    <section
      class="behavior-replay"
      role="dialog"
      aria-modal="true"
      aria-label="行为回放"
    >
      <header>
        <h2>行为回放</h2>
        <span title="按已采集事件呈现页面和操作">操作重现 ⓘ</span
        ><button aria-label="关闭行为回放" @click="emit('close')">
          <Icon name="close" :size="21" />
        </button>
      </header>
      <div v-if="error" class="form-error" role="alert">{{ error }}</div>
      <div v-if="loading && !result" class="replay-loading">正在读取会话…</div>
      <template v-if="result"
        ><div v-if="!events.length" class="replay-empty">
          <Icon name="list" :size="31" /><b>此会话暂无可回放事件</b
          ><span v-for="note in result.availability.notes" :key="note">{{
            note
          }}</span>
        </div>
        <div v-else class="replay-grid">
          <div class="replay-stage">
            <ReplayPhone
              :screen="screen"
              :event="current"
              :touches="touches"
              :credit="lastCredit"
            />
            <div class="replay-controls">
              <button
                class="primary"
                :aria-label="playing ? '暂停回放' : '播放回放'"
                @click="togglePlay"
              >
                <Icon :name="playing ? 'pause' : 'play'" :size="16" /></button
              ><span>{{ duration(position) }} / {{ duration(endTime) }}</span
              ><select v-model.number="speed" aria-label="回放倍速">
                <option
                  v-for="value in [0.5, 1, 2, 4]"
                  :key="value"
                  :value="value"
                >
                  {{ value }}×
                </option>
              </select>
            </div>
            <input
              class="replay-progress"
              type="range"
              min="0"
              :max="endTime"
              :value="position"
              step="1"
              aria-label="回放进度"
              @input="seek($event.target.value)"
            />
            <div class="replay-credit">
              <Icon name="check" :size="14" />已核对到账
              {{ number(totalCredit) }} 金币
            </div>
          </div>
          <div class="replay-side">
            <section class="replay-session">
              <h3>会话信息</h3>
              <dl>
                <div>
                  <dt>开始时间</dt>
                  <dd>{{ dateTime(result.session.started_at) }}</dd>
                </div>
                <div>
                  <dt>最近上报</dt>
                  <dd>{{ dateTime(result.session.last_received_at) }}</dd>
                </div>
                <div>
                  <dt>设备标识</dt>
                  <dd>
                    <button
                      v-if="result.session.device_id"
                      :title="result.session.device_id"
                      @click="copy(result.session.device_id)"
                    >
                      {{ result.session.device_id.slice(0, 20)
                      }}<Icon name="list" :size="11" />
                    </button>
                    <span v-else>未关联</span>
                  </dd>
                </div>
                <div>
                  <dt>核验结果</dt>
                  <dd>{{ label(result.session.proof_level) }}</dd>
                </div>
                <div>
                  <dt>采集事件 / 缺失</dt>
                  <dd>
                    {{ number(result.events_total) }} /
                    {{ number(result.session.dropped_events) }}
                  </dd>
                </div>
              </dl>
              <p v-for="note in result.availability.notes" :key="note">
                {{ note }}
              </p>
            </section>
            <section v-if="result.alerts.length" class="replay-alerts">
              <h3>风险证据</h3>
              <article v-for="alert in result.alerts" :key="alert.id">
                <header>
                  <b>{{ alert.score }} 分</b
                  ><span
                    >{{ label(alert.action) }} · V{{
                      alert.policy_revision
                    }}</span
                  >
                </header>
                <div v-for="evidence in alert.evidence" :key="evidence.code">
                  <span>{{ evidence.label }}</span><b>+{{ evidence.points }}</b>
                </div>
              </article>
            </section>
            <section v-if="result.failures.length" class="replay-failures">
              <h3>采集诊断</h3>
              <details
                v-for="failure in result.failures"
                :key="failure.reference"
              >
                <summary>{{ failure.stage }} · {{ failure.code }}</summary>
                <p>{{ failure.detail }}</p>
                <small
                  >{{ dateTime(failure.created_at) }} ·
                  {{ failure.reference }}</small
                >
              </details>
            </section>
            <section class="replay-events">
              <header>
                <h3>
                  操作事件
                  <small
                    >{{ number(events.length) }} /
                    {{ number(result.events_total) }}</small
                  >
                </h3>
                <select v-model="eventType" aria-label="回放事件类型">
                  <option value="">全部事件</option>
                  <option
                    v-for="type in [
                      ...new Set(events.map((event) => event.type)),
                    ]"
                    :key="type"
                    :value="type"
                  >
                    {{ label(type) }}
                  </option>
                </select>
              </header>
              <div class="replay-event-list">
                <button
                  v-for="event in shownEvents"
                  :key="event.id"
                  :class="{ active: current?.id === event.id }"
                  @click="seek(event.time_ms)"
                >
                  <time>{{ duration(event.time_ms) }}</time>
                  <div>
                    <b
                      >{{ label(event.type)
                      }}<span
                        v-if="
                          event.control_id && event.control_id !== 'unknown'
                        "
                      >
                        · {{ label(event.control_id) }}</span
                      ></b
                    ><span
                      >{{ label(event.screen)
                      }}<small v-if="event.duration_ms">
                        · {{ duration(event.duration_ms) }}</small
                      ></span
                    ><span
                      v-if="event.operation_ref"
                      class="replay-operation"
                      >{{ event.operation_ref }}</span
                    >
                  </div>
                  <strong
                    v-if="
                      event.server_confirmed &&
                      event.server_state === 'credited' &&
                      event.credited_coins > 0
                    "
                    >+{{ event.credited_coins
                    }}<Icon name="check" :size="12" /></strong
                  ><small
                    v-else-if="event.type === 'business'"
                    class="replay-unconfirmed"
                    >{{ event.server_confirmed ? "已核对" : "未核对" }}</small
                  >
                </button>
              </div>
              <button
                v-if="result.next_offset != null"
                class="outline replay-more"
                :disabled="loading"
                @click="load(true)"
              >
                {{ loading ? "读取中…" : "继续加载事件" }}
              </button>
            </section>
          </div>
        </div>
      </template>
    </section>
  </div>
</template>
<style scoped>
.behavior-replay-overlay {
  position: fixed;
  inset: 0;
  background: #4c362b80;
  backdrop-filter: blur(5px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 25px;
  z-index: 90;
}
.behavior-replay {
  width: 1180px;
  max-width: 100%;
  max-height: 95vh;
  overflow-y: auto;
  background: #fdfaf4;
  border: 1px solid #e9d8bf;
  border-radius: 20px;
  padding: 24px 27px;
}
.behavior-replay > header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 23px;
}
.behavior-replay > header h2 {
  font-size: 20px;
  color: #785336;
}
.behavior-replay > header > span {
  font-size: 10px;
  color: #b39b7c;
  cursor: help;
}
.behavior-replay > header > button {
  margin-left: auto;
  color: #b69a79;
}
.replay-grid {
  display: grid;
  grid-template-columns: minmax(330px, 0.9fr) minmax(390px, 1.2fr);
  gap: 28px;
}
.replay-stage {
  display: flex;
  flex-direction: column;
  align-items: center;
  background: #f5eddf;
  border: 1px solid #ebdcc8;
  border-radius: 15px;
  padding: 24px 20px;
  align-self: start;
  position: sticky;
  top: 0;
}
.replay-controls {
  display: flex;
  align-items: center;
  gap: 15px;
  width: 292px;
  margin: 20px 0 14px;
}
.replay-controls .primary {
  width: 35px;
  height: 35px;
  padding: 0;
  border-radius: 9px;
}
.replay-controls > span {
  font-size: 10px;
  color: #a78b69;
  font-variant-numeric: tabular-nums;
  flex: 1;
}
.replay-controls select {
  padding: 7px 6px;
  border-radius: 7px;
  font-size: 10px;
  width: 53px;
  border-color: #e3cfb4;
  color: #a17e54;
}
.replay-progress {
  width: 292px;
  padding: 0;
  height: 4px;
  accent-color: #d7a466;
  border: 0;
  margin: 0;
  border-radius: 5px;
}
.replay-credit {
  display: flex;
  gap: 6px;
  align-items: center;
  color: #9ba785;
  font-size: 10px;
  margin-top: 17px;
}
.replay-side {
  min-width: 0;
}
.replay-side h3 {
  font-size: 13px;
  color: #8b6847;
  font-weight: 600;
}
.replay-session {
  padding: 4px 0 20px;
  border-bottom: 1px solid #eadbc6;
}
.replay-session dl {
  margin: 16px 0 0;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 15px;
}
.replay-session dt {
  font-size: 9px;
  color: #b79d7e;
  margin-bottom: 6px;
}
.replay-session dd {
  font-size: 10px;
  color: #9d7d58;
  margin: 0;
  overflow-wrap: anywhere;
}
.replay-session dd button {
  display: flex;
  gap: 6px;
  align-items: center;
  font: 9px monospace;
  text-align: left;
}
.replay-session > p {
  font-size: 9px;
  line-height: 1.8;
  color: #ba9c77;
  margin-top: 12px;
}
.replay-alerts,
.replay-failures {
  margin-top: 21px;
}
.replay-alerts article {
  margin-top: 12px;
  border: 1px solid #ecd3b6;
  border-radius: 9px;
  padding: 13px;
  background: #fff4e3;
}
.replay-alerts article > header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.replay-alerts header b {
  font-size: 13px;
  color: #bd8b53;
}
.replay-alerts header span {
  font-size: 9px;
  color: #b69a76;
}
.replay-alerts article > div {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  font-size: 10px;
  color: #ab865c;
  line-height: 2;
}
.replay-alerts article > div b {
  font-size: 9px;
  font-weight: 400;
}
.replay-failures details {
  margin-top: 10px;
  padding: 10px;
  border: 1px solid #ebd6c1;
  border-radius: 8px;
  background: #fff7ed;
}
.replay-failures summary {
  font-size: 10px;
  color: #ba8e60;
  cursor: pointer;
}
.replay-failures p {
  font-size: 10px;
  line-height: 1.8;
  color: #ae8560;
  overflow-wrap: anywhere;
  margin-top: 9px;
}
.replay-failures small {
  display: block;
  font-size: 8px;
  color: #bda387;
  margin-top: 8px;
  overflow-wrap: anywhere;
}
.replay-events {
  margin-top: 22px;
}
.replay-events > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.replay-events h3 small {
  font-size: 9px;
  font-weight: 400;
  color: #b99e7b;
  margin-left: 7px;
}
.replay-events > header select {
  width: 96px;
  padding: 7px;
  font-size: 9px;
  border-radius: 7px;
  color: #af906c;
  background: #fffaf0;
}
.replay-event-list {
  max-height: 430px;
  overflow-y: auto;
  border: 1px solid #ede1ce;
  border-radius: 11px;
  background: #fffcf7;
}
.replay-event-list > button {
  width: 100%;
  display: flex;
  align-items: flex-start;
  gap: 11px;
  padding: 12px 13px;
  border-bottom: 1px solid #f0e4d3;
  text-align: left;
}
.replay-event-list > button:last-child {
  border: 0;
}
.replay-event-list > button.active {
  background: #fff0d9;
}
.replay-event-list time {
  font-size: 9px;
  color: #b99b77;
  font-variant-numeric: tabular-nums;
  padding-top: 2px;
}
.replay-event-list > button > div {
  flex: 1;
  min-width: 0;
}
.replay-event-list b {
  font-size: 10px;
  color: #a27b51;
  font-weight: 500;
}
.replay-event-list > button > div > span {
  display: block;
  font-size: 9px;
  color: #b49b7c;
  margin-top: 5px;
}
.replay-event-list small {
  font-size: 8px;
  font-weight: 400;
}
.replay-event-list .replay-operation {
  font: 8px monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.replay-event-list strong {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  color: #8da179;
}
.replay-unconfirmed {
  color: #b69d80;
  white-space: nowrap;
  margin-top: 3px;
}
.replay-more {
  width: 100%;
  font-size: 10px;
  padding: 10px;
  margin-top: 10px;
}
.replay-empty,
.replay-loading {
  padding: 80px 15px;
  display: flex;
  align-items: center;
  flex-direction: column;
  gap: 17px;
  color: #b59d7e;
  font-size: 12px;
}
.replay-empty b {
  font-weight: 500;
}
.replay-empty span {
  font-size: 10px;
}
@media (max-width: 850px) {
  .replay-grid {
    grid-template-columns: 1fr;
  }
  .replay-stage {
    position: static;
  }
  .behavior-replay {
    max-width: 680px;
  }
  .replay-session dl {
    grid-template-columns: 1fr 1fr;
  }
}
@media (max-width: 650px) {
  .behavior-replay-overlay {
    padding: 0;
  }
  .behavior-replay {
    height: 100%;
    max-height: 100%;
    border-radius: 0;
    padding: 20px 16px;
  }
  .behavior-replay > header h2 {
    font-size: 18px;
  }
  .behavior-replay > header > span {
    font-size: 9px;
  }
  .replay-stage {
    padding: 20px 12px;
  }
  .replay-grid {
    gap: 20px;
  }
  .replay-session dl {
    gap: 13px;
  }
  .replay-event-list {
    max-height: 360px;
  }
}
</style>
