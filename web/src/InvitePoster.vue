<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import { createInvitePoster } from "./invite-poster";
const props = defineProps({ api: Function });
const summary = ref(null),
  poster = ref(""),
  error = ref(""),
  notice = ref("");
let objectUrl;
async function load() {
  error.value = "";
  try {
    summary.value = await props.api("/invite/summary");
    const qr = await props.api("/invite/qr/" + summary.value.invite_code);
    const blob = await createInvitePoster(summary.value, qr);
    objectUrl = URL.createObjectURL(blob);
    poster.value = objectUrl;
  } catch (failure) {
    error.value = failure.message;
  }
}
async function copy(value, label) {
  try {
    await navigator.clipboard.writeText(value);
    notice.value = label + "已复制";
  } catch (failure) {
    error.value = failure.message;
  }
}
onMounted(load);
onUnmounted(() => {
  if (objectUrl) URL.revokeObjectURL(objectUrl);
});
</script>
<template>
  <div class="invite-poster-content">
    <template v-if="summary"
      ><div class="invite-summary">
        <div>
          <span>我的邀请码</span><b>{{ summary.invite_code }}</b>
        </div>
        <button @click="copy(summary.invite_code, '邀请码')">复制</button>
      </div>
      <div class="invite-counts">
        <div>
          <b>{{ summary.invited_count }}</b
          ><span>已邀请</span>
        </div>
        <div>
          <b>{{ summary.registered_count }}</b
          ><span>已注册</span>
        </div>
        <div>
          <b>{{ summary.pending_count }}</b
          ><span>待加入</span>
        </div>
      </div>
      <img
        v-if="poster"
        class="invite-poster-preview"
        :src="poster"
        alt="我的专属邀请海报"
      />
      <p v-else-if="!error" class="invite-poster-loading">
        正在准备你的专属海报…
      </p>
      <div class="invite-poster-actions">
        <a
          v-if="poster"
          class="primary"
          :href="poster"
          :download="'小橙有喜-邀请' + summary.invite_code + '.png'"
          ><Icon name="gift" :size="16" />保存海报</a
        ><button class="outline" @click="copy(summary.invite_url, '邀请链接')">
          复制链接
        </button>
      </div>
      <p v-if="notice" class="invite-copy-notice">{{ notice }}</p></template
    >
    <div v-if="error" class="form-error">{{ error }}</div>
  </div>
</template>
