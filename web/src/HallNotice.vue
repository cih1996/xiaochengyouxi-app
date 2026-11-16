<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const runtime = ref(null),
  error = ref("");
let controller, timer;
async function refresh() {
  if (document.visibilityState !== "visible") return;
  controller?.abort();
  controller = new AbortController();
  error.value = "";
  try {
    runtime.value = await props.api("/app/runtime", {
      signal: controller.signal,
    });
  } catch (failure) {
    if (failure.name !== "AbortError") error.value = failure.message;
  }
}
onMounted(() => {
  refresh();
  timer = setInterval(refresh, 45000);
  window.addEventListener("focus", refresh);
  document.addEventListener("visibilitychange", refresh);
});
onUnmounted(() => {
  clearInterval(timer);
  controller?.abort();
  window.removeEventListener("focus", refresh);
  document.removeEventListener("visibilitychange", refresh);
});
</script>

<template>
  <div v-if="error" class="hall-notice-error" role="alert">{{ error }}</div>
  <aside v-if="runtime?.hall_notice" class="hall-notice" aria-label="大厅公告">
    <span><Icon name="bell" :size="17" /></span>
    <div>
      <b>大厅公告</b>
      <p>{{ runtime.hall_notice }}</p>
    </div>
  </aside>
</template>

<style scoped>
.hall-notice {
  display: flex;
  gap: 9px;
  align-items: flex-start;
  padding: 11px 12px;
  background: #fff1dc;
  border: 1px solid #f6deba;
  border-radius: 13px;
  margin-bottom: 12px;
}
.hall-notice > span {
  display: grid;
  place-items: center;
  height: 25px;
  width: 25px;
  flex-shrink: 0;
  color: #c39451;
  background: #ffe7bd;
  border-radius: 8px;
}
.hall-notice > div {
  min-width: 0;
}
.hall-notice b {
  font-size: 10px;
  font-weight: 600;
  color: #ac7c44;
}
.hall-notice p {
  font-size: 10px;
  color: #b58b58;
  line-height: 1.8;
  margin-top: 3px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.hall-notice-error {
  font-size: 11px;
  color: #b16f4e;
  line-height: 1.8;
  padding: 10px 0;
}
</style>
