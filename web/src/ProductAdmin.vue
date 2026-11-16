<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import { productApiExamples } from "./product-api-examples";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const items = ref([]),
  total = ref(0),
  page = ref(1),
  search = ref(""),
  status = ref("all");
const loading = ref(false),
  saving = ref(false),
  error = ref(""),
  notice = ref("");
const config = ref(null),
  keyInfo = ref(null),
  analytics = ref(null),
  analyticsDays = ref(7),
  analyticsLoading = ref(false),
  displayConfig = ref(null),
  heroImageFile = ref(null),
  heroImagePreview = ref(""),
  keyVisible = ref(false),
  confirmRotate = ref(false);
const editing = ref(false),
  original = ref(null),
  draft = ref({}),
  imageFile = ref(null),
  imagePreview = ref("");
const controller = new AbortController();
const clock = ref(Date.now());
let clockTimer;
const pageSize = 20;
const statuses = {
  active: "上架中",
  scheduled: "待生效",
  expired: "已过期",
  disabled: "已下架",
};
const example = productApiExamples(location.origin).json;
const number = (n) => Number(n).toLocaleString("zh-CN");
function date(value) {
  return new Date(value).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}
function localDate(value) {
  const time = new Date(value);
  return new Date(time.getTime() - time.getTimezoneOffset() * 60000)
    .toISOString()
    .slice(0, 19);
}
function remainingSeconds(product, field) {
  const elapsed = Math.max(0, (clock.value - product.received_at) / 1000);
  return Math.max(0, Math.ceil(product[field] - elapsed));
}
function productStatus(product) {
  if (!product.enabled) return "disabled";
  if (remainingSeconds(product, "seconds_until_start") > 0) return "scheduled";
  return remainingSeconds(product, "seconds_until_expiry") > 0
    ? "active"
    : "expired";
}
function remaining(product) {
  const status = productStatus(product);
  if (status === "disabled" || status === "expired") return statuses[status];
  const seconds = remainingSeconds(
    product,
    status === "scheduled" ? "seconds_until_start" : "seconds_until_expiry",
  );
  const days = Math.floor(seconds / 86400);
  const time = [
    Math.floor(seconds / 3600) % 24,
    Math.floor(seconds / 60) % 60,
    seconds % 60,
  ]
    .map((value) => String(value).padStart(2, "0"))
    .join(":");
  return `${status === "scheduled" ? "距生效" : "剩余"} ${days ? days + "天 " : ""}${time}`;
}
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function loadProducts(nextPage = 1) {
  loading.value = true;
  error.value = "";
  try {
    const query = new URLSearchParams({
      q: search.value.trim(),
      status: status.value,
      page: String(nextPage),
      page_size: String(pageSize),
    });
    const result = await props.api("/admin/products?" + query, {
      signal: controller.signal,
    });
    items.value = result.items.map((product) => ({
      ...product,
      received_at: Date.now(),
    }));
    total.value = result.total;
    page.value = result.page;
  } catch (failure) {
    fail(failure);
  } finally {
    loading.value = false;
  }
}
async function refresh() {
  error.value = "";
  try {
    await Promise.all([
      loadProducts(page.value),
      props
        .api("/admin/products/config", { signal: controller.signal })
        .then((value) => (config.value = value)),
      props
        .api("/admin/products/api-key", { signal: controller.signal })
        .then((value) => (keyInfo.value = value)),
      props
        .api("/admin/products/display-config", { signal: controller.signal })
        .then((value) => {
          displayConfig.value = value;
          heroImagePreview.value = value.hero_image || "";
        }),
      loadAnalytics(),
    ]);
  } catch (failure) {
    fail(failure);
  }
}
async function loadAnalytics() {
  analyticsLoading.value = true;
  try {
    analytics.value = await props.api(
      `/admin/products/analytics?days=${analyticsDays.value}`,
      { signal: controller.signal },
    );
  } catch (failure) {
    fail(failure);
  } finally {
    analyticsLoading.value = false;
  }
}
function pickHeroImage(event) {
  const file = event.target.files[0];
  if (!file) return;
  if (heroImagePreview.value.startsWith("blob:"))
    URL.revokeObjectURL(heroImagePreview.value);
  heroImageFile.value = file;
  heroImagePreview.value = URL.createObjectURL(file);
}
async function saveHeroImage() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    let imageId = null;
    if (heroImageFile.value) {
      const reader = new FileReader();
      const base64 = await new Promise((resolve, reject) => {
        reader.onload = () => resolve(String(reader.result).split(",").pop());
        reader.onerror = reject;
        reader.readAsDataURL(heroImageFile.value);
      });
      const uploaded = await props.api("/admin/products/images", {
        method: "POST",
        body: { image_base64: base64 },
        signal: controller.signal,
      });
      imageId = uploaded.image_id;
    }
    displayConfig.value = await props.api("/admin/products/display-config", {
      method: "PUT",
      body: { image_id: imageId },
      signal: controller.signal,
    });
    heroImageFile.value = null;
    heroImagePreview.value = displayConfig.value.hero_image || "";
    notice.value = "捡漏展示图已保存";
  } catch (failure) {
    fail(failure);
  } finally {
    saving.value = false;
  }
}
function clearPreview() {
  if (imagePreview.value.startsWith("blob:"))
    URL.revokeObjectURL(imagePreview.value);
  imageFile.value = null;
  imagePreview.value = "";
}
function edit(product = null) {
  const starts = Date.now();
  clearPreview();
  error.value = "";
  notice.value = "";
  original.value = product;
  draft.value = product
    ? {
        title: product.title,
        url: product.url,
        price: product.price_text,
        commission_percent: String(product.commission_percent),
        platform: product.platform,
        category: product.category,
        external_id: product.external_id || "",
        starts_at: localDate(product.starts_at),
        expires_at: localDate(product.expires_at),
        enabled: product.enabled,
      }
    : {
        title: "",
        url: "",
        price: "",
        commission_percent: "",
        platform: "",
        category: "",
        external_id: "",
        starts_at: localDate(starts),
        expires_at: localDate(starts + 7 * 86400000),
        enabled: true,
      };
  imagePreview.value = product?.image || "";
  editing.value = true;
}
function closeEditor() {
  editing.value = false;
  clearPreview();
}
function pickImage(event) {
  const file = event.target.files[0];
  if (!file) return;
  clearPreview();
  imageFile.value = file;
  imagePreview.value = URL.createObjectURL(file);
}
async function saveProduct() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    const body = new FormData();
    for (const [key, value] of Object.entries(draft.value)) {
      const previous =
        original.value &&
        (key === "price"
          ? original.value.price_text
          : ["starts_at", "expires_at"].includes(key)
            ? localDate(original.value[key])
            : (original.value[key] ?? ""));
      if (original.value && String(value) === String(previous)) continue;
      body.append(
        key,
        ["starts_at", "expires_at"].includes(key)
          ? new Date(value).toISOString()
          : String(value),
      );
    }
    if (imageFile.value) body.append("image", imageFile.value);
    await props.api(
      "/admin/products" + (original.value ? "/" + original.value.id : ""),
      {
        method: original.value ? "PATCH" : "POST",
        body,
        signal: controller.signal,
      },
    );
    closeEditor();
    notice.value = "商品已保存";
    await loadProducts(page.value);
  } catch (failure) {
    fail(failure);
  } finally {
    saving.value = false;
  }
}
async function toggleProduct(product) {
  error.value = "";
  const body = new FormData();
  body.append("enabled", String(!product.enabled));
  try {
    await props.api("/admin/products/" + product.id, {
      method: "PATCH",
      body,
      signal: controller.signal,
    });
    notice.value = product.enabled ? "商品已下架" : "商品已上架";
    await loadProducts(page.value);
  } catch (failure) {
    fail(failure);
  }
}
async function saveConfig() {
  saving.value = true;
  error.value = "";
  try {
    config.value = await props.api("/admin/products/config", {
      method: "PUT",
      signal: controller.signal,
      body: {
        platform_share_percent: config.value.platform_share_percent,
        default_valid_days: config.value.default_valid_days,
      },
    });
    notice.value = "商品设置已保存";
    await loadProducts(page.value);
  } catch (failure) {
    fail(failure);
  } finally {
    saving.value = false;
  }
}
async function generateKey() {
  saving.value = true;
  error.value = "";
  try {
    keyInfo.value = await props.api("/admin/products/api-key", {
      method: "POST",
      body: {},
      signal: controller.signal,
    });
    keyVisible.value = true;
    confirmRotate.value = false;
    notice.value = "接入密钥已生成";
  } catch (failure) {
    fail(failure);
  } finally {
    saving.value = false;
  }
}
async function copy(value, message) {
  try {
    await navigator.clipboard.writeText(value);
    notice.value = message;
  } catch (failure) {
    fail(failure);
  }
}
onMounted(() => {
  refresh();
  clockTimer = setInterval(() => (clock.value = Date.now()), 1000);
});
onUnmounted(() => {
  clearInterval(clockTimer);
  controller.abort();
  clearPreview();
  if (heroImagePreview.value.startsWith("blob:"))
    URL.revokeObjectURL(heroImagePreview.value);
});
defineExpose({ refresh });
</script>

<template>
  <div class="product-admin">
    <div v-if="error && !editing" class="form-error" role="alert">
      {{ error }}
    </div>
    <div v-if="notice" class="product-notice">
      <Icon name="check" :size="16" />{{ notice }}
    </div>
    <section class="admin-card product-library">
      <div class="product-library-heading">
        <div>
          <h2>精选商品</h2>
          <span>共 {{ number(total) }} 件</span>
        </div>
        <button class="primary" :disabled="!config" @click="edit()">
          <Icon name="bag" :size="16" />新增商品
        </button>
      </div>
      <form class="product-filters" @submit.prevent="loadProducts(1)">
        <label class="product-search"
          ><Icon name="search" :size="17" /><input
            v-model="search"
            aria-label="搜索商品"
            placeholder="搜索标题或外部编号"
        /></label>
        <select
          v-model="status"
          aria-label="商品状态"
          @change="loadProducts(1)"
        >
          <option value="all">全部状态</option>
          <option v-for="(text, value) in statuses" :key="value" :value="value">
            {{ text }}
          </option>
        </select>
        <button class="outline">查询</button>
      </form>
      <div class="product-table-wrap">
        <table class="product-table">
          <thead>
            <tr>
              <th>商品</th>
              <th>券后价 / 返佣</th>
              <th>用户预计金币</th>
              <th>有效时间</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="product in items" :key="product.id">
              <td>
                <div class="product-cell">
                  <img :src="product.image" :alt="product.title" />
                  <div>
                    <b>{{ product.title }}</b><small>{{
                      [product.platform, product.category]
                        .filter(Boolean)
                        .join(" · ") || "—"
                    }}</small
                    ><small v-if="product.external_id">{{
                      product.external_id
                    }}</small>
                  </div>
                </div>
              </td>
              <td>
                <strong>¥{{ product.price_text }}</strong
                ><small class="product-sub"
                  >{{ product.commission_percent }}%</small
                >
              </td>
              <td>
                <span class="product-coins"
                  ><Icon name="coin" :size="15" />{{
                    number(product.reward_coins)
                  }}</span
                >
              </td>
              <td class="product-dates">
                {{ date(product.starts_at)
                }}<small class="product-sub"
                  >至 {{ date(product.expires_at) }}</small
                >
              </td>
              <td>
                <span class="product-status" :class="productStatus(product)">{{
                  statuses[productStatus(product)]
                }}</span>
                <small class="product-sub product-remaining">{{
                  remaining(product)
                }}</small>
              </td>
              <td>
                <div class="product-row-actions">
                  <button @click="edit(product)">编辑</button
                  ><button @click="toggleProduct(product)">
                    {{ product.enabled ? "下架" : "上架" }}
                  </button>
                </div>
              </td>
            </tr>
            <tr v-if="!items.length">
              <td colspan="6" class="product-empty">
                <Icon name="bag" :size="29" /><b>{{
                  loading
                    ? "正在读取商品…"
                    : search || status !== "all"
                      ? "暂无符合条件的商品"
                      : "还没有商品"
                }}</b
                ><span v-if="!loading && !search && status === 'all'"
                  >添加一件好物，开启新的发现</span
                >
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <footer class="product-pagination">
        <span>第 {{ page }} 页 · {{ number(total) }} 件</span>
        <div>
          <button
            :disabled="loading || page <= 1"
            @click="loadProducts(page - 1)"
          >
            <Icon name="back" :size="15" />上一页</button
          ><button
            :disabled="loading || page * pageSize >= total"
            @click="loadProducts(page + 1)"
          >
            下一页<Icon name="arrow" :size="15" />
          </button>
        </div>
      </footer>
    </section>
    <section class="admin-card product-analytics">
      <header>
        <div>
          <h2>商品分析</h2>
          <span>跳转次数 ÷ 详情打开次数</span>
        </div>
        <div class="product-analytics-days">
          <button
            v-for="value in [7, 30, 90]"
            :key="value"
            :class="{ active: analyticsDays === value }"
            @click="
              analyticsDays = value;
              loadAnalytics();
            "
          >
            近{{ value }}天
          </button>
        </div>
      </header>
      <div v-if="analyticsLoading" class="product-analytics-empty">
        正在读取商品数据…
      </div>
      <template v-else-if="analytics"
        ><div class="product-analytics-summary">
          <article>
            <span>曝光次数</span
            ><b>{{ number(analytics.impressions.count) }}</b>
          </article>
          <article>
            <span>详情打开</span
            ><b>{{ number(analytics.summary.detail_opens) }}</b>
          </article>
          <article>
            <span>外链跳转</span
            ><b>{{ number(analytics.summary.outbound_clicks) }}</b>
          </article>
          <article>
            <span>独立用户</span
            ><b>{{ number(analytics.summary.unique_users) }}</b>
          </article>
          <article class="accent">
            <span>跳转率</span
            ><b>{{
              analytics.summary_ctr == null
                ? "—"
                : (analytics.summary_ctr * 100).toFixed(1) + "%"
            }}</b>
          </article>
        </div>
        <div class="product-analytics-trend">
          <h3>每日趋势</h3>
          <div v-if="!analytics.daily?.length" class="product-analytics-empty">
            所选时段暂无趋势数据
          </div>
          <svg
            v-else
            viewBox="0 0 700 150"
            role="img"
            aria-label="商品每日趋势"
          >
            <polyline
              :points="
                analytics.daily
                  .map(
                    (item, index) =>
                      `${(index * 680) / Math.max(1, analytics.daily.length - 1) + 10},${140 - Math.min(120, ((item.outbound || 0) * 120) / Math.max(1, ...analytics.daily.map((day) => day.outbound || 0)))}`,
                  )
                  .join(' ')
              "
              fill="none"
              stroke="#dfa56d"
              stroke-width="3"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <polyline
              :points="
                analytics.daily
                  .map(
                    (item, index) =>
                      `${(index * 680) / Math.max(1, analytics.daily.length - 1) + 10},${140 - Math.min(120, ((item.details || 0) * 120) / Math.max(1, ...analytics.daily.map((day) => day.details || 0)))}`,
                  )
                  .join(' ')
              "
              fill="none"
              stroke="#91ae9a"
              stroke-width="2"
              stroke-dasharray="5 4"
            />
          </svg>
        </div>
        <div class="product-analytics-body">
          <div class="product-ranking">
            <h3>商品排行</h3>
            <div
              v-if="!analytics.products.length"
              class="product-analytics-empty"
            >
              所选时段暂无商品数据
            </div>
            <div
              v-for="(item, index) in analytics.products"
              :key="item.product_id"
              class="product-ranking-row"
            >
              <strong>{{ index + 1 }}</strong>
              <div>
                <b>{{ item.title }}</b><span
                  >曝光 {{ number(item.impressions) }} · 详情
                  {{ number(item.details) }} · 跳转
                  {{ number(item.outbound) }}</span
                >
              </div>
              <em>{{
                item.ctr == null ? "—" : (item.ctr * 100).toFixed(1) + "%"
              }}</em>
            </div>
          </div>
          <div class="product-analytics-note">
            <Icon name="shield" :size="18" />
            <p>
              统计按跳转次数 ÷ 详情打开次数计算。独立用户按服务端可识别账号或会话统计。
            </p>
          </div>
        </div></template
      >
    </section>
    <section class="admin-card product-display-config">
      <header>
        <div>
          <h2>捡漏展示图</h2>
          <span>未设置时使用简洁标题栏</span>
        </div>
        <button class="outline" :disabled="saving" @click="saveHeroImage">
          保存展示图
        </button>
      </header>
      <div class="product-display-form">
        <div class="product-display-preview">
          <img
            v-if="heroImagePreview"
            :src="heroImagePreview"
            alt="捡漏展示图"
          /><span v-else>暂无展示图</span>
        </div>
        <label
          >选择图片<input
            type="file"
            accept="image/png,image/jpeg,image/webp"
            aria-label="捡漏展示图"
            @change="pickHeroImage" /></label
        ><button
          v-if="displayConfig?.hero_image"
          class="product-display-clear"
          @click="
            heroImageFile = null;
            heroImagePreview = '';
            saveHeroImage();
          "
        >
          清除展示图
        </button>
      </div>
    </section>
    <div class="product-settings-grid">
      <section class="admin-card product-config">
        <h2><Icon name="settings" :size="18" />商品设置</h2>
        <form v-if="config" @submit.prevent="saveConfig">
          <label
            >平台分成<input
              v-model.number="config.platform_share_percent"
              aria-label="平台分成"
              type="number"
              min="0"
              max="100"
              step="0.01"
              required
            /><small>%</small></label
          ><button class="outline" :disabled="saving">保存设置</button>
        </form>
      </section>
      <section class="admin-card product-api">
        <h2><Icon name="send" :size="18" />商品接入</h2>
        <template v-if="keyInfo"
          ><div v-if="keyInfo.key" class="product-key">
            <input
              :value="keyInfo.key"
              :type="keyVisible ? 'text' : 'password'"
              aria-label="商品接入密钥"
              readonly
            /><button
              :aria-label="keyVisible ? '隐藏密钥' : '显示密钥'"
              @click="keyVisible = !keyVisible"
            >
              <Icon :name="keyVisible ? 'eye-off' : 'eye'" :size="17" /></button
            ><button @click="copy(keyInfo.key, '密钥已复制')">复制</button>
          </div>
          <div v-if="confirmRotate" class="product-key-confirm">
            <b>轮换接入密钥？</b><span>原密钥将立即失效。</span>
            <div>
              <button class="outline" @click="confirmRotate = false">
                取消</button
              ><button class="primary" :disabled="saving" @click="generateKey">
                确认轮换
              </button>
            </div>
          </div>
          <button
            v-else
            class="outline"
            :disabled="saving"
            @click="keyInfo.key ? (confirmRotate = true) : generateKey()"
          >
            {{ keyInfo.key ? "轮换密钥" : "生成接入密钥" }}
          </button>
          <div class="product-doc-links">
            <a href="/docs/product-api" target="_blank" rel="noopener"
              ><Icon name="list" :size="15" />接口文档<Icon
                name="arrow"
                :size="12" /></a
            >
          </div>
          <details class="product-example">
            <summary>JSON 示例<Icon name="chevron" :size="14" /></summary>
            <pre>{{ example }}</pre>
            <button @click="copy(example, '接入示例已复制')">复制示例</button>
          </details>
        </template>
      </section>
    </div>
    <div
      v-if="editing"
      class="product-editor-backdrop"
      @click.self="closeEditor"
    >
      <section
        class="product-editor"
        role="dialog"
        aria-modal="true"
        :aria-label="original ? '编辑商品' : '新增商品'"
      >
        <header>
          <div>
            <h2>{{ original ? "编辑商品" : "新增商品" }}</h2>
          </div>
          <button type="button" aria-label="关闭商品编辑" @click="closeEditor">
            <Icon name="close" />
          </button>
        </header>
        <form @submit.prevent="saveProduct">
          <div class="product-image-edit">
            <img v-if="imagePreview" :src="imagePreview" alt="商品图片预览" />
            <div>
              <b>商品图片</b><span>PNG / JPEG / WebP · 最大 5MB</span
              ><input
                type="file"
                name="image"
                aria-label="商品图片"
                accept="image/png,image/jpeg,image/webp"
                :required="!original"
                @change="pickImage"
              />
            </div>
          </div>
          <label
            >商品标题<input
              v-model="draft.title"
              name="title"
              required /></label
          ><label
            >商品链接<input
              v-model="draft.url"
              name="url"
              type="url"
              placeholder="https://"
              required
          /></label>
          <div class="product-form-grid">
            <label
              >券后价（元）<input
                v-model="draft.price"
                name="price"
                type="number"
                min="0.01"
                step="0.01"
                required /></label
            ><label
              >返佣比例（%）<input
                v-model="draft.commission_percent"
                name="commission_percent"
                type="number"
                min="0"
                max="100"
                step="0.01"
                required /></label
            ><label
              >平台<input
                v-model="draft.platform"
                name="platform"
                placeholder="选填" /></label
            ><label
              >分类<input
                v-model="draft.category"
                name="category"
                placeholder="选填" /></label
            ><label
              >生效时间<input
                v-model="draft.starts_at"
                name="starts_at"
                type="datetime-local"
                step="1"
                required /></label
            ><label
              >到期时间<input
                v-model="draft.expires_at"
                name="expires_at"
                type="datetime-local"
                step="1"
                required
            /></label>
          </div>
          <label
            >外部编号<input
              v-model="draft.external_id"
              name="external_id"
              placeholder="选填" /></label
          ><label class="product-enabled"
            ><input v-model="draft.enabled" type="checkbox" />上架商品</label
          >
          <div v-if="error" class="form-error" role="alert">{{ error }}</div>
          <footer>
            <button type="button" class="outline" @click="closeEditor">
              取消</button
            ><button class="primary" :disabled="saving">
              {{ saving ? "保存中…" : "保存商品" }}
            </button>
          </footer>
        </form>
      </section>
    </div>
  </div>
</template>

<style scoped>
.product-admin {
  max-width: 1280px;
}
.product-analytics,
.product-display-config {
  margin-top: 22px;
  padding: 25px 27px;
}
.product-analytics > header,
.product-display-config > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 20px;
}
.product-analytics > header > div:first-child span,
.product-display-config > header > div:first-child span {
  display: block;
  margin-top: 7px;
  color: #b2997d;
  font-size: 10px;
}
.product-analytics-days {
  display: flex;
  gap: 4px;
  padding: 4px;
  border-radius: 8px;
  background: #f8eee2;
}
.product-analytics-days button {
  padding: 8px 11px;
  border-radius: 6px;
  color: #ae9375;
  font-size: 10px;
}
.product-analytics-days button.active {
  background: #fffaf4;
  color: #c38652;
  box-shadow: 0 2px 5px #9f6f3320;
}
.product-analytics-summary {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 12px;
}
.product-analytics-summary article {
  padding: 15px 13px;
  border: 1px solid #eee1d1;
  border-radius: 10px;
  background: #fffcf8;
}
.product-analytics-summary span {
  display: block;
  color: #b29a7e;
  font-size: 10px;
}
.product-analytics-summary b {
  display: block;
  margin-top: 12px;
  color: #95704d;
  font-size: 21px;
  font-weight: 600;
}
.product-analytics-summary .accent b {
  color: #d78951;
}
.product-analytics-body {
  display: grid;
  grid-template-columns: 1.4fr 1fr;
  gap: 20px;
  margin-top: 22px;
}
.product-ranking h3 {
  font-size: 13px;
  color: #94704e;
  margin-bottom: 14px;
}
.product-ranking-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 13px 0;
  border-top: 1px solid #f1e5d6;
}
.product-ranking-row > strong {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  background: #f7ebdb;
  color: #bb8a5b;
  font-size: 9px;
  font-weight: 500;
}
.product-ranking-row > div {
  flex: 1;
  min-width: 0;
}
.product-ranking-row b {
  display: block;
  color: #8f6b4b;
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.product-ranking-row span {
  display: block;
  color: #b69b7b;
  font-size: 9px;
  margin-top: 5px;
}
.product-ranking-row em {
  font-style: normal;
  color: #c48959;
  font-size: 11px;
}
.product-analytics-note {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 15px;
  border-radius: 10px;
  background: #fbf3e6;
  color: #b19676;
  font-size: 10px;
  line-height: 1.8;
}
.product-analytics-note svg {
  flex-shrink: 0;
  color: #c8a27b;
}
.product-analytics-empty {
  padding: 45px 10px;
  text-align: center;
  color: #bba383;
  font-size: 11px;
}
.product-display-form {
  display: flex;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}
.product-display-preview {
  width: 190px;
  height: 78px;
  border-radius: 11px;
  border: 1px dashed #e7d4bd;
  background: #fff7ea;
  display: grid;
  place-items: center;
  overflow: hidden;
  color: #b59a7a;
  font-size: 10px;
}
.product-display-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.product-display-form label {
  margin: 0;
  font-size: 10px;
  font-weight: 400;
  color: #ad9273;
}
.product-display-form input {
  display: block;
  margin-top: 8px;
  font-size: 10px;
  width: 220px;
}
.product-display-clear {
  font-size: 10px;
  color: #ba8657;
}
.product-analytics-trend {
  margin-top: 22px;
  border-top: 1px solid #f0e4d5;
  padding-top: 18px;
}
.product-analytics-trend h3 {
  font-size: 12px;
  color: #95704e;
  margin-bottom: 10px;
}
.product-analytics-trend svg {
  width: 100%;
  height: 125px;
  background: linear-gradient(to bottom, #fffdf9, #fff8ef);
  border-radius: 10px;
}
.product-library {
  overflow: hidden;
}
.product-library-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 25px 27px 20px;
}
.product-library-heading > div {
  display: flex;
  gap: 13px;
  align-items: baseline;
}
.product-admin h2 {
  color: #704b31;
  font-size: 17px;
}
.product-library-heading span {
  color: #b29479;
  font-size: 12px;
}
.product-admin button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}
.product-filters {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 27px 22px;
}
.product-search {
  flex: 1 1 0;
  min-width: 0;
  display: flex;
  gap: 8px;
  align-items: center;
  border: 1px solid #eadfd4;
  border-radius: 10px;
  padding: 0 12px;
  margin: 0;
  height: 42px;
  color: #b69a83;
}
.product-search input {
  border: 0;
  background: transparent;
  outline: 0;
  width: 100%;
  min-width: 0;
  padding: 11px 0;
  margin: 0;
  font-size: 12px;
}
.product-filters select {
  flex: 0 0 140px;
  width: 140px;
  height: 42px;
  border: 1px solid #eadfd4;
  border-radius: 10px;
  padding: 9px 12px;
  font-size: 12px;
  color: #86654b;
  background: #fffdf9;
}
.product-filters > button {
  flex: 0 0 68px;
  white-space: nowrap;
  height: 42px;
}
.product-table-wrap {
  overflow-x: auto;
}
.product-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
  text-align: left;
}
.product-table th {
  background: #fcf8f2;
  color: #b49a84;
  font-size: 11px;
  font-weight: 500;
  padding: 13px 18px;
  white-space: nowrap;
}
.product-table td {
  padding: 16px 18px;
  border-top: 1px solid #f1e8df;
  color: #88684e;
}
.product-cell {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 220px;
}
.product-cell img {
  width: 60px;
  height: 60px;
  border-radius: 11px;
  object-fit: cover;
  background: #fff5e8;
}
.product-cell b {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  max-width: 260px;
  line-height: 1.6;
  font-size: 12px;
  color: #795539;
}
.product-cell small,
.product-sub {
  display: block;
  margin-top: 5px;
  color: #b39a83;
  font-size: 10px;
}
.product-coins {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #eb8c24;
  font-weight: 700;
  white-space: nowrap;
}
.product-dates {
  white-space: nowrap;
  font-size: 11px;
}
.product-remaining {
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}
.product-doc-links {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 17px;
  margin-top: 22px;
}
.product-doc-links a {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #b8875e;
}
.product-doc-links a:hover {
  color: #e39150;
}
.product-status {
  padding: 5px 8px;
  border-radius: 6px;
  color: #a38d78;
  background: #f5f0eb;
  font-size: 10px;
  white-space: nowrap;
}
.product-status.active {
  background: #edf7ec;
  color: #6d9b67;
}
.product-status.scheduled {
  background: #fff2d8;
  color: #bc953d;
}
.product-row-actions {
  display: flex;
  gap: 12px;
  white-space: nowrap;
}
.product-row-actions button {
  font-size: 11px;
  color: #ba8053;
}
.product-empty {
  text-align: center;
  height: 210px;
}
.product-empty > * {
  display: block;
  margin: auto;
}
.product-empty svg {
  color: #cfb49b;
  margin-bottom: 12px;
}
.product-empty b {
  font-size: 13px;
  font-weight: 500;
}
.product-empty span {
  margin-top: 9px;
  color: #bea68f;
  font-size: 11px;
}
.product-pagination {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px 24px;
  border-top: 1px solid #f1e8df;
  color: #b1967f;
  font-size: 11px;
}
.product-pagination > div {
  display: flex;
  gap: 18px;
}
.product-pagination button:disabled {
  opacity: 0.35;
}
.product-settings-grid {
  display: grid;
  grid-template-columns: 1fr 1.15fr;
  gap: 22px;
  margin-top: 22px;
}
.product-settings-grid > section {
  padding: 25px 27px;
}
.product-settings-grid h2 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 22px;
}
.product-config form {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 17px;
}
.product-config label {
  font-size: 11px;
  color: #9c7d62;
  position: relative;
  flex: 1;
  min-width: 110px;
}
.product-config input {
  display: block;
  width: 100%;
  margin-top: 9px;
  padding: 10px 32px 10px 12px;
  border: 1px solid #eadfd4;
  border-radius: 9px;
  background: #fffdf9;
  color: #755038;
  outline: none;
}
.product-config small {
  position: absolute;
  right: 12px;
  bottom: 11px;
  color: #b69d86;
}
.product-key {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}
.product-key input {
  flex: 1;
  width: 0;
  min-width: 0;
  border: 1px solid #eadfd4;
  border-radius: 9px;
  padding: 11px;
  font: 11px monospace;
  color: #856146;
  background: #fffdf9;
}
.product-key > button {
  font-size: 11px;
  color: #a07c5b;
  flex-shrink: 0;
}
.product-key-confirm {
  border: 1px solid #f2cfb4;
  border-radius: 10px;
  padding: 14px;
  background: #fff8ee;
}
.product-key-confirm b {
  font-size: 12px;
}
.product-key-confirm span {
  display: block;
  margin: 6px 0 13px;
  font-size: 11px;
  color: #b38b68;
}
.product-key-confirm > div {
  display: flex;
  gap: 9px;
  justify-content: flex-end;
}
.product-example {
  margin-top: 18px;
  padding-top: 15px;
  border-top: 1px solid #f0e5da;
}
.product-example summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  color: #ac8b6c;
  font-size: 11px;
}
.product-example pre {
  overflow-x: auto;
  background: #fcf5eb;
  padding: 14px;
  border-radius: 8px;
  font: 10px/1.8 monospace;
  margin: 13px 0 8px;
  color: #947153;
}
.product-example button {
  color: #b78b62;
  font-size: 10px;
}
.product-notice {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 12px 15px;
  margin-bottom: 16px;
  background: #edf7ed;
  border: 1px solid #dbead7;
  border-radius: 10px;
  color: #73926c;
  font-size: 12px;
}
.product-editor-backdrop {
  position: fixed;
  inset: 0;
  background: #45301d55;
  z-index: 80;
  display: flex;
  justify-content: flex-end;
  backdrop-filter: blur(3px);
}
.product-editor {
  width: 540px;
  max-width: 100%;
  height: 100%;
  overflow-y: auto;
  background: #fffdf9;
  box-shadow: -10px 0 50px #5f37151a;
  padding: 26px 29px;
}
.product-editor header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 25px;
}
.product-editor h2 {
  font-size: 21px;
}
.product-editor header button {
  color: #ad8d70;
}
.product-editor form > label,
.product-form-grid label {
  display: block;
  margin-bottom: 17px;
  font-size: 11px;
  color: #a08063;
}
.product-editor label > input:not([type="checkbox"]) {
  display: block;
  width: 100%;
  margin-top: 8px;
  border: 1px solid #e9dcd0;
  border-radius: 9px;
  background: #fff;
  color: #755239;
  padding: 11px 12px;
  font-size: 12px;
  outline: none;
}
.product-editor input:focus {
  border-color: #ecaa72;
}
.product-form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0 16px;
}
.product-image-edit {
  display: flex;
  gap: 18px;
  align-items: center;
  padding: 17px;
  border: 1px dashed #e8cdb1;
  border-radius: 13px;
  background: #fff8ee;
  margin-bottom: 23px;
}
.product-image-edit > img {
  width: 94px;
  height: 94px;
  object-fit: cover;
  border-radius: 10px;
}
.product-image-edit > div {
  min-width: 0;
}
.product-image-edit b {
  display: block;
  font-size: 12px;
  color: #9a704c;
}
.product-image-edit span {
  display: block;
  color: #b89a7e;
  font-size: 10px;
  margin: 7px 0 12px;
}
.product-image-edit input {
  max-width: 100%;
  width: 240px;
  font-size: 10px;
  color: #aa8664;
}
.product-enabled {
  display: flex !important;
  gap: 8px;
  align-items: center;
}
.product-enabled input {
  width: 16px;
  height: 16px;
  margin: 0;
  accent-color: #f68a4a;
}
.product-editor footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 18px 0 6px;
  border-top: 1px solid #f0e6dc;
}
@media (max-width: 1000px) {
  .product-analytics-summary {
    grid-template-columns: repeat(3, 1fr);
  }
  .product-analytics-body {
    grid-template-columns: 1fr;
  }
  .product-display-config > header {
    align-items: flex-start;
  }
  .product-settings-grid {
    grid-template-columns: 1fr;
  }
  .product-library-heading,
  .product-settings-grid > section {
    padding: 21px;
  }
  .product-filters {
    padding: 0 21px 20px;
  }
}
@media (max-width: 650px) {
  .product-analytics,
  .product-display-config {
    padding: 20px 16px;
  }
  .product-analytics > header,
  .product-display-config > header {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .product-analytics-days {
    width: 100%;
  }
  .product-analytics-days button {
    flex: 1;
  }
  .product-analytics-summary {
    grid-template-columns: 1fr 1fr;
  }
  .product-analytics-summary b {
    font-size: 18px;
  }
  .product-display-preview {
    width: 100%;
    height: 98px;
  }
  .product-display-form label {
    width: 100%;
  }
  .product-display-form input {
    width: 100%;
  }
  .product-library-heading {
    padding: 18px 15px;
  }
  .product-library-heading > div {
    display: block;
  }
  .product-library-heading span {
    display: block;
    margin-top: 6px;
    font-size: 10px;
  }
  .product-filters {
    padding: 0 15px 16px;
    flex-wrap: wrap;
  }
  .product-search {
    flex-basis: 100%;
  }
  .product-filters select {
    flex: 1;
  }
  .product-settings-grid > section {
    padding: 20px 16px;
  }
  .product-editor {
    padding: 22px 18px;
  }
  .product-image-edit {
    padding: 12px;
    gap: 12px;
  }
  .product-image-edit > img {
    width: 68px;
    height: 68px;
  }
  .product-form-grid {
    gap: 0 10px;
  }
  .product-editor input[type="datetime-local"] {
    padding: 10px 5px;
    font-size: 10px;
  }
  .product-pagination {
    padding: 15px;
  }
  .product-pagination > div {
    gap: 11px;
  }
}
</style>
