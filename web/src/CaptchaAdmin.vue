<script setup>
import { ref, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const cfg = ref(null),
  secret = ref(""),
  error = ref(""),
  notice = ref(""),
  saving = ref(false);
const controller = new AbortController();
onUnmounted(() => controller.abort());
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try {
    cfg.value = await props.api("/admin/captcha-config", { signal: controller.signal });
  } catch (e) {
    fail(e);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    cfg.value = await props.api("/admin/captcha-config", {
      method: "PUT",
      body: { enabled: cfg.value.enabled, app_id: cfg.value.app_id, app_secret_key: secret.value },
      signal: controller.signal,
    });
    secret.value = "";
    notice.value = cfg.value.ready ? "已保存，邀请落地页将要求安全验证后才能发送验证码" : "已保存";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
defineExpose({ refresh });
refresh();
</script>
<template>
  <form v-if="cfg" class="admin-card cap-card" @submit.prevent="save">
    <header class="cap-head">
      <div>
        <h2>人机验证（腾讯云验证码）</h2>
        <p>
          邀请落地页发送短信验证码前，必须先通过滑块/点选验证；票据只在服务端校验且只能用一次。未开启时，落地页不发短信，只引导下载 App 并在注册时填写邀请码。
          在腾讯云“验证码”控制台创建验证，复制 CaptchaAppId 与 AppSecretKey；校验接口使用上方短信通道的 SecretId/SecretKey。
        </p>
      </div>
      <span class="cap-state" :class="{ on: cfg.ready }">{{ cfg.ready ? "已启用" : "未启用" }}</span>
    </header>
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="cap-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <p v-if="!cfg.cloud_keys_set" class="cap-warn">请先在上方短信通道填写 SecretId 与 SecretKey（校验验证码需要）。</p>
    <div class="cap-fields">
      <label class="cap-toggle"><input v-model="cfg.enabled" type="checkbox" /> 开启人机验证</label>
      <label>CaptchaAppId<input v-model.trim="cfg.app_id" inputmode="numeric" placeholder="例如 190000000" /></label>
      <label
        >AppSecretKey<input
          v-model.trim="secret"
          type="password"
          autocomplete="new-password"
          :placeholder="cfg.app_secret_key_set ? '已保存，留空不修改' : '粘贴 AppSecretKey'"
      /></label>
    </div>
    <div class="cap-actions">
      <button class="primary" :disabled="saving"><Icon name="check" :size="16" />{{ saving ? "保存中…" : "保存" }}</button>
    </div>
  </form>
</template>
<style scoped>
.cap-card { padding: 18px 20px; margin-top: 12px; }
.cap-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.cap-head { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; }
.cap-head p { color: #9a8274; font-size: 13.5px; line-height: 1.7; margin: 0; max-width: 860px; }
.cap-state { font-size: 13px; border-radius: 999px; padding: 5px 12px; background: #f7efe8; color: #8a6f61; white-space: nowrap; }
.cap-state.on { background: #eefaf1; color: #2f8a4c; }
.cap-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 10px 14px; margin: 12px 0 0; font-size: 13.5px; }
.cap-warn { color: #b03a2a; background: #fff3f0; border-radius: 10px; padding: 9px 12px; font-size: 13px; margin: 12px 0 0; }
.cap-fields { display: grid; grid-template-columns: 180px 1fr 1fr; gap: 12px 14px; margin-top: 14px; align-items: end; }
.cap-fields label { margin: 0; display: grid; gap: 6px; font-size: 13px; }
.cap-fields input:not([type="checkbox"]) { margin: 0; padding: 9px 11px; font-size: 13.5px; border-radius: 10px; }
.cap-toggle { display: flex !important; align-items: center; gap: 8px; padding-bottom: 10px; font-size: 14px !important; color: #6f4b33; }
.cap-toggle input { width: 16px; height: 16px; margin: 0; }
.cap-actions { margin-top: 14px; }
.cap-actions .primary { width: auto; padding: 10px 22px; margin: 0; display: inline-flex; gap: 6px; align-items: center; }
@media (max-width: 900px) { .cap-fields { grid-template-columns: 1fr; } }
</style>
