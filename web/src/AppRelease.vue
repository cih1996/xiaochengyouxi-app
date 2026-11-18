<script setup>
import { ref, computed, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const release = ref(null),
  runtime = ref(null),
  error = ref(""),
  notice = ref(""),
  savingRelease = ref(false),
  savingNotice = ref(false),
  fileInput = ref(null);
const controller = new AbortController();
const fields = [
  "enabled",
  "latest_version_code",
  "latest_version_name",
  "min_supported_version_code",
  "title",
  "release_notes",
  "apk_url",
  "apk_sha256",
  "apk_size_bytes",
];
const packageSize = computed(() =>
  release.value?.apk_size_bytes > 0
    ? (release.value.apk_size_bytes / 1048576).toFixed(1) + " MB"
    : "",
);
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function refresh() {
  release.value = null;
  runtime.value = null;
  error.value = "";
  notice.value = "";
  try {
    [release.value, runtime.value] = await Promise.all([
      props.api("/admin/app-release", { signal: controller.signal }),
      props.api("/admin/app-runtime", { signal: controller.signal }),
    ]);
  } catch (failure) {
    fail(failure);
  }
}
async function saveRelease() {
  savingRelease.value = true;
  error.value = "";
  notice.value = "";
  try {
    release.value = await props.api("/admin/app-release", {
      method: "PUT",
      signal: controller.signal,
      body: Object.fromEntries(fields.map((key) => [key, release.value[key]])),
    });
    notice.value = "发布设置已保存";
  } catch (failure) {
    fail(failure);
  } finally {
    savingRelease.value = false;
  }
}
async function saveNotice() {
  savingNotice.value = true;
  error.value = "";
  notice.value = "";
  try {
    runtime.value = await props.api("/admin/app-runtime", {
      method: "PUT",
      signal: controller.signal,
      body: { hall_notice: runtime.value.hall_notice },
    });
    notice.value = "大厅公告已保存";
  } catch (failure) {
    fail(failure);
  } finally {
    savingNotice.value = false;
  }
}
async function importRelease(event) {
  const file = event.target.files[0];
  if (!file) return;
  error.value = "";
  notice.value = "";
  try {
    const data = JSON.parse(await file.text());
    const imported = fields.filter((key) => Object.hasOwn(data, key));
    if (!imported.length) throw new Error("文件中没有发布信息");
    for (const key of imported) release.value[key] = data[key];
    notice.value = "已导入，保存后生效";
  } catch (failure) {
    fail(failure);
  } finally {
    event.target.value = "";
  }
}
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <div class="app-release-page">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="release-notice" role="status">
      <Icon name="check" :size="16" />{{ notice }}
    </div>
    <div v-if="!release && !error" class="release-loading">
      正在读取发布设置…
    </div>
    <template v-if="release && runtime">
      <div class="release-columns">
        <section class="admin-card release-form">
          <header class="release-card-heading">
            <div>
              <h2>App 更新</h2>
            </div>
            <button
              class="outline release-import"
              type="button"
              @click="fileInput.click()"
            >
              <Icon name="list" :size="16" />导入发布信息</button
            ><input
              ref="fileInput"
              class="release-file-input"
              type="file"
              accept="application/json,.json"
              aria-label="选择发布信息文件"
              @change="importRelease"
            />
          </header>
          <form @submit.prevent="saveRelease">
            <div class="release-switch-row">
              <div>
                <b>开启版本检测</b>
                <p>用户打开 App 时检查新版本</p>
              </div>
              <label class="release-switch"
                ><input
                  v-model="release.enabled"
                  type="checkbox"
                  aria-label="开启版本检测" /><span></span
              ></label>
            </div>
            <div class="release-two-fields">
              <label
                >最新版本名称<input
                  v-model="release.latest_version_name"
                  placeholder="例如 0.3.9"
                  :required="release.enabled" /></label
              ><label
                >最新版本代码<input
                  v-model.number="release.latest_version_code"
                  type="number"
                  min="0"
                  step="1"
                  :required="release.enabled"
              /></label>
            </div>
            <label
              >更新标题<input v-model="release.title" placeholder="发现新版本"
            /></label>
            <label
              >更新内容<textarea
                v-model="release.release_notes"
                rows="4"
                placeholder="写下这次带来的新体验"
              ></textarea>
            </label>
            <label class="release-minimum"
              >最低支持版本（代码）<input
                v-model.number="release.min_supported_version_code"
                aria-label="最低支持版本（代码）"
                type="number"
                min="0"
                step="1"
                required
              /><small>低于此代码时必须更新，0 为可选更新。</small></label
            >
            <details class="release-package">
              <summary>
                <span><Icon name="download" :size="17" />安装包信息</span
                ><Icon name="chevron" :size="16" />
              </summary>
              <div>
                <label
                  >下载地址<input
                    v-model="release.apk_url"
                    inputmode="url"
                    placeholder="https://…" /></label
                ><label
                  >SHA-256<input
                    v-model="release.apk_sha256"
                    class="release-mono"
                    spellcheck="false" /></label
                ><label
                  >安装包大小（字节）<input
                    v-model.number="release.apk_size_bytes"
                    type="number"
                    min="0"
                    step="1"
                /></label>
              </div>
            </details>
            <footer class="release-form-footer">
              <span>配置 #{{ release.revision }}</span
              ><button class="primary" :disabled="savingRelease">
                <Icon name="check" :size="16" />{{
                  savingRelease ? "保存中…" : "保存发布设置"
                }}
              </button>
            </footer>
          </form>
        </section>
        <aside class="release-preview">
          <div class="release-preview-heading">
            <Icon name="eye" :size="16" /><span>更新弹窗预览</span>
          </div>
          <div class="release-preview-card">
            <div class="release-preview-glow"></div>
            <img src="/assets/mascot.png" alt="" /><span
              class="release-preview-brand"
              >小橙有喜</span
            >
            <h3>{{ release.title || "更新标题" }}</h3>
            <p class="release-preview-version">
              {{ release.latest_version_name || "版本名称"
              }}<span v-if="packageSize"> · {{ packageSize }}</span>
            </p>
            <p class="release-preview-notes">
              {{ release.release_notes || "更新内容将在这里展示" }}
            </p>
            <div class="release-preview-action">
              立即更新 <Icon name="download" :size="16" />
            </div>
            <span class="release-preview-later">{{
              release.min_supported_version_code > 0
                ? "低版本须更新"
                : "可选择稍后更新"
            }}</span>
          </div>
        </aside>
      </div>
      <section class="admin-card release-announcement">
        <header class="release-card-heading">
          <div>
            <h2>大厅公告</h2>
          </div>
          <span class="release-announcement-icon"
            ><Icon name="bell" :size="23"
          /></span>
        </header>
        <form @submit.prevent="saveNotice">
          <label class="release-announcement-input">
            <textarea
              v-model="runtime.hall_notice"
              rows="3"
              maxlength="200"
              aria-label="大厅公告"
              placeholder="写一条简短的大厅公告"
            ></textarea
            ><span>{{ runtime.hall_notice.length }}/200</span></label
          >
          <footer class="release-form-footer">
            <span>{{
              runtime.updated_at
                ? "更新于 " +
                  new Date(runtime.updated_at).toLocaleString("zh-CN", {
                    hour12: false,
                  })
                : "尚未发布公告"
            }}</span><button class="primary" :disabled="savingNotice">
              <Icon name="check" :size="16" />{{
                savingNotice ? "保存中…" : "保存公告"
              }}
            </button>
          </footer>
        </form>
      </section>
    </template>
  </div>
</template>

<style scoped>
.app-release-page {
  max-width: 1160px;
}
.release-columns {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 305px;
  gap: 25px;
  align-items: start;
}
.release-form,
.release-announcement {
  padding: 18px 20px;
}
.release-card-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 15px;
  margin-bottom: 18px;
}
.release-card-heading h2 {
  font-size: 18px;
  color: #7f5a3a;
}
.release-import {
  font-size: 12.5px;
  padding: 9px 11px;
  border-radius: 9px;
  white-space: nowrap;
}
.release-file-input {
  display: none;
}
.release-switch-row {
  display: flex;
  gap: 20px;
  justify-content: space-between;
  align-items: center;
  padding: 18px 0 22px;
  border-top: 1px solid #f0e4d4;
}
.release-switch-row b {
  font-size: 13.5px;
  color: #896645;
  font-weight: 600;
}
.release-switch-row p {
  font-size: 12.5px;
  color: #b69a79;
  margin-top: 7px;
}
.release-switch {
  position: relative;
  display: block;
  width: 40px;
  height: 23px;
  flex-shrink: 0;
  margin: 0;
}
.release-switch input {
  opacity: 0;
  width: 100%;
  height: 100%;
  position: absolute;
  margin: 0;
  cursor: pointer;
}
.release-switch > span {
  pointer-events: none;
  display: block;
  height: 23px;
  border-radius: 30px;
  background: #dfd4c3;
  transition: background 0.2s;
}
.release-switch > span::after {
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
.release-switch input:checked + span {
  background: #ec985b;
}
.release-switch input:checked + span::after {
  transform: translateX(17px);
}
.release-switch input:focus-visible + span {
  outline: 2px solid #d18448;
  outline-offset: 3px;
}
.release-two-fields {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 19px;
}
.release-form label {
  font-size: 13px;
  color: #a78a67;
  margin-bottom: 20px;
}
.release-form input,
.release-form textarea,
.release-announcement textarea {
  font-size: 13.5px;
  background: #fffdf9;
  border-color: #e9ddca;
  border-radius: 9px;
  margin-top: 8px;
}
.release-form textarea,
.release-announcement textarea {
  line-height: 1.9;
  resize: vertical;
}
.release-minimum {
  display: block;
  max-width: 290px;
}
.release-minimum > small {
  display: block;
  font-size: 12.5px;
  font-weight: 400;
  line-height: 1.7;
  color: #b8a080;
  margin-top: 8px;
}
.release-package {
  border-top: 1px solid #efe4d4;
  border-bottom: 1px solid #efe4d4;
  margin: 24px 0;
}
.release-package summary {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px 0;
  font-size: 13px;
  color: #a98a62;
  cursor: pointer;
  list-style: none;
}
.release-package summary::-webkit-details-marker {
  display: none;
}
.release-package summary > span {
  display: flex;
  align-items: center;
  gap: 8px;
}
.release-package[open] summary > svg {
  transform: rotate(180deg);
}
.release-package > div {
  padding-top: 3px;
}
.release-mono {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12.5px !important;
}
.release-form-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 15px;
  margin-top: 23px;
}
.release-form-footer > span {
  font-size: 12px;
  color: #bca181;
}
.release-form-footer > .primary {
  font-size: 13px;
  padding: 12px 17px;
  border-radius: 9px;
  white-space: nowrap;
}
.release-preview {
  padding-top: 14px;
}
.release-preview-heading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  font-size: 12.5px;
  color: #b39a7e;
  margin-bottom: 19px;
}
.release-preview-card {
  position: relative;
  overflow: hidden;
  border: 0;
  border-radius: 25px;
  padding: 19px 24px 23px;
  background: #fff8eb;
  text-align: center;
  box-shadow: 0 17px 35px #b3875220;
}
.release-preview-glow {
  position: absolute;
  inset: -80px -30px auto;
  height: 240px;
  border-radius: 50%;
  background: radial-gradient(ellipse, #ffe1ad, transparent 70%);
}
.release-preview-card > img {
  width: 116px;
  height: 116px;
  object-fit: contain;
  margin: 0 auto;
  position: relative;
}
.release-preview-brand {
  display: block;
  font-size: 12.5px;
  color: #c59969;
  margin-top: 9px;
}
.release-preview-card h3 {
  font-size: 21px;
  color: #99663b;
  line-height: 1.6;
  margin-top: 10px;
  overflow-wrap: anywhere;
}
.release-preview-version {
  font-size: 12.5px;
  color: #b69a78;
  margin-top: 8px;
}
.release-preview-notes {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  text-align: left;
  font-size: 13px;
  line-height: 2;
  color: #ac8967;
  margin: 23px 0;
  min-height: 60px;
}
.release-preview-action {
  background: linear-gradient(110deg, #ff9860, #f06942);
  color: #fff8e8;
  font-size: 13.5px;
  font-weight: 600;
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 9px;
  border-radius: 100px;
  padding: 13px;
  box-shadow: 0 5px 13px #e2935c23;
}
.release-preview-later {
  display: block;
  font-size: 12px;
  color: #b99b7b;
  margin-top: 16px;
}
.release-announcement {
  margin-top: 26px;
}
.release-announcement-icon {
  display: grid;
  place-items: center;
  height: 45px;
  width: 45px;
  border-radius: 15px;
  color: #c99a64;
  background: #fcf0dc;
}
.release-announcement-input {
  position: relative;
  margin: 0;
}
.release-announcement-input textarea {
  margin: 0;
  padding-bottom: 27px;
}
.release-announcement-input > span {
  position: absolute;
  bottom: 13px;
  right: 13px;
  font-size: 12px;
  font-weight: 400;
  color: #b89d7a;
}
.release-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  background: #f3f3e6;
  color: #91965e;
  border: 0;
  border-radius: 10px;
  padding: 12px 15px;
  font-size: 13px;
  margin-bottom: 20px;
}
.release-loading {
  padding: 30px 0;
  font-size: 13.5px;
  color: #b09978;
}
@media (max-width: 1100px) {
  .release-columns {
    grid-template-columns: minmax(0, 1fr) 270px;
    gap: 18px;
  }
}
@media (max-width: 850px) {
  .release-columns {
    grid-template-columns: 1fr;
  }
  .release-preview {
    max-width: 330px;
    width: 100%;
    margin: 0 auto;
  }
}
@media (max-width: 550px) {
  .release-form,
  .release-announcement {
    padding: 20px;
  }
  .release-card-heading {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .release-two-fields {
    gap: 13px;
  }
  .release-form-footer {
    flex-wrap: wrap;
  }
}
</style>
