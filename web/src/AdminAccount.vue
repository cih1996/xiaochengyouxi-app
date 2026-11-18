<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["close", "changed", "unauthorized"]);
const account = ref(null),
  username = ref(""),
  currentPassword = ref(""),
  newPassword = ref(""),
  confirmation = ref(""),
  error = ref(""),
  saving = ref(false);
const visible = ref({ current: false, next: false, confirmation: false });
const controller = new AbortController();
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function load() {
  try {
    account.value = await props.api("/admin/account", {
      signal: controller.signal,
    });
    username.value = account.value.username;
  } catch (failure) {
    fail(failure);
  }
}
async function save() {
  error.value = "";
  if (newPassword.value !== confirmation.value) {
    error.value = "两次输入的新密码不一致";
    return;
  }
  saving.value = true;
  try {
    const result = await props.api("/admin/account", {
      method: "PUT",
      signal: controller.signal,
      body: {
        username: username.value.trim(),
        current_password: currentPassword.value,
        ...(newPassword.value ? { new_password: newPassword.value } : {}),
      },
    });
    emit("changed", result);
  } catch (failure) {
    fail(failure);
  } finally {
    saving.value = false;
  }
}
function close() {
  if (!saving.value) emit("close");
}
onMounted(load);
onUnmounted(() => controller.abort());
</script>

<template>
  <div class="admin-account-overlay" @click.self="close">
    <section
      class="admin-account-panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="admin-account-heading"
    >
      <header>
        <div>
          <h2 id="admin-account-heading">管理账号</h2>
        </div>
        <button
          type="button"
          aria-label="关闭账户设置"
          :disabled="saving"
          @click="close"
        >
          <Icon name="close" :size="21" />
        </button>
      </header>
      <div v-if="error" class="form-error" role="alert">{{ error }}</div>
      <p v-if="!account && !error" class="admin-account-loading">
        正在读取账户…
      </p>
      <form v-if="account?.auth_required" @submit.prevent="save">
        <label
          >账号<input
            v-model="username"
            autocomplete="username"
            minlength="2"
            maxlength="40"
            required
        /></label>
        <label
          >当前密码
          <div class="admin-password-field">
            <input
              v-model="currentPassword"
              :type="visible.current ? 'text' : 'password'"
              autocomplete="current-password"
              required
            /><button
              type="button"
              :aria-label="visible.current ? '隐藏当前密码' : '显示当前密码'"
              :aria-pressed="visible.current"
              @click="visible.current = !visible.current"
            >
              <Icon :name="visible.current ? 'eye-off' : 'eye'" :size="18" />
            </button></div
        ></label>
        <div class="admin-account-divider"></div>
        <label
          >新密码<span class="admin-field-note" aria-hidden="true">选填</span>
          <div class="admin-password-field">
            <input
              v-model="newPassword"
              :type="visible.next ? 'text' : 'password'"
              aria-label="新密码"
              autocomplete="new-password"
              minlength="8"
              placeholder="留空仅修改账号"
            /><button
              type="button"
              :aria-label="visible.next ? '隐藏新密码' : '显示新密码'"
              :aria-pressed="visible.next"
              @click="visible.next = !visible.next"
            >
              <Icon :name="visible.next ? 'eye-off' : 'eye'" :size="18" />
            </button></div
        ></label>
        <label
          >确认新密码
          <div class="admin-password-field">
            <input
              v-model="confirmation"
              :type="visible.confirmation ? 'text' : 'password'"
              autocomplete="new-password"
            /><button
              type="button"
              :aria-label="
                visible.confirmation ? '隐藏确认新密码' : '显示确认新密码'
              "
              :aria-pressed="visible.confirmation"
              @click="visible.confirmation = !visible.confirmation"
            >
              <Icon
                :name="visible.confirmation ? 'eye-off' : 'eye'"
                :size="18"
              />
            </button></div
        ></label>
        <button class="primary full" :disabled="saving">
          <Icon name="check" :size="17" />{{
            saving ? "保存中…" : "保存并重新登录"
          }}
        </button>
      </form>
    </section>
  </div>
</template>

<style scoped>
.admin-account-overlay {
  position: fixed;
  inset: 0;
  z-index: 80;
  background: #59433458;
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
.admin-account-panel {
  width: 420px;
  max-height: calc(100dvh - 48px);
  overflow-y: auto;
  border: 0;
  border-radius: 23px;
  background: #fffcf6;
  padding: 27px;
  box-shadow: 0 25px 70px #72503229;
}
.admin-account-panel > header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.admin-account-panel h2 {
  font-size: 20px;
  color: #7f5b3c;
}
.admin-account-panel header button {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  color: #b39978;
  background: #f8eedf;
}
.admin-account-panel label {
  font-size: 13px;
  color: #a58a68;
  margin-bottom: 18px;
}
.admin-account-panel input {
  font-size: 13.5px;
  margin-top: 8px;
  border-color: #e8dbc8;
  background: #fffefa;
  border-radius: 10px;
}
.admin-account-panel .admin-password-field {
  margin-top: 8px;
}
.admin-account-panel .admin-password-field input {
  margin-top: 0;
}
.admin-account-panel .primary {
  font-size: 13.5px;
  margin-top: 8px;
  border-radius: 11px;
}
.admin-account-divider {
  height: 1px;
  background: #efe3d0;
  margin: 23px 0;
}
.admin-field-note {
  float: right;
  font-weight: 400;
  font-size: 12.5px;
  color: #c0a584;
}
.admin-account-loading {
  color: #b09675;
  padding: 25px 0;
  font-size: 13.5px;
}
@media (max-width: 500px) {
  .admin-account-overlay {
    padding: 16px;
  }
  .admin-account-panel {
    padding: 23px;
    max-height: calc(100dvh - 32px);
  }
}
</style>
