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
const tiers = [
  { id: "new_device", name: "新设备", icon: "spark", limit: true },
  { id: "active_device", name: "活跃设备", icon: "gift", limit: true },
  { id: "mature_device", name: "成熟设备", icon: "coin", limit: false },
  { id: "unknown_device", name: "未知设备", icon: "shield", limit: false },
];
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function refresh() {
  policy.value = null;
  error.value = "";
  try {
    policy.value = await props.api("/admin/reward-policy", {
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
    policy.value = await props.api("/admin/reward-policy", {
      method: "PUT",
      body: policy.value,
      signal: controller.signal,
    });
    notice.value = "奖励策略已保存";
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
  <section class="admin-card reward-policy">
    <header class="reward-policy-heading">
      <div>
        <h2>激励红包阶梯奖励</h2>
      </div>
      <span v-if="policy" class="policy-version"
        >策略版本 {{ policy.policy_version }}</span
      >
    </header>
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="policy-notice">
      <Icon name="check" :size="16" />{{ notice }}
    </div>
    <p v-if="!policy && !error" class="policy-loading">正在读取奖励策略…</p>
    <form v-if="policy" @submit.prevent="save">
      <div class="policy-switch-row">
        <div>
          <b>启用阶梯奖励</b>
          <p>按设备参与情况分档发放金币</p>
        </div>
        <label class="policy-switch"
          ><input
            v-model="policy.enabled"
            type="checkbox"
            aria-label="启用阶梯奖励" /><span></span
        ></label>
      </div>
      <div v-if="!policy.enabled" class="policy-fixed">
        <Icon name="coin" :size="23" />
        <label
          >固定奖励（金币）<input
            v-model.number="policy.fixed_coins"
            type="number"
            min="1"
            required
        /></label>
      </div>
      <template v-else>
        <div class="policy-tier-grid">
          <article
            v-for="tier in tiers"
            :key="tier.id"
            class="policy-tier"
            :class="tier.id"
          >
            <h3>
              <span><Icon :name="tier.icon" :size="17" />{{ tier.name }}</span
              ><small>{{
                tier.id === "unknown_device"
                  ? "保守档"
                  : tier.id === "mature_device"
                    ? "其他已验签设备"
                    : "同时满足以下条件"
              }}</small>
            </h3>
            <div v-if="tier.limit" class="policy-fields">
              <label
                >设备天龄 ≤<input
                  v-model.number="policy[tier.id].max_days"
                  :aria-label="tier.name + '天龄上限'"
                  type="number"
                  min="1"
                  required
                /><small>天</small></label
              >
              <label
                >完成与在途次数 &lt;<input
                  v-model.number="policy[tier.id].max_completed"
                  :aria-label="tier.name + '完成次数上限'"
                  type="number"
                  min="1"
                  required
                /><small>次</small></label
              >
            </div>
            <div class="policy-fields policy-coins">
              <label
                >最低金币<input
                  v-model.number="policy[tier.id].min_coins"
                  :aria-label="tier.name + '最低金币'"
                  type="number"
                  min="1"
                  required
              /></label>
              <label
                >最高金币<input
                  v-model.number="policy[tier.id].max_coins"
                  :aria-label="tier.name + '最高金币'"
                  type="number"
                  min="1"
                  required
              /></label>
            </div>
          </article>
        </div>
        <p class="policy-rule">
          天龄自首次验签起算，已完成与有效在途会话跨账号累计名额。新设备档优先，其次活跃档；无法确认设备时使用保守档。
        </p>
      </template>
      <div class="policy-footer">
        <label
          >报价有效期（秒）<input
            v-model.number="policy.quote_ttl_seconds"
            type="number"
            min="30"
            max="300"
            required
        /></label>
        <button class="primary" :disabled="saving">
          <Icon name="check" :size="16" />{{
            saving ? "保存中…" : "保存奖励策略"
          }}
        </button>
      </div>
    </form>
  </section>
</template>

<style scoped>
.reward-policy {
  max-width: 1000px;
  padding: 18px 20px;
  margin-bottom: 12px;
}
.reward-policy-heading {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: center;
  margin-bottom: 18px;
}
.reward-policy-heading h2 {
  font-size: 18px;
  color: #775337;
}
.policy-version {
  font-size: 12.5px;
  color: #b18a61;
  padding: 7px 10px;
  background: #fcf1df;
  border-radius: 7px;
  white-space: nowrap;
}
.policy-loading {
  font-size: 13.5px;
  color: #ad9272;
  padding: 20px 0;
}
.policy-switch-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 18px;
  border-top: 1px solid #f2e8d9;
  padding: 19px 0 22px;
}
.policy-switch-row b {
  font-size: 13.5px;
  color: #8a6746;
  font-weight: 600;
}
.policy-switch-row p {
  font-size: 12.5px;
  color: #b59a7a;
  margin-top: 7px;
}
.policy-switch {
  position: relative;
  display: block;
  width: 40px;
  height: 23px;
  flex-shrink: 0;
  margin: 0;
}
.policy-switch input {
  opacity: 0;
  width: 100%;
  height: 100%;
  position: absolute;
  margin: 0;
  cursor: pointer;
}
.policy-switch > span {
  pointer-events: none;
  display: block;
  height: 23px;
  border-radius: 30px;
  background: #dfd4c3;
  transition: background 0.2s;
}
.policy-switch > span::after {
  content: "";
  position: absolute;
  height: 17px;
  width: 17px;
  top: 3px;
  left: 3px;
  background: #fff;
  border-radius: 50%;
  transition: transform 0.2s;
  box-shadow: 0 1px 3px #98714824;
}
.policy-switch input:checked + span {
  background: #ec985b;
}
.policy-switch input:checked + span::after {
  transform: translateX(17px);
}
.policy-switch input:focus-visible + span {
  outline: 2px solid #d18448;
  outline-offset: 3px;
}
.policy-tier-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}
.policy-tier {
  border: 0;
  border-radius: 13px;
  background: #fffbf5;
  padding: 18px;
}
.policy-tier.new_device {
  background: linear-gradient(125deg, #fff8ec, #fff0e2);
  border-color: #f0d2af;
}
.policy-tier.unknown_device {
  background: #f8f6f1;
  border-color: #e6dfd3;
}
.policy-tier h3 {
  font-size: 13.5px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: space-between;
  align-items: center;
  color: #987044;
  margin-bottom: 18px;
}
.policy-tier h3 > span {
  display: inline-flex;
  gap: 7px;
  align-items: center;
}
.policy-tier h3 > small {
  font-size: 12px;
  color: #b29a7a;
  font-weight: 400;
}
.policy-fields {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.policy-fields label,
.policy-footer label,
.policy-fixed label {
  font-size: 12.5px;
  color: #a18666;
  margin: 0;
  position: relative;
}
.policy-fields input,
.policy-footer input,
.policy-fixed input {
  font-size: 14px;
  border: 1px solid #e8dcc9;
  background: #fffefb;
  border-radius: 8px;
  margin-top: 7px;
  padding: 11px 12px;
}
.policy-fields label > small {
  position: absolute;
  right: 12px;
  bottom: 12px;
  font-size: 12px;
  color: #b49e82;
}
.policy-fields:not(.policy-coins) input {
  padding-right: 30px;
}
.policy-coins {
  margin-top: 14px;
}
.policy-rule {
  font-size: 12.5px;
  line-height: 1.9;
  color: #b29a7b;
  margin: 15px 0 0;
}
.policy-footer {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 20px;
  border-top: 1px solid #f0e6d7;
  padding-top: 22px;
  margin-top: 24px;
}
.policy-footer label {
  max-width: 170px;
}
.policy-footer .primary {
  border-radius: 9px;
  padding: 12px 19px;
  font-size: 13px;
  white-space: nowrap;
}
.policy-fixed {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 18px;
  border: 0;
  border-radius: 12px;
  background: #fff9ed;
  color: #d39b53;
}
.policy-fixed label {
  max-width: 190px;
}
.policy-notice {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-bottom: 16px;
  font-size: 13px;
  color: #8f9a5d;
}
@media (max-width: 700px) {
  .reward-policy {
    padding: 20px;
  }
  .policy-tier-grid {
    grid-template-columns: 1fr;
  }
  .policy-footer {
    flex-wrap: wrap;
  }
}
</style>
