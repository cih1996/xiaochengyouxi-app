<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const rules = ref(null),
  error = ref(""),
  notice = ref(""),
  saving = ref(false);
const controller = new AbortController();
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try {
    rules.value = await props.api("/admin/withdraw-rules", { signal: controller.signal });
  } catch (e) {
    fail(e);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    rules.value = await props.api("/admin/withdraw-rules", {
      method: "PUT",
      body: { tiers: rules.value.tiers },
      signal: controller.signal,
    });
    notice.value = "提现档位已保存，App 下次打开提现时生效";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
function addTier() {
  rules.value.tiers.push({
    id: `t${Date.now().toString(36)}`,
    amount_yuan: 10,
    daily_limit: 0,
    lifetime_limit: 0,
    enabled: true,
    note: "",
  });
}
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <section class="admin-card rules-admin">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="rules-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <p v-if="!rules && !error" class="rules-muted">正在读取提现档位…</p>
    <form v-if="rules" @submit.prevent="save">
      <header>
        <div>
          <p>用户只能选择下面启用的固定金额。余额满档位金额才可提；次数填 0 表示不限。被驳回退回的申请不占次数。</p>
        </div>
        <button type="button" class="outline" @click="addTier">
          <Icon name="wallet" :size="14" />添加档位
        </button>
      </header>
      <div class="rules-table">
        <div class="rules-row rules-head">
          <span>启用</span><span>金额（元）</span><span>每天次数</span><span>累计次数</span><span>说明（选填）</span><span></span>
        </div>
        <div v-for="(tier, index) in rules.tiers" :key="tier.id" class="rules-row">
          <label class="rules-check"><input v-model="tier.enabled" type="checkbox" :aria-label="`启用 ${tier.amount_yuan} 元档`" /></label>
          <input v-model.number="tier.amount_yuan" type="number" min="0.1" max="5000" step="0.01" required aria-label="金额" />
          <input v-model.number="tier.daily_limit" type="number" min="0" max="100" required aria-label="每天次数" />
          <input v-model.number="tier.lifetime_limit" type="number" min="0" max="1000" required aria-label="累计次数" />
          <input v-model="tier.note" maxlength="20" placeholder="如：新人专享" aria-label="说明" />
          <button type="button" class="rules-remove" :aria-label="`删除 ${tier.amount_yuan} 元档`" @click="rules.tiers.splice(index, 1)">删除</button>
        </div>
        <p v-if="!rules.tiers.length" class="rules-muted">没有档位时 App 会提示“提现暂未开放”。</p>
      </div>
      <p class="rules-muted">示例：0.5 元每天 1 次；20 元不限次数但要余额满 20 元；新人 0.3 元累计 1 次。</p>
      <button class="primary" :disabled="saving"><Icon name="check" :size="17" />保存档位</button>
    </form>
  </section>
</template>

<style scoped>
.rules-admin form > header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  margin-bottom: 16px;
}
.rules-admin h2 {
  margin: 0 0 6px;
}
.rules-admin header p,
.rules-muted {
  color: #9a8274;
  font-size: 14px;
  line-height: 1.7;
  margin: 0;
}
.rules-table {
  display: grid;
  gap: 8px;
  margin-bottom: 14px;
}
.rules-row {
  display: grid;
  grid-template-columns: 44px 1fr 1fr 1fr 1.6fr 56px;
  gap: 8px;
  align-items: center;
}
.rules-head span {
  font-size: 13.5px;
  color: #9a8274;
}
.rules-row input:not([type="checkbox"]) {
  min-width: 0;
}
.rules-check {
  display: grid;
  place-items: center;
}
.rules-remove {
  border: 0;
  background: none;
  color: #d0533a;
  cursor: pointer;
}
.rules-notice {
  display: flex;
  gap: 6px;
  align-items: center;
  color: #3f8a52;
  margin-bottom: 12px;
}
.rules-admin .primary {
  margin-top: 14px;
}
@media (max-width: 720px) {
  .rules-admin form > header {
    flex-direction: column;
  }
  .rules-head {
    display: none;
  }
  .rules-row {
    grid-template-columns: 36px 1fr 1fr;
    padding: 10px;
    border: 0;
    border-radius: 12px;
  }
}
.rules-admin { padding: 18px 20px; }
.rules-admin form > header button { white-space: nowrap; flex-shrink: 0; display: inline-flex; gap: 6px; align-items: center; }
.rules-check input[type="checkbox"] { width: auto; margin: 0; padding: 0; accent-color: #f65331; }
</style>
