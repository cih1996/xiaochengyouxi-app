<script setup>
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import { prepareCoinSound, playCoinArrival } from "./coin-sound";
const props = defineProps({ api: Function, config: Object });
const emit = defineEmits(["credit", "large"]);
const state = ref(null),
  now = ref(Date.now()),
  rainVisible = ref(false),
  joining = ref(false),
  packets = ref([]),
  joinConfig = ref(null),
  joinedRound = ref(null),
  error = ref(""),
  activityRoot = ref(null),
  overlayTop = ref(113),
  field = ref(null),
  fieldHeight = ref(400),
  floats = ref([]);
const pending = ref(new Set()),
  claimed = ref(new Set());
const dismissedStart = ref("");
let offset = 0,
  clockTimer,
  pollTimer,
  closedRound = "",
  attemptedRound = "",
  stopped = false;
const controller = new AbortController();
const options = { signal: controller.signal };
const active = computed(
  () =>
    state.value?.phase === "active" &&
    Date.parse(state.value.round.end_at) > now.value,
);
const cfg = computed(() =>
  joinedRound.value?.id === state.value?.round?.id && joinConfig.value
    ? joinConfig.value
    : state.value?.config,
);
const countdown = computed(() =>
  state.value?.enabled && !active.value && state.value.next_start_at
    ? Math.max(
        0,
        Math.ceil((Date.parse(state.value.next_start_at) - now.value) / 1000),
      )
    : 0,
);
const countdownVisible = computed(
  () =>
    countdown.value > 0 &&
    countdown.value <= 10 &&
    dismissedStart.value !== state.value?.next_start_at,
);
const remaining = computed(() =>
  active.value
    ? Math.max(
        0,
        Math.ceil((Date.parse(state.value.round.end_at) - now.value) / 1000),
      )
    : 0,
);
const downloadUrl = computed(
  () =>
    props.config.app_download_url ||
    "https://orange.lingzidev.com/download/orange-rewards.apk",
);
const appRequired = computed(() => props.config.device_trust_enabled);
const visiblePackets = computed(() =>
  packets.value
    .filter(
      (packet) =>
        packet.status === "available" &&
        !claimed.value.has(packet.id) &&
        Date.parse(packet.reveal_at) <= now.value &&
        now.value < Date.parse(packet.expires_at),
    )
    .slice(0, cfg.value?.max_visible || 0),
);
const clockText = computed(() => {
  const value = countdown.value;
  return [Math.floor(value / 3600), Math.floor(value / 60) % 60, value % 60]
    .map((n) => String(n).padStart(2, "0"))
    .join(":");
});
const vRainDrop = {
  mounted(element, binding) {
    element.style.animationDelay =
      -Math.max(0, now.value - Date.parse(binding.value.reveal_at)) / 1000 +
      "s";
  },
};
function syncClock(serverNow) {
  offset = Date.parse(serverNow) - Date.now();
  now.value = Date.now() + offset;
}
function showError(failure) {
  if (failure.name !== "AbortError") error.value = failure.message;
}
async function refreshStatus() {
  try {
    state.value = await props.api("/rain/status", options);
    syncClock(state.value.server_now);
    if (
      active.value &&
      closedRound !== state.value.round.id &&
      attemptedRound !== state.value.round.id
    )
      enterRound();
    if (
      !active.value &&
      joinedRound.value &&
      Date.parse(joinedRound.value.end_at) <= now.value
    ) {
      rainVisible.value = false;
      packets.value = [];
      joinedRound.value = null;
    }
  } catch (failure) {
    showError(failure);
  }
}
async function enterRound() {
  if (!active.value || joining.value) return;
  closedRound = "";
  attemptedRound = state.value.round.id;
  rainVisible.value = true;
  error.value = "";
  if (appRequired.value) return;
  if (joinedRound.value?.id === state.value.round.id) {
    await measureField();
    return;
  }
  joining.value = true;
  try {
    const result = await props.api("/rain/join", {
      ...options,
      method: "POST",
      body: {},
    });
    syncClock(result.server_now);
    joinedRound.value = result.round;
    joinConfig.value = result.config;
    packets.value = result.packets.map((packet, index) => ({
      ...packet,
      index,
    }));
    pending.value.clear();
    claimed.value.clear();
    await measureField();
  } catch (failure) {
    showError(failure);
  } finally {
    joining.value = false;
  }
}
function closeRain() {
  closedRound = state.value?.round?.id || "";
  rainVisible.value = false;
}
function closeCountdown() {
  dismissedStart.value = state.value.next_start_at;
}
async function measureField() {
  await nextTick();
  if (activityRoot.value) {
    const surface = activityRoot.value.closest(".app-surface");
    overlayTop.value =
      activityRoot.value.getBoundingClientRect().bottom -
      surface.getBoundingClientRect().top;
  }
  if (field.value) fieldHeight.value = field.value.clientHeight;
}
function packetStyle(packet) {
  return {
    left: 7 + ((packet.index * 0.61803398875) % 1) * 67 + "%",
    "--fall-duration":
      (Date.parse(packet.expires_at) - Date.parse(packet.reveal_at)) / 1000 +
      "s",
    "--fall-distance": fieldHeight.value + 150 + "px",
    "--turn": -13 + (packet.index % 7) * 4 + "deg",
    "--rest-y": 5 + (packet.index % 4) * 20 + "%",
  };
}
async function claim(packet, event) {
  if (pending.value.has(packet.id) || claimed.value.has(packet.id)) return;
  if (appRequired.value) {
    location.assign(downloadUrl.value);
    return;
  }
  pending.value.add(packet.id);
  error.value = "";
  const fieldBox = field.value.getBoundingClientRect();
  const point = {
    x: event.clientX - fieldBox.left,
    y: event.clientY - fieldBox.top,
  };
  try {
    await prepareCoinSound();
    const result = await props.api("/rain/claim", {
      ...options,
      method: "POST",
      body: { packet_id: packet.id },
    });
    claimed.value.add(packet.id);
    if (result.kind === "large") {
      emit("large", result.task);
    } else {
      emit("credit", result);
      if (
        result.credited !== false &&
        !result.already_claimed &&
        result.reward > 0
      ) {
        playCoinArrival();
        floats.value.push({ id: packet.id, reward: result.reward, ...point });
        setTimeout(() => {
          if (!stopped)
            floats.value = floats.value.filter((item) => item.id !== packet.id);
        }, 1400);
      }
    }
  } catch (failure) {
    showError(failure);
  } finally {
    pending.value.delete(packet.id);
  }
}
watch(countdown, (seconds) => {
  if (
    seconds === 0 &&
    state.value?.phase === "waiting" &&
    state.value.next_start_at &&
    Date.parse(state.value.next_start_at) <= now.value
  )
    refreshStatus();
});
watch(active, (value) => {
  if (!value) {
    rainVisible.value = false;
    if (state.value?.phase === "active") refreshStatus();
  }
});
watch(rainVisible, (value) => {
  if (value) measureField();
});
watch(() => state.value?.enabled, measureField);
onMounted(() => {
  refreshStatus();
  clockTimer = setInterval(() => (now.value = Date.now() + offset), 200);
  pollTimer = setInterval(() => {
    refreshStatus();
  }, 5000);
  window.addEventListener("resize", measureField);
});
onUnmounted(() => {
  stopped = true;
  controller.abort();
  clearInterval(clockTimer);
  clearInterval(pollTimer);
  window.removeEventListener("resize", measureField);
});
</script>

<template>
  <div
    ref="activityRoot"
    class="hall-activity"
    :class="{ 'has-rain': state?.enabled }"
    :style="{ '--rain-panel-top': overlayTop + 'px' }"
  >
    <button
      v-if="state?.enabled"
      class="rain-banner"
      @click="active ? enterRound() : refreshStatus()"
    >
      <img src="/assets/rain-small.png" alt="" />
      <div>
        <b>{{ active ? "红包雨正在下" : "距下场红包雨" }}</b
        ><span>{{
          active ? "点开，接住今天的小欢喜" : "好事就要从天而降"
        }}</span>
      </div>
      <strong v-if="!active">{{ clockText }}</strong
      ><strong v-else class="rain-banner-enter"
        >去接红包<Icon name="arrow" :size="12"
      /></strong>
    </button>
    <div v-if="error && !rainVisible" class="rain-inline-error" role="alert">
      {{ error }}
    </div>
    <div
      v-if="countdownVisible"
      class="rain-countdown-overlay"
      role="dialog"
      aria-modal="true"
      aria-label="红包雨即将开始"
    >
      <img src="/assets/rain-sky.png" class="countdown-sky" alt="" /><button
        class="rain-close"
        @click="closeCountdown"
        aria-label="关闭红包雨倒计时"
      >
        <Icon name="close" :size="20" /></button
      ><span class="rain-kicker">好事，就要从天而降</span>
      <h2>红包雨要来啦</h2>
      <img
        src="/assets/rain-large.png"
        class="countdown-large"
        alt="大奖红包"
      /><strong :key="countdown" class="countdown-number">{{
        countdown
      }}</strong>
      <p>准备接住这场小欢喜</p>
      <button class="countdown-stay" @click="closeCountdown">
        先逛逛大厅<Icon name="arrow" :size="14" />
      </button>
    </div>
    <section
      v-if="rainVisible && active"
      class="rain-overlay"
      :style="{ '--rain-opacity': cfg.overlay_opacity }"
      role="dialog"
      aria-modal="true"
      aria-label="红包雨"
    >
      <div class="rain-backdrop"></div>
      <img src="/assets/rain-sky.png" class="rain-sky" alt="" />
      <header>
        <div><span>小橙有喜 · 红包雨</span><b>接住今天的小幸运</b></div>
        <span class="rain-time">{{ remaining }}s</span
        ><button class="rain-close" @click="closeRain" aria-label="关闭红包雨">
          <Icon name="close" :size="19" />
        </button>
      </header>
      <template v-if="appRequired"
        ><div class="rain-app-required">
          <img src="/assets/rain-large.png" alt="红包雨大奖" />
          <h2>红包雨已经开始</h2>
          <p>在 App 里，接住这场小欢喜</p>
          <a class="rain-app-link" :href="downloadUrl"
            >打开App接红包<Icon name="arrow" :size="17"
          /></a></div></template
      ><template v-else
        ><div ref="field" class="rain-field">
          <button
            v-for="packet in visiblePackets"
            :key="packet.id"
            v-rain-drop="packet"
            class="rain-falling-packet"
            :class="{
              large: packet.kind === 'large',
              claiming: pending.has(packet.id),
            }"
            :style="packetStyle(packet)"
            :disabled="pending.has(packet.id)"
            :aria-label="packet.kind === 'large' ? '大奖红包' : '小红包'"
            :data-packet-id="packet.id"
            @click="claim(packet, $event)"
          >
            <img
              :src="
                packet.kind === 'large'
                  ? '/assets/rain-large.png'
                  : '/assets/rain-small.png'
              "
              :alt="packet.kind === 'large' ? '大奖红包' : '小红包'"
            /><span
              v-if="pending.has(packet.id)"
              class="packet-claiming-dot"
            ></span></button
          ><span
            v-for="item in floats"
            :key="item.id"
            class="rain-reward-float"
            :style="{ left: item.x + 'px', top: item.y + 'px' }"
            >+{{ item.reward }}<small>金币</small></span
          >
          <p v-if="joining" class="rain-field-loading">小红包正在赶来…</p>
        </div>
        <footer>
          <span><img src="/assets/rain-small.png" alt="" />小包点开领金币</span
          ><span
            ><img src="/assets/rain-large.png" alt="" />大包藏着大惊喜</span
          >
        </footer></template
      >
      <div v-if="error" class="rain-claim-error" role="alert">
        {{ error
        }}<button v-if="!joinedRound" @click="enterRound">重新参加</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.hall-activity {
  position: static;
  flex-shrink: 0;
}
.hall-activity.has-rain {
  padding: 0 18px 12px;
}
.rain-banner {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  background: linear-gradient(110deg, #fff0cf, #ffe2b6);
  border: 1px solid #f7d29b;
  border-radius: 14px;
  padding: 8px 11px;
  text-align: left;
  margin: 0;
  box-shadow: 0 4px 10px #d5984010;
}
.rain-banner > img {
  width: 37px;
  height: 39px;
  object-fit: contain;
  transform: rotate(-9deg);
  filter: drop-shadow(0 3px 3px #b47c3920);
}
.rain-banner > div {
  flex: 1;
}
.rain-banner b {
  display: block;
  font-size: 11px;
  color: #a56a32;
  margin-bottom: 4px;
}
.rain-banner span {
  display: block;
  font-size: 8px;
  color: #c29661;
}
.rain-banner > strong {
  color: #ba7b36;
  font-family: ui-monospace, Consolas, monospace;
  font-size: 16px;
  letter-spacing: 1px;
  font-weight: 600;
  white-space: nowrap;
}
.rain-banner .rain-banner-enter {
  display: flex;
  align-items: center;
  gap: 3px;
  font-family: inherit;
  font-size: 10px;
  letter-spacing: 0;
}
.rain-inline-error {
  padding: 9px 12px;
  margin: 10px 0;
  border: 1px solid #edcdb1;
  background: #fff2e6;
  color: #b78358;
  border-radius: 11px;
  font-size: 10px;
  line-height: 1.7;
}
.rain-countdown-overlay {
  position: absolute;
  inset: var(--rain-panel-top, 113px) 0 84px;
  z-index: 22;
  background: radial-gradient(
    ellipse at 50% 30%,
    #ffe3b6,
    #ffe9cf 48%,
    #fff3e5 85%
  );
  display: flex;
  align-items: center;
  flex-direction: column;
  justify-content: center;
  text-align: center;
  overflow: hidden;
  padding: 24px 22px 20px;
  animation: rainFade 0.25s ease;
}
.countdown-sky {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  width: 100%;
  height: 180px;
  object-fit: cover;
  object-position: top;
  pointer-events: none;
}
.rain-close {
  display: grid;
  place-items: center;
  border-radius: 50%;
  width: 31px;
  height: 31px;
  color: #a97643;
  background: #fff2dcaa;
  border: 1px solid #edc79b80;
  flex-shrink: 0;
}
.rain-countdown-overlay > .rain-close {
  position: absolute;
  right: 17px;
  top: 17px;
}
.rain-kicker {
  font-size: 10px;
  letter-spacing: 2px;
  color: #bd8a50;
  position: relative;
  margin-top: 13px;
}
.rain-countdown-overlay > h2 {
  font-size: 29px;
  font-weight: 800;
  letter-spacing: 1px;
  color: #a25128;
  margin: 14px 0 -8px;
  position: relative;
}
.countdown-large {
  width: 165px;
  height: 165px;
  flex-shrink: 0;
  object-fit: contain;
  position: relative;
  filter: drop-shadow(0 9px 15px #c48e3b33);
  animation: rainGrandFloat 2.5s ease-in-out infinite;
}
.countdown-number {
  position: relative;
  font-size: 64px;
  line-height: 1.1;
  font-weight: 800;
  color: #d86a32;
  text-shadow: 0 4px 0 #fff6df;
  animation: countdownPop 0.65s cubic-bezier(0.2, 0.8, 0.2, 1);
}
.rain-countdown-overlay > p {
  font-size: 11px;
  color: #b98a55;
  letter-spacing: 1px;
  margin-top: 12px;
}
.countdown-stay {
  display: flex;
  gap: 5px;
  align-items: center;
  color: #b39168;
  font-size: 10px;
  margin-top: 17px;
  border-bottom: 1px solid #dfbc91;
  padding: 7px 0;
}
.rain-overlay {
  position: absolute;
  inset: var(--rain-panel-top, 113px) 0 84px;
  z-index: 20;
  overflow: hidden;
  animation: rainFade 0.3s ease;
}
.rain-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(82, 39, 20, var(--rain-opacity));
  backdrop-filter: blur(2px);
}
.rain-sky {
  position: absolute;
  left: 0;
  right: 0;
  top: -12px;
  width: 100%;
  height: 260px;
  object-fit: cover;
  object-position: top;
  pointer-events: none;
}
.rain-overlay > header {
  display: flex;
  align-items: center;
  gap: 10px;
  position: relative;
  z-index: 2;
  margin: 24px 17px 0;
}
.rain-overlay > header > div {
  flex: 1;
}
.rain-overlay > header > div > span {
  display: block;
  color: #ffedbf;
  font-size: 8px;
  letter-spacing: 1px;
  text-shadow: 0 1px 3px #8f53283b;
}
.rain-overlay > header b {
  display: block;
  font-size: 19px;
  color: #fff2ce;
  margin-top: 5px;
  text-shadow: 0 2px 5px #8c461f75;
  letter-spacing: 0.2px;
}
.rain-time {
  background: #fff0cba3;
  border: 1px solid #ffe3aa9e;
  color: #995c25;
  border-radius: 100px;
  padding: 5px 9px;
  font-size: 10px;
  font-weight: 600;
}
.rain-overlay .rain-close {
  background: #fff4d08f;
  color: #986127;
  border-color: #ffe5b180;
  width: 28px;
  height: 28px;
}
.rain-field {
  position: absolute;
  inset: 93px 0 64px;
  overflow: hidden;
  z-index: 3;
}
.rain-falling-packet {
  position: absolute;
  top: -115px;
  width: 74px;
  height: 82px;
  padding: 0;
  border: 0;
  background: none;
  opacity: 0;
  filter: drop-shadow(0 6px 6px #8e3c1c2b);
  animation: packetFall var(--fall-duration) linear both;
  touch-action: manipulation;
}
.rain-falling-packet.large {
  width: 96px;
  height: 105px;
  filter: drop-shadow(0 0 12px #ffdb6f7a);
}
.rain-falling-packet > img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  pointer-events: none;
}
.rain-falling-packet.claiming {
  animation-play-state: paused;
  filter: drop-shadow(0 0 15px #fff0a8);
  opacity: 1;
  cursor: wait;
}
.rain-falling-packet:active {
  scale: 0.93;
}
.packet-claiming-dot {
  position: absolute;
  left: 50%;
  top: 46%;
  transform: translate(-50%, -50%);
  width: 16px;
  height: 16px;
  background: #fff0c9;
  border-radius: 50%;
  box-shadow: 0 0 14px #ffecb0;
  animation: pendingPulse 0.8s ease-in-out infinite;
}
.rain-reward-float {
  position: absolute;
  font-size: 28px;
  line-height: 1;
  font-weight: 800;
  color: #ffe49b;
  text-shadow: 0 2px 4px #874319;
  white-space: nowrap;
  pointer-events: none;
  animation: rainCoinUp 1.35s ease-out both;
  z-index: 8;
}
.rain-reward-float > small {
  font-size: 10px;
  margin-left: 4px;
  font-weight: 500;
}
.rain-field-loading {
  position: absolute;
  top: 36%;
  left: 0;
  right: 0;
  text-align: center;
  font-size: 12px;
  color: #ffe8bf;
}
.rain-overlay > footer {
  position: absolute;
  bottom: 15px;
  left: 16px;
  right: 16px;
  display: flex;
  justify-content: center;
  gap: 18px;
  color: #ffe6bb;
  font-size: 9px;
  z-index: 2;
}
.rain-overlay > footer > span {
  display: flex;
  align-items: center;
  gap: 5px;
}
.rain-overlay > footer img {
  width: 28px;
  height: 30px;
  object-fit: contain;
}
.rain-claim-error {
  position: absolute;
  left: 22px;
  right: 22px;
  bottom: 62px;
  z-index: 10;
  padding: 10px 12px;
  font-size: 10px;
  line-height: 1.8;
  border: 1px solid #f9d6a7a3;
  background: #fff3dce8;
  color: #aa713e;
  border-radius: 12px;
  box-shadow: 0 5px 16px #8f542c1c;
}
.rain-claim-error button {
  display: block;
  font-size: 10px;
  color: #c28949;
  text-decoration: underline;
  padding: 6px 0 0;
}
.rain-app-required {
  position: absolute;
  inset: 89px 20px 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  z-index: 3;
  text-align: center;
}
.rain-app-required > img {
  width: 203px;
  height: 203px;
  object-fit: contain;
  animation: rainGrandFloat 3s ease-in-out infinite;
  filter: drop-shadow(0 6px 15px #b1682573);
}
.rain-app-required > h2 {
  font-size: 24px;
  color: #fff0c1;
  letter-spacing: 0.5px;
  margin-top: 5px;
  text-shadow: 0 2px 4px #8b4c28;
}
.rain-app-required > p {
  font-size: 11px;
  color: #ffe3ad;
  margin-top: 12px;
}
.rain-app-link {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 8px;
  min-height: 45px;
  width: 100%;
  max-width: 280px;
  margin-top: 27px;
  background: linear-gradient(115deg, #ffe7a8, #ffc868);
  border: 1px solid #ffe4a1;
  color: #a36b28;
  font-size: 13px;
  font-weight: 700;
  border-radius: 100px;
  box-shadow: 0 5px 16px #9b4b2829;
}
@keyframes packetFall {
  0% {
    transform: translateY(0) rotate(var(--turn));
    opacity: 0;
  }
  8% {
    opacity: 1;
  }
  90% {
    opacity: 1;
  }
  100% {
    transform: translateY(var(--fall-distance)) rotate(calc(var(--turn) * -1));
    opacity: 0;
  }
}
@keyframes rainCoinUp {
  0% {
    transform: translate(-50%, 0) scale(0.6);
    opacity: 0;
  }
  15% {
    opacity: 1;
    transform: translate(-50%, -12px) scale(1.1);
  }
  80% {
    opacity: 1;
  }
  100% {
    transform: translate(-50%, -100px) scale(0.9);
    opacity: 0;
  }
}
@keyframes countdownPop {
  0% {
    transform: scale(0.72);
    opacity: 0.35;
  }
  30% {
    transform: scale(1.08);
    opacity: 1;
  }
  100% {
    transform: scale(1);
  }
}
@keyframes rainGrandFloat {
  50% {
    transform: translateY(-7px) rotate(2deg);
  }
}
@keyframes pendingPulse {
  50% {
    opacity: 0.4;
    scale: 0.75;
  }
}
@keyframes rainFade {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@media (max-height: 760px) {
  .rain-countdown-overlay {
    padding-top: 15px;
  }
  .countdown-large {
    width: 163px;
    height: 163px;
  }
  .countdown-number {
    font-size: 58px;
  }
  .rain-countdown-overlay > h2 {
    font-size: 25px;
  }
  .countdown-stay {
    margin-top: 14px;
  }
  .rain-app-required > img {
    width: 163px;
    height: 163px;
  }
  .rain-app-required > h2 {
    font-size: 20px;
  }
  .rain-app-link {
    margin-top: 20px;
  }
}
@media (prefers-reduced-motion: reduce) {
  .rain-falling-packet {
    animation: none !important;
    opacity: 1 !important;
    top: var(--rest-y) !important;
    transform: rotate(var(--turn)) !important;
  }
  .countdown-large,
  .countdown-number,
  .rain-app-required > img,
  .packet-claiming-dot,
  .rain-reward-float {
    animation: none !important;
  }
  .rain-reward-float {
    display: none;
  }
  .rain-overlay,
  .rain-countdown-overlay {
    animation: none !important;
  }
}
.rain-falling-packet.large > img {
  animation: rainLargePulse 1.25s ease-in-out infinite;
}
@keyframes rainLargePulse {
  0%,
  100% {
    transform: scale(1);
    filter: brightness(1) drop-shadow(0 0 4px #ffdc7399);
  }
  50% {
    transform: scale(1.08);
    filter: brightness(1.12) drop-shadow(0 0 12px #ffe69abb);
  }
}
@media (prefers-reduced-motion: reduce) {
  .rain-falling-packet.large > img {
    animation: none !important;
  }
}
</style>
