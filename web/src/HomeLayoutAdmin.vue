<script setup>
import { ref, computed, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const layout = ref(null),
  error = ref(""),
  notice = ref(""),
  saving = ref(false);
const controller = new AbortController();
const targetLabels = {
  checkin: "每日签到",
  invite: "邀请好友",
  activity: "转盘活动",
  withdraw: "提现",
  shop: "捡漏",
  ledger: "金币明细",
  me: "我的",
};
const iconLabels = {
  gift: "礼物",
  coin: "金币",
  clock: "时钟",
  shop: "购物袋",
  me: "用户",
  hall: "消息",
  notice: "公告",
  shield: "盾牌",
  check: "对勾",
};
const animations = { float: "上下漂浮", bounce: "弹跳", shake: "摇摆", none: "静止" };
const hosts = computed(() => (layout.value?.allowed_hosts || []).join("、"));
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try {
    layout.value = await props.api("/admin/home-layout", { signal: controller.signal });
  } catch (e) {
    fail(e);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    layout.value = await props.api("/admin/home-layout", {
      method: "PUT",
      body: { entries: layout.value.entries, floating: layout.value.floating },
      signal: controller.signal,
    });
    notice.value = "已保存，App 在 1 分钟内自动刷新";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
const uploading = ref("");
async function upload(event, item) {
  const file = event.target.files?.[0];
  event.target.value = "";
  if (!file) return;
  if (file.size > 5 * 1024 * 1024) {
    error.value = "图片不能超过 5MB";
    return;
  }
  uploading.value = item === layout.value.floating ? "floating" : item.id;
  error.value = "";
  try {
    const base64 = await new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).split(",").pop());
      reader.onerror = reject;
      reader.readAsDataURL(file);
    });
    const stored = await props.api("/admin/products/images", {
      method: "POST",
      body: { image_base64: base64 },
      signal: controller.signal,
    });
    item.image_url = stored.image;
  } catch (e) {
    fail(e);
  } finally {
    uploading.value = "";
  }
}
function addEntry() {
  layout.value.entries.push({
    id: `e${Date.now().toString(36)}`,
    title: "",
    badge: "",
    icon: "gift",
    image_url: "",
    action: "web",
    target: "",
    enabled: false,
    starts_at: "",
    expires_at: "",
  });
}
function move(index, step) {
  const list = layout.value.entries;
  const next = index + step;
  if (next < 0 || next >= list.length) return;
  [list[index], list[next]] = [list[next], list[index]];
}
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <section class="admin-card home-admin">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="home-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <p v-if="!layout && !error" class="home-muted">正在读取首页入口…</p>
    <form v-if="layout" @submit.prevent="save">
      <header>
        <div>
          <p>
            大厅图标入口和右下角悬浮入口默认不显示，只用于运营活动；签到、捡漏、提现等已有入口无需重复添加。名称、角标、图标都按活动自行填写或上传。网页活动填 <code>/activity/xxx.html</code>
            这类站内路径，或允许域名（{{ hosts }}）下的 https 地址。时间留空表示不限，格式如
            <code>2026-12-01T00:00:00+08:00</code>。
          </p>
        </div>
        <button type="button" class="outline" @click="addEntry"><Icon name="grid" :size="14" />添加入口</button>
      </header>

      <div class="home-list">
        <article v-for="(entry, index) in layout.entries" :key="entry.id" class="home-item" :class="{ off: !entry.enabled }">
          <div class="home-item-top">
            <label class="home-switch"><input v-model="entry.enabled" type="checkbox" />{{ entry.enabled ? "显示中" : "已隐藏" }}</label>
            <span class="home-order">
              <button type="button" aria-label="上移" @click="move(index, -1)">↑</button>
              <button type="button" aria-label="下移" @click="move(index, 1)">↓</button>
              <button type="button" class="home-remove" @click="layout.entries.splice(index, 1)">删除</button>
            </span>
          </div>
          <div class="home-grid">
            <label>名称<input v-model="entry.title" maxlength="6" required placeholder="按活动填写" /></label>
            <label>角标<input v-model="entry.badge" maxlength="4" placeholder="如：新" /></label>
            <label
              >图标<select v-model="entry.icon">
                <option v-for="name in layout.icons" :key="name" :value="name">{{ iconLabels[name] || name }}</option>
              </select></label
            >
            <div class="wide home-image">
              <span>图标图片（选填，优先于内置图标，建议 120×120）</span>
              <div>
                <img v-if="entry.image_url" :src="entry.image_url" alt="" />
                <label class="outline home-upload">{{ uploading === entry.id ? "上传中…" : entry.image_url ? "更换" : "上传图片" }}<input type="file" accept="image/png,image/jpeg,image/webp" hidden @change="upload($event, entry)" /></label>
                <button v-if="entry.image_url" type="button" class="home-clear" @click="entry.image_url = ''">移除</button>
              </div>
            </div>
            <label
              >打开方式<select v-model="entry.action">
                <option value="native">App 内置页面</option>
                <option value="web">网页活动</option>
              </select></label
            >
            <label v-if="entry.action === 'native'"
              >页面<select v-model="entry.target" required>
                <option v-for="name in layout.native_targets" :key="name" :value="name">{{ targetLabels[name] || name }}</option>
              </select></label
            >
            <label v-else class="wide">活动地址<input v-model="entry.target" required placeholder="/activity/demo.html" /></label>
            <label>开始时间<input v-model="entry.starts_at" placeholder="不限" /></label>
            <label>结束时间<input v-model="entry.expires_at" placeholder="不限" /></label>
          </div>
        </article>
      </div>

      <section class="home-floating">
        <div class="home-item-top">
          <h3>悬浮入口</h3>
          <label class="home-switch"><input v-model="layout.floating.enabled" type="checkbox" />{{ layout.floating.enabled ? "显示中" : "已隐藏" }}</label>
        </div>
        <div class="home-grid">
          <label>名称（选填，留空不显示文字）<input v-model="layout.floating.title" maxlength="8" placeholder="按活动填写" /></label>
          <label
            >动画<select v-model="layout.floating.animation">
              <option v-for="(label, key) in animations" :key="key" :value="key">{{ label }}</option>
            </select></label
          >
          <label
            >图标<select v-model="layout.floating.icon">
              <option v-for="name in layout.icons" :key="name" :value="name">{{ iconLabels[name] || name }}</option>
            </select></label
          >
          <div class="wide home-image">
            <span>悬浮图片（选填，建议 200×200 透明 PNG）</span>
            <div>
              <img v-if="layout.floating.image_url" :src="layout.floating.image_url" alt="" />
              <label class="outline home-upload">{{ uploading === "floating" ? "上传中…" : layout.floating.image_url ? "更换" : "上传图片" }}<input type="file" accept="image/png,image/jpeg,image/webp" hidden @change="upload($event, layout.floating)" /></label>
              <button v-if="layout.floating.image_url" type="button" class="home-clear" @click="layout.floating.image_url = ''">移除</button>
            </div>
          </div>
          <label
            >打开方式<select v-model="layout.floating.action">
              <option value="native">App 内置页面</option>
              <option value="web">网页活动</option>
            </select></label
          >
          <label v-if="layout.floating.action === 'native'"
            >页面<select v-model="layout.floating.target">
              <option v-for="name in layout.native_targets" :key="name" :value="name">{{ targetLabels[name] || name }}</option>
            </select></label
          >
          <label v-else class="wide">活动地址<input v-model="layout.floating.target" placeholder="/activity/demo.html" /></label>
          <label>开始时间<input v-model="layout.floating.starts_at" placeholder="不限" /></label>
          <label>结束时间<input v-model="layout.floating.expires_at" placeholder="不限" /></label>
        </div>
      </section>
      <button class="primary" :disabled="saving"><Icon name="check" :size="17" />保存并发布</button>
    </form>
  </section>
</template>

<style scoped>
.home-admin form > header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  margin-bottom: 16px;
}
.home-admin h2 {
  margin: 0 0 6px;
}
.home-admin h3 {
  margin: 0;
  font-size: 16px;
}
.home-admin header p,
.home-muted {
  color: #9a8274;
  font-size: 14px;
  line-height: 1.7;
  margin: 0;
}
.home-admin code {
  background: #fff2e6;
  padding: 1px 5px;
  border-radius: 5px;
}
.home-list {
  display: grid;
  gap: 12px;
}
.home-item,
.home-floating {
  border: 0;
  border-radius: 14px;
  padding: 12px 14px;
  background: #fffdfb;
}
.home-floating {
  margin-top: 16px;
  background: #fff7ee;
}
.home-item.off {
  opacity: 0.6;
}
.home-item-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}
.home-switch {
  display: flex;
  gap: 6px;
  align-items: center;
  font-size: 14px;
}
.home-order {
  display: flex;
  gap: 6px;
}
.home-order button {
  border: 1px solid #ecdccd;
  background: #fff;
  border-radius: 8px;
  padding: 2px 9px;
  cursor: pointer;
}
.home-order .home-remove {
  color: #d0533a;
}
.home-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}
.home-grid label {
  display: grid;
  gap: 4px;
  font-size: 13.5px;
  color: #8a7266;
  min-width: 0;
}
.home-grid .wide {
  grid-column: span 2;
}
.home-grid input,
.home-grid select {
  min-width: 0;
}
.home-image {
  display: grid;
  gap: 4px;
  font-size: 13.5px;
  color: #8a7266;
}
.home-image > div {
  display: flex;
  gap: 8px;
  align-items: center;
}
.home-image img {
  width: 40px;
  height: 40px;
  object-fit: cover;
  border-radius: 10px;
  background: #fff2e6;
}
.home-upload {
  cursor: pointer;
  padding: 6px 12px;
  font-size: 13.5px;
}
.home-clear {
  border: 0;
  background: none;
  color: #d0533a;
  cursor: pointer;
}
.home-notice {
  display: flex;
  gap: 6px;
  align-items: center;
  color: #3f8a52;
  margin-bottom: 12px;
}
.home-admin .primary {
  margin-top: 16px;
}
@media (max-width: 720px) {
  .home-admin form > header {
    flex-direction: column;
  }
  .home-grid {
    grid-template-columns: 1fr 1fr;
  }
}
/* Card padding and compact controls (global form styles stretch inputs to full width). */
.home-admin { padding: 18px 20px; }
.home-admin form > header button { white-space: nowrap; flex-shrink: 0; display: inline-flex; gap: 6px; align-items: center; }
.home-switch { white-space: nowrap; margin: 0; }
.home-switch input[type="checkbox"] { width: auto; margin: 0; padding: 0; accent-color: #f65331; }
.home-item, .home-floating { padding: 16px 18px; }
</style>
