<script setup>
import { ref } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["bound", "refuse"]);
const inviteCode = ref(""),
  busy = ref(false),
  error = ref("");
async function bind() {
  busy.value = true;
  error.value = "";
  try {
    const result = await props.api("/invite/bind", {
      method: "POST",
      body: { invite_code: inviteCode.value.trim().toUpperCase() },
    });
    emit("bound", result.user);
  } catch (failure) {
    error.value = failure.message;
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <div class="invite-bind-page">
    <section class="invite-bind-card">
      <button
        class="invite-back"
        :disabled="busy"
        @click="emit('refuse')"
        aria-label="返回登录"
      >
        <Icon name="back" :size="20" />
      </button>
      <a class="brand" href="/" @click.prevent="!busy && emit('refuse')"
        ><span class="brand-symbol">橙</span><b>小橙有喜</b></a
      >
      <img class="invite-bind-art" src="/assets/mascot.png" alt="小橙助手" />
      <h1>绑定邀请码</h1>
      <form @submit.prevent="bind">
        <label
          >邀请码<input
            :value="inviteCode"
            @input="inviteCode = $event.target.value.toUpperCase()"
            class="invite-code-input"
            placeholder="输入朋友的邀请码"
            minlength="5"
            maxlength="8"
            pattern="[A-Z0-9]{5,8}"
            autocomplete="off"
            autocapitalize="characters"
            required
        /></label>
        <div v-if="error" class="form-error">{{ error }}</div>
        <button class="primary full" :disabled="busy">
          {{ busy ? "正在绑定…" : "绑定并进入"
          }}<Icon name="arrow" :size="17" />
        </button>
      </form>
      <button class="invite-refuse" :disabled="busy" @click="emit('refuse')">
        暂不绑定，返回登录
      </button>
    </section>
  </div>
</template>
