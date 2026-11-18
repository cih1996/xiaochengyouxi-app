<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const cfg = ref(null),
  secretKey = ref(""),
  error = ref(""),
  notice = ref(""),
  saving = ref(false),
  testFile = ref(""),
  testing = ref(false),
  test = ref(null),
  upload = ref({ file: null, name: "", progress: 0, busy: false, result: null, error: "" });
const controller = new AbortController();
const REGIONS = ["ap-guangzhou", "ap-shanghai", "ap-nanjing", "ap-beijing", "ap-chengdu", "ap-chongqing", "ap-hongkong"];
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try {
    cfg.value = await props.api("/admin/cos-config", { signal: controller.signal });
    secretKey.value = "";
  } catch (e) {
    fail(e);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    const { secret_key_set, ready, default_host, revision, updated_at, ...config } = cfg.value;
    cfg.value = await props.api("/admin/cos-config", {
      method: "PUT",
      body: { ...config, secret_key: secretKey.value },
      signal: controller.signal,
    });
    secretKey.value = "";
    notice.value = cfg.value.enabled
      ? "已保存。官网和 App 下载会优先走 COS，COS 上没有的文件仍由服务器提供。"
      : "已保存，COS 下载未启用，下载仍由服务器提供。";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
async function runTest() {
  testing.value = true;
  test.value = null;
  error.value = "";
  try {
    test.value = await props.api("/admin/cos-config/test", {
      method: "POST",
      body: { file: testFile.value.trim() },
      signal: controller.signal,
    });
  } catch (e) {
    fail(e);
  } finally {
    testing.value = false;
  }
}
function pick(event) {
  const file = event.target.files?.[0] || null;
  upload.value = { file, name: file ? file.name : "", progress: 0, busy: false, result: null, error: "" };
}
async function sha256(file) {
  const digest = await crypto.subtle.digest("SHA-256", await file.arrayBuffer());
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
async function startUpload() {
  const u = upload.value;
  if (!u.file || !u.name.trim()) return;
  u.busy = true;
  u.error = "";
  u.result = null;
  u.progress = 0;
  try {
    const name = u.name.trim();
    const { url, key } = await props.api("/admin/cos/upload-url", { method: "POST", body: { file: name }, signal: controller.signal });
    const digest = await sha256(u.file);
    await new Promise((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open("PUT", url);
      xhr.setRequestHeader("Content-Type", "application/vnd.android.package-archive");
      xhr.upload.onprogress = (e) => {
        if (e.lengthComputable) u.progress = Math.round((e.loaded * 100) / e.total);
      };
      xhr.onload = () => (xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new Error(`COS 返回 ${xhr.status}`)));
      xhr.onerror = () => reject(new Error("上传失败，请检查网络或存储桶跨域设置"));
      xhr.send(u.file);
    });
    u.progress = 100;
    u.result = { key, name, size: u.file.size, sha256: digest, url: `${location.origin}/download/${name}` };
  } catch (e) {
    u.error = e.message;
    if (e.status === 401) emit("unauthorized", e);
  } finally {
    u.busy = false;
  }
}
async function copy(text) {
  try {
    await navigator.clipboard.writeText(text);
    notice.value = "已复制";
  } catch {
    /* ignore */
  }
}
const fmtSize = (n) => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.round(n / 1024)} KB`);
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>

<template>
  <div class="cos-page">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="cos-notice"><Icon name="check" :size="15" />{{ notice }}</div>
    <section v-if="!cfg && !error" class="admin-card cos-card"><p class="cos-muted">正在读取 COS 配置…</p></section>

    <form v-if="cfg" class="admin-card cos-card" @submit.prevent="save">
      <header class="cos-head">
        <div>
          <h2>腾讯云 COS</h2>
          <p>安装包存放在 COS，官网和 App 内更新的下载地址不变（/download/文件名），服务器签发 2 小时有效的下载链接后跳转到 COS 下载域名。COS 上没有的文件仍由服务器提供。</p>
        </div>
        <span class="cos-state" :class="{ on: cfg.enabled && cfg.ready }">{{ cfg.enabled && cfg.ready ? "已启用" : cfg.ready ? "已配置，未启用" : "未配置" }}</span>
      </header>
      <div class="cos-toggle">
        <div>
          <b>下载走 COS</b>
          <p>开启前请先“检测”，确认下载域名能正常下载。</p>
        </div>
        <label class="cos-switch"><input v-model="cfg.enabled" type="checkbox" aria-label="下载走 COS" /><span></span></label>
      </div>
      <div class="cos-fields">
        <label>SecretId<input v-model="cfg.secret_id" maxlength="64" autocomplete="off" placeholder="访问管理 → API 密钥" /></label>
        <label>
          SecretKey
          <input v-model="secretKey" type="password" maxlength="64" autocomplete="new-password" :placeholder="cfg.secret_key_set ? '已设置，留空保持不变' : '与 SecretId 配对的密钥'" />
        </label>
        <label>存储桶<input v-model="cfg.bucket" maxlength="64" placeholder="如 xiaochengyouxi-1308564197" /></label>
        <label>地域<select v-model="cfg.region"><option v-for="r in REGIONS" :key="r" :value="r">{{ r }}</option></select></label>
        <label>目录<input v-model="cfg.prefix" maxlength="64" placeholder="releases/" /></label>
        <label>
          下载域名
          <div class="cos-domain">
            <select v-model="cfg.download_scheme" aria-label="下载协议"><option value="https">https://</option><option value="http">http://</option></select>
            <input v-model="cfg.download_domain" maxlength="100" placeholder="dl.lingzidev.com" aria-label="下载域名" />
          </div>
        </label>
      </div>
      <p class="cos-hint">
        COS 不允许用默认域名（{{ cfg.default_host || "存储桶.cos.地域.myqcloud.com" }}）分发 APK，必须在存储桶绑定自定义下载域名。
        <b>请使用 https：</b>App 内更新只允许 HTTPS，浏览器也会拦截从 HTTPS 页面发起的 HTTP 下载。存储桶保持“私有读写”即可。
      </p>
      <div class="cos-actions">
        <button class="primary" :disabled="saving"><Icon name="check" :size="16" />{{ saving ? "保存中…" : "保存配置" }}</button>
      </div>
    </form>

    <section v-if="cfg" class="admin-card cos-card">
      <h3>检测</h3>
      <p class="cos-muted">用已保存的配置检查 COS 上的文件，并通过下载域名实际请求一次。</p>
      <div class="cos-row">
        <input v-model="testFile" maxlength="120" placeholder="文件名，留空检测 orange-rewards.apk" aria-label="检测文件名" />
        <button type="button" class="outline" :disabled="testing" @click="runTest"><Icon name="search" :size="14" />{{ testing ? "检测中…" : "检测" }}</button>
      </div>
      <dl v-if="test" class="cos-result">
        <div>
          <dt>COS 文件 {{ test.file }}</dt>
          <dd :class="test.object.ok && test.object.exists ? 'ok' : 'bad'">
            {{ !test.object.ok ? `检查失败：${test.object.error}` : test.object.exists ? `存在 · ${fmtSize(test.object.size)} · MD5 ${test.object.etag}` : "不存在（会用服务器本地文件）" }}
          </dd>
        </div>
        <div>
          <dt>通过下载域名下载</dt>
          <dd :class="test.download.ok ? 'ok' : 'bad'">
            {{ test.download.ok ? "正常" : test.download.code === "UserCnameInvalid" ? "域名绑定未生效（COS：UserCnameInvalid），请检查 DNS 的 CNAME 和存储桶的自定义域名状态" : `失败：${test.download.code || test.download.error || ("HTTP " + test.download.status)}` }}
          </dd>
        </div>
      </dl>
    </section>

    <section v-if="cfg" class="admin-card cos-card">
      <h3>上传安装包</h3>
      <p class="cos-muted">从你的电脑直接上传到 COS（不经过服务器）。上传后可在“版本发布”里用下面的下载地址、SHA-256 和大小填写发布信息。</p>
      <div class="cos-row">
        <input type="file" accept=".apk" aria-label="选择安装包" @change="pick" />
        <input v-model="upload.name" maxlength="120" placeholder="保存为文件名，如 xiaochengyouxi-0.3.23-26.apk" aria-label="保存文件名" />
        <button type="button" class="primary small" :disabled="!upload.file || upload.busy || !cfg.ready" @click="startUpload">
          <Icon name="download" :size="14" />{{ upload.busy ? `上传中 ${upload.progress}%` : "上传" }}
        </button>
      </div>
      <div v-if="upload.busy" class="cos-progress"><i :style="{ width: `${upload.progress}%` }"></i></div>
      <p v-if="upload.error" class="cos-bad">{{ upload.error }}</p>
      <dl v-if="upload.result" class="cos-result">
        <div><dt>COS 路径</dt><dd>{{ upload.result.key }}</dd></div>
        <div><dt>下载地址</dt><dd class="mono">{{ upload.result.url }} <button type="button" class="linkish" @click="copy(upload.result.url)">复制</button></dd></div>
        <div><dt>SHA-256</dt><dd class="mono">{{ upload.result.sha256 }} <button type="button" class="linkish" @click="copy(upload.result.sha256)">复制</button></dd></div>
        <div><dt>大小</dt><dd>{{ upload.result.size }} 字节（{{ fmtSize(upload.result.size) }}）</dd></div>
      </dl>
    </section>
  </div>
</template>

<style scoped>
.cos-card { padding: 18px 20px; margin-bottom: 12px; }
.cos-card h2 { font-size: 17px; color: #6f4b33; margin: 0 0 6px; }
.cos-card h3 { font-size: 15px; color: #4f3627; margin: 0 0 6px; }
.cos-head { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; }
.cos-head p, .cos-muted { color: #9a8274; font-size: 13.5px; line-height: 1.7; margin: 0; max-width: 780px; }
.cos-state { font-size: 13px; border-radius: 999px; padding: 5px 12px; background: #f7efe8; color: #8a6f61; white-space: nowrap; }
.cos-state.on { background: #eefaf1; color: #2f8a4c; }
.cos-notice { display: flex; gap: 6px; align-items: center; color: #2f8a4c; background: #eefaf1; border-radius: 12px; padding: 10px 14px; margin-bottom: 12px; font-size: 13.5px; }
.cos-toggle { display: flex; justify-content: space-between; align-items: center; gap: 20px; border-top: 1px solid #f2e8d9; padding: 14px 0; margin-top: 12px; }
.cos-toggle b { font-size: 14px; color: #6f4b33; }
.cos-toggle p { font-size: 13px; color: #a8907f; margin: 4px 0 0; }
.cos-switch { position: relative; display: block; width: 42px; height: 24px; flex-shrink: 0; margin: 0; }
.cos-switch input { opacity: 0; width: 100%; height: 100%; position: absolute; inset: 0; margin: 0; padding: 0; cursor: pointer; }
.cos-switch > span { pointer-events: none; display: block; height: 24px; border-radius: 30px; background: #dfd4c3; transition: background 0.2s; }
.cos-switch > span::after { content: ""; position: absolute; height: 18px; width: 18px; top: 3px; left: 3px; background: #fff; border-radius: 50%; transition: transform 0.2s; }
.cos-switch input:checked + span { background: #ec985b; }
.cos-switch input:checked + span::after { transform: translateX(18px); }
.cos-fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px 14px; }
.cos-fields label { margin: 0; display: grid; gap: 6px; font-size: 13px; }
.cos-fields input, .cos-fields select { margin: 0; padding: 9px 11px; font-size: 13.5px; border-radius: 10px; }
.cos-domain { display: flex; gap: 6px; }
.cos-domain select { width: 96px; flex-shrink: 0; }
.cos-hint { font-size: 13px; color: #8a6f61; background: #fbf6f1; border-radius: 10px; padding: 10px 12px; line-height: 1.7; margin: 14px 0 0; }
.cos-actions { margin-top: 14px; }
.cos-actions .primary { width: auto; padding: 10px 22px; margin: 0; }
.cos-row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-top: 10px; }
.cos-row input { flex: 1; min-width: 220px; margin: 0; padding: 9px 11px; font-size: 13.5px; border-radius: 10px; }
.cos-row input[type="file"] { flex: 0 1 auto; min-width: 0; padding: 7px; }
.cos-row .primary.small, .cos-row .outline { width: auto; margin: 0; padding: 9px 16px; display: inline-flex; gap: 6px; align-items: center; white-space: nowrap; }
.cos-progress { height: 6px; border-radius: 3px; background: #f3e9df; overflow: hidden; margin-top: 10px; }
.cos-progress i { display: block; height: 100%; background: linear-gradient(90deg, #ff9a62, #f65331); transition: width 0.2s; }
.cos-result { margin: 12px 0 0; display: grid; gap: 8px; }
.cos-result div { display: grid; grid-template-columns: 150px 1fr; gap: 10px; font-size: 13.5px; }
.cos-result dt { color: #a8907f; }
.cos-result dd { margin: 0; color: #4f3627; word-break: break-all; }
.cos-result dd.ok { color: #2f8a4c; }
.cos-result dd.bad, .cos-bad { color: #b03a2a; font-size: 13.5px; }
.mono { font-family: ui-monospace, monospace; font-size: 12.5px; }
.linkish { border: 0; background: none; color: #f65331; cursor: pointer; padding: 0 4px; font: inherit; }
@media (max-width: 900px) { .cos-fields { grid-template-columns: 1fr; } .cos-result div { grid-template-columns: 1fr; } }
</style>
