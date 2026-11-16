<script setup>
import { computed, ref } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ activity: Object, requestSpin: Function });
const open = ref(false),
  spinning = ref(false),
  result = ref(null);
const enabled = computed(() => Boolean(props.activity?.enabled));
const opportunities = computed(() => props.activity?.opportunities || []);
const canSpin = computed(() => enabled.value && opportunities.value.length > 0);
function show() {
  if (enabled.value) open.value = true;
}
async function spin() {
  if (spinning.value || !canSpin.value) return;
  spinning.value = true;
  result.value = null;
  try {
    result.value = props.requestSpin
      ? await props.requestSpin()
      : { status: "unavailable", message: "活动暂不可用" };
  } catch (failure) {
    result.value = { status: "error", message: failure.message };
  } finally {
    spinning.value = false;
  }
}
</script>
<template>
  <button v-if="enabled" class="activity-entry" @click="show">
    <Icon name="spark" :size="16" />{{ activity.title }}
  </button>
  <div v-if="open" class="activity-overlay" @click.self="open = false">
    <section
      class="activity-sheet"
      role="dialog"
      aria-modal="true"
      aria-label="活动"
    >
      <button
        class="activity-close"
        aria-label="关闭活动"
        @click="open = false"
      >
        <Icon name="close" /></button
      ><img :src="activity.hero_image || '/assets/reward.png'" alt="奖励活动" />
      <h2>{{ activity.title }}</h2>
      <p>{{ activity.subtitle }}</p>
      <div class="activity-rules">
        <span>今日剩余 {{ opportunities.length }} 次</span
        ><span
          >金币门槛
          {{
            activity.arrival_threshold_coins ?? activity.arrival_threshold ?? 0
          }}</span
        ><span>结算等待 {{ activity.settle_delay_seconds ?? 0 }} 秒</span>
      </div>
      <div class="activity-wheel-large" :class="{ spinning }">
        <span>✦</span><b>{{ activity.entry_label || "转盘" }}</b>
      </div>
      <button
        class="primary full"
        :disabled="spinning || !canSpin"
        @click="spin"
      >
        {{ spinning ? "结果确认中…" : canSpin ? "开始转盘" : "今日次数已用完" }}
      </button>
      <div v-if="result" class="activity-result">
        <Icon
          :name="
            result.status === 'settled' || result.status === 'credited'
              ? 'check'
              : 'bell'
          "
          :size="18"
        /><b>{{
          result.status === "settled" || result.status === "credited"
            ? `已到账 ${result.prize_coins} 金币`
            : result.status === "pending"
              ? "结果待确认"
              : result.message || "活动暂不可用"
        }}</b>
      </div>
      <small class="activity-disclaimer"
        >实际奖励与到账状态以服务端返回为准</small
      >
    </section>
  </div>
</template>
<style scoped>
.activity-entry {
  display: flex;
  align-items: center;
  gap: 6px;
  border: 1px solid #f0c99f;
  background: #fff4df;
  color: #cf8750;
  border-radius: 9px;
  padding: 8px 11px;
  font-size: 10px;
}
.activity-overlay {
  position: fixed;
  inset: 0;
  z-index: 80;
  background: #50382c80;
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}
.activity-sheet {
  width: 350px;
  max-width: 100%;
  background: #fffaf2;
  border: 1px solid #efd5b6;
  border-radius: 20px;
  padding: 27px;
  text-align: center;
  position: relative;
  box-shadow: 0 16px 45px #5d351b33;
}
.activity-close {
  position: absolute;
  right: 16px;
  top: 15px;
  color: #b49878;
}
.activity-sheet > img {
  width: 128px;
  margin: 4px auto -3px;
}
.activity-sheet h2 {
  font-size: 22px;
  color: #875334;
}
.activity-sheet > p {
  font-size: 11px;
  color: #b18c6e;
  line-height: 1.8;
  margin: 9px 0;
}
.activity-rules {
  display: flex;
  justify-content: center;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 9px;
  color: #b38e69;
}
.activity-rules span {
  padding: 6px 8px;
  background: #f9eddd;
  border-radius: 6px;
}
.activity-wheel-large {
  width: 145px;
  height: 145px;
  margin: 21px auto;
  border-radius: 50%;
  border: 8px solid #fff2da;
  background: conic-gradient(#ffb874, #f5784b, #ffd58f, #e97b52, #ffb874);
  box-shadow: 0 10px 22px #9c542a27;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: #fff8e8;
}
.activity-wheel-large span {
  font-size: 29px;
}
.activity-wheel-large b {
  font-size: 14px;
}
.activity-wheel-large.spinning {
  animation: activitySpin 0.9s ease-in-out;
}
@keyframes activitySpin {
  to {
    transform: rotate(360deg);
  }
}
.activity-result {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  color: #98a77f;
  font-size: 11px;
  margin-top: 16px;
}
.activity-disclaimer {
  display: block;
  color: #c0a78d;
  font-size: 8px;
  margin-top: 17px;
}
@media (prefers-reduced-motion: reduce) {
  .activity-wheel-large.spinning {
    animation: none;
  }
}
</style>
