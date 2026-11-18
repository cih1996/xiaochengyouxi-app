<script setup>
import { ref, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const cfg = ref(null), error = ref(""), notice = ref(""), saving = ref(false);
const controller = new AbortController();
onUnmounted(() => controller.abort());
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try { cfg.value = await props.api("/admin/reminders", { signal: controller.signal }); } catch (e) { fail(e); }
}
async function save() {
  saving.value = true; error.value = ""; notice.value = "";
  try {
    cfg.value = await props.api("/admin/reminders", { method: "PUT", body: cfg.value, signal: controller.signal });
    notice.value = "已保存，用户下次打开 App 后按新配置提醒";
  } catch (e) { fail(e); } finally { saving.value = false; }
}
const GROUPS = [
  { key: "rain", title: "红包雨开场提醒", hint: "用户离开 App 后，提醒最近的下一场红包雨；每次打开 App 最多安排一次，不会每场都提醒。", time: null },
  { key: "checkin", title: "签到提醒", hint: "当天还没签到时，在设定时间提醒一次。", time: "checkin_time" },
  { key: "wheel", title: "幸运转盘提醒", hint: "幸运转盘开启且当天免费次数未用时，在设定时间提醒一次。", time: "wheel_time" },
];
defineExpose({ refresh });
refresh();
</script>
<template>
  <form v-if="cfg" class="admin-card rm-card" @submit.prevent="save">
    <header class="rm-head">
      <div>
        <h2>通知提醒</h2>
        <p>由 App 在手机通知栏本地提醒，不接第三方推送服务。用户需在 App 内授权通知权限，可在“设置 → 通知提醒”中关闭。免打扰时段内不会提醒。</p>
      </div>
      <label class="rm-switch"><input v-model="cfg.enabled" type="checkbox" aria-label="开启通知提醒" /><span></span></label>
    </header>
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="rm-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <div class="rm-quiet">
      <label>免打扰开始<input v-model="cfg.quiet_start" placeholder="22:30" maxlength="5" /></label>
      <label>免打扰结束<input v-model="cfg.quiet_end" placeholder="08:00" maxlength="5" /></label>
    </div>
    <section v-for="g in GROUPS" :key="g.key" class="rm-group">
      <div class="rm-group-head">
        <div><h3>{{ g.title }}</h3><p>{{ g.hint }}</p></div>
        <label class="rm-switch"><input v-model="cfg[g.key + '_enabled']" type="checkbox" :aria-label="g.title" /><span></span></label>
      </div>
      <div class="rm-fields">
        <label v-if="g.time">提醒时间<input v-model="cfg[g.time]" placeholder="20:00" maxlength="5" /></label>
        <label v-else>提前几分钟<input v-model.number="cfg.rain_lead_minutes" type="number" min="1" max="30" /></label>
        <label>通知标题（≤20 字）<input v-model="cfg[g.key + '_title']" maxlength="20" /></label>
        <label class="wide">通知内容（≤40 字）<input v-model="cfg[g.key + '_body']" maxlength="40" /><small v-if="g.key === 'rain'">{minutes} 会替换为提前的分钟数</small></label>
      </div>
    </section>
    <div class="rm-actions"><button class="primary" :disabled="saving"><Icon name="check" :size="16" />{{ saving ? "保存中…" : "保存" }}</button></div>
  </form>
  <p v-else-if="error" class="form-error">{{ error }}</p>
</template>
<style scoped>
.rm-card { padding: 18px 20px; }
.rm-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.rm-card h3 { font-size: 15px; color: #4f3627; margin: 0; }
.rm-head, .rm-group-head { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; }
.rm-head p, .rm-group-head p { color: #9a8274; font-size: 13px; line-height: 1.7; margin: 4px 0 0; max-width: 820px; }
.rm-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 10px 14px; margin-top: 12px; font-size: 13.5px; }
.rm-quiet { display: flex; gap: 14px; margin-top: 14px; }
.rm-quiet label, .rm-fields label { margin: 0; display: grid; gap: 6px; font-size: 13px; }
.rm-quiet input { width: 120px; }
.rm-card input:not([type="checkbox"]) { margin: 0; padding: 9px 11px; font-size: 13.5px; border-radius: 10px; }
.rm-group { border-top: 1px solid #f2e8d9; margin-top: 16px; padding-top: 14px; }
.rm-fields { display: grid; grid-template-columns: 160px 1fr 2fr; gap: 12px 14px; margin-top: 10px; }
.rm-fields small { color: #a8907f; font-size: 12px; }
.rm-switch { position: relative; display: block; width: 42px; height: 24px; flex-shrink: 0; margin: 0; }
.rm-switch input { opacity: 0; width: 100%; height: 100%; position: absolute; inset: 0; margin: 0; padding: 0; cursor: pointer; }
.rm-switch > span { pointer-events: none; display: block; height: 24px; border-radius: 30px; background: #dfd4c3; transition: background 0.2s; }
.rm-switch > span::after { content: ""; position: absolute; height: 18px; width: 18px; top: 3px; left: 3px; background: #fff; border-radius: 50%; transition: transform 0.2s; }
.rm-switch input:checked + span { background: #ec985b; }
.rm-switch input:checked + span::after { transform: translateX(18px); }
.rm-actions { margin-top: 16px; }
.rm-actions .primary { width: auto; padding: 10px 22px; margin: 0; display: inline-flex; gap: 6px; align-items: center; }
@media (max-width: 900px) { .rm-fields { grid-template-columns: 1fr; } }
</style>
