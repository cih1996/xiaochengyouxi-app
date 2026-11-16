<script setup>
import { ref, computed } from "vue";
import Icon from "./Icon.vue";
import { productApiExamples } from "./product-api-examples";
const separateImage = ref(false),
  notice = ref(""),
  error = ref("");
const examples = computed(() =>
  productApiExamples(location.origin, separateImage.value),
);
const baseUrl = location.origin;
const imageExample = JSON.stringify(
  { image_base64: "图片文件的 Base64 字符串" },
  null,
  2,
);
async function copy(text) {
  error.value = "";
  try {
    await navigator.clipboard.writeText(text);
    notice.value = "已复制";
  } catch (failure) {
    error.value = failure.message;
  }
}
</script>
<template>
  <div class="product-doc-page">
    <header class="product-doc-header">
      <a href="/" class="brand"
        ><span class="brand-symbol">橙</span><b>小橙有喜</b></a
      ><a href="/admin">管理中心<Icon name="arrow" :size="14" /></a>
    </header>
    <main>
      <section class="product-doc-hero">
        <h1>商品接入文档</h1>
        <div class="product-doc-hero-links">
          <a href="#examples" class="primary"
            >查看示例<Icon name="arrow" :size="15"
          /></a>
        </div>
      </section>
      <div class="product-doc-steps">
        <article>
          <span>01</span>
          <h2>获取接入密钥</h2>
          <p>
            在「商品库 · 商品接入」生成密钥，放入 X-Product-Api-Key 请求头。
          </p>
        </article>
        <article>
          <span>02</span>
          <h2>准备商品与图片</h2>
          <p>JSON 提交 Base64 图片，或先上传图片，再使用返回的 image_id。</p>
        </article>
        <article>
          <span>03</span>
          <h2>逐件设置有效期</h2>
          <p>为每件商品填写生效、到期时间。到期后自动停止展示。</p>
        </article>
      </div>
      <section id="examples" class="product-doc-section">
        <div class="product-doc-section-heading">
          <div>
            <h2>JSON 上传</h2>
          </div>
          <div class="product-doc-tabs">
            <button
              :class="{ active: !separateImage }"
              @click="separateImage = false"
            >
              一步提交</button
            ><button
              :class="{ active: separateImage }"
              @click="separateImage = true"
            >
              先上传图片
            </button>
          </div>
        </div>
        <div v-if="separateImage" class="product-doc-code">
          <header>
            <b>1. 上传图片</b><span>POST /api/integration/product-images</span
            ><button @click="copy(imageExample)">
              <Icon name="list" :size="14" />复制
            </button>
          </header>
          <pre>{{ imageExample }}</pre>
          <p>
            成功后返回 image_id、图片地址、格式和大小。将 image_id
            填入下方商品资料。
          </p>
        </div>
        <div class="product-doc-code">
          <header>
            <b>{{ separateImage ? "2. 提交商品" : "提交商品" }}</b><span
              >POST /api/integration/products</span
            ><button @click="copy(examples.json)">
              <Icon name="list" :size="14" />复制
            </button>
          </header>
          <pre>{{ examples.json }}</pre>
        </div>
        <div class="product-doc-facts">
          <span
            ><Icon name="shield" :size="16" />Content-Type:
            application/json</span
          ><span>PNG / JPEG / WebP ≤ 5 MiB</span><span>JSON 请求 ≤ 8 MiB</span>
        </div>
      </section>
      <section class="product-doc-section">
        <div class="product-doc-section-heading">
          <div>
            <h2>Python 标准库</h2>
          </div>
          <button class="outline" @click="copy(examples.python)">
            <Icon name="list" :size="15" />复制代码
          </button>
        </div>
        <p class="product-doc-caption">
          将图片保存为 product.jpg，并在运行环境中设置 PRODUCT_API_KEY。
        </p>
        <pre class="product-doc-python">{{ examples.python }}</pre>
      </section>
      <section class="product-doc-rules">
        <article>
          <Icon name="calendar" :size="23" />
          <div>
            <h2>每件商品，独立生效</h2>
            <p>
              逐件填写生效与到期时间，精确到秒。starts_at 和 expires_at
              使用带时区的时间，示例按 UTC 填写。
            </p>
          </div>
        </article>
        <article>
          <Icon name="coin" :size="23" />
          <div>
            <h2>金额与预计金币</h2>
            <p>
              price 以元填写，commission_percent
              为佣金百分比。服务端结合券后价、佣金比例和平台分成计算预计金币。
            </p>
          </div>
        </article>
        <article>
          <Icon name="refresh" :size="23" />
          <div>
            <h2>重复提交同一商品</h2>
            <p>
              相同 external_id
              更新同一件商品，完整上传需包含图片。省略的起止时间、平台、分类和上架状态保留原值。
            </p>
          </div>
        </article>
      </section>
      <div v-if="error" class="form-error" role="alert">{{ error }}</div>
      <div v-if="notice" class="product-doc-notice" role="status">
        <Icon name="check" :size="15" />{{ notice }}
      </div>
      <footer class="product-doc-footer">
        <span>完整字段、错误响应及兼容上传方式见 Markdown 文档。</span
        ><a href="/docs/PRODUCT-API.md" download="PRODUCT-API.md"
          ><Icon name="download" :size="15" />下载接口文档</a
        >
      </footer>
    </main>
  </div>
</template>
<style scoped>
.product-doc-page {
  min-height: 100vh;
  background: #faf7f1;
  color: #755840;
}
.product-doc-header {
  max-width: 1100px;
  margin: auto;
  padding: 27px 25px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.product-doc-header > a:last-child {
  display: flex;
  gap: 7px;
  align-items: center;
  font-size: 12px;
  color: #ac8766;
}
.product-doc-page main {
  max-width: 1050px;
  margin: auto;
  padding: 0 25px 45px;
}
.product-doc-hero {
  padding: 28px 0 32px;
}
.product-doc-hero h1 {
  font-size: 42px;
  letter-spacing: -1.5px;
  color: #68482e;
  margin: 0;
}
.product-doc-hero-links {
  display: flex;
  gap: 12px;
  margin-top: 20px;
}
.product-doc-hero-links > a {
  font-size: 12px;
  padding: 12px 17px;
}
.product-doc-steps {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 18px;
  margin-bottom: 36px;
}
.product-doc-steps article {
  border: 1px solid #eadbcb;
  border-radius: 17px;
  background: #fffcf7;
  padding: 24px;
}
.product-doc-steps article > span {
  color: #ec9c60;
  font:
    14px Georgia,
    serif;
}
.product-doc-steps h2 {
  font-size: 15px;
  margin: 15px 0 10px;
}
.product-doc-steps p {
  font-size: 11px;
  line-height: 1.9;
  color: #ae947c;
}
.product-doc-section {
  border: 1px solid #eaddce;
  border-radius: 19px;
  background: #fffdfa;
  padding: 27px;
  margin-bottom: 23px;
  scroll-margin-top: 20px;
}
.product-doc-section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 22px;
}
.product-doc-section h2 {
  font-size: 19px;
  color: #7c5638;
}
.product-doc-tabs {
  display: flex;
  background: #f8eee2;
  border-radius: 10px;
  padding: 4px;
  gap: 3px;
}
.product-doc-tabs button {
  padding: 9px 13px;
  font-size: 11px;
  border-radius: 7px;
  color: #b08d6c;
  white-space: nowrap;
}
.product-doc-tabs button.active {
  background: #fffaf3;
  color: #cb854d;
  box-shadow: 0 2px 6px #a86c2820;
}
.product-doc-code {
  border: 1px solid #eedfce;
  border-radius: 12px;
  overflow: hidden;
  margin-top: 17px;
}
.product-doc-code header {
  display: flex;
  align-items: center;
  gap: 15px;
  padding: 14px 17px;
  background: #fbf4e9;
  border-bottom: 1px solid #eedfce;
  font-size: 10px;
}
.product-doc-code header b {
  font-weight: 500;
  color: #9f7753;
}
.product-doc-code header span {
  color: #b09479;
  font-family: monospace;
  overflow-wrap: anywhere;
}
.product-doc-code header button {
  display: flex;
  gap: 5px;
  align-items: center;
  margin-left: auto;
  white-space: nowrap;
  font-size: 10px;
  color: #ba8756;
}
.product-doc-page pre {
  font:
    11px/1.9 Consolas,
    monospace;
  overflow-x: auto;
  white-space: pre;
  color: #896749;
  margin: 0;
  padding: 20px;
}
.product-doc-code > p {
  padding: 0 20px 16px;
  color: #b0967c;
  font-size: 11px;
  line-height: 1.8;
}
.product-doc-facts {
  display: flex;
  gap: 20px;
  flex-wrap: wrap;
  margin-top: 19px;
  font-size: 10px;
  color: #ad947c;
}
.product-doc-facts > span {
  display: flex;
  gap: 6px;
  align-items: center;
}
.product-doc-caption {
  font-size: 11px;
  line-height: 1.8;
  color: #ae9177;
  margin-bottom: 15px;
}
.product-doc-python {
  border: 1px solid #eee0d0;
  border-radius: 12px;
  background: #fcf7ee;
}
.product-doc-section-heading > .outline {
  font-size: 11px;
  padding: 9px 12px;
}
.product-doc-rules {
  display: grid;
  grid-template-columns: 1fr;
  gap: 18px;
  margin: 35px 0;
}
.product-doc-rules article {
  display: flex;
  gap: 17px;
  align-items: flex-start;
  padding: 0 4px;
}
.product-doc-rules svg {
  flex-shrink: 0;
  color: #d19b68;
  margin-top: 3px;
}
.product-doc-rules h2 {
  font-size: 14px;
  margin-bottom: 7px;
}
.product-doc-rules p {
  font-size: 11px;
  line-height: 1.9;
  color: #a78e78;
  max-width: 760px;
}
.product-doc-footer {
  border-top: 1px solid #e8dacb;
  padding-top: 22px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 14px;
  font-size: 11px;
  color: #b2977d;
}
.product-doc-footer a {
  display: flex;
  gap: 6px;
  align-items: center;
  white-space: nowrap;
  color: #bb8555;
}
.product-doc-notice {
  position: fixed;
  bottom: 28px;
  left: 50%;
  transform: translateX(-50%);
  padding: 11px 18px;
  border-radius: 100px;
  color: #fff3e1;
  background: #795a40;
  display: flex;
  gap: 7px;
  align-items: center;
  font-size: 11px;
}
@media (max-width: 650px) {
  .product-doc-header {
    padding: 22px 20px;
  }
  .product-doc-header .brand b {
    font-size: 17px;
  }
  .product-doc-page main {
    padding: 0 18px 30px;
  }
  .product-doc-hero {
    padding: 22px 2px;
  }
  .product-doc-hero h1 {
    font-size: 31px;
  }
  .product-doc-steps {
    grid-template-columns: 1fr;
    gap: 10px;
    margin-bottom: 23px;
  }
  .product-doc-steps article {
    padding: 19px 20px;
  }
  .product-doc-steps article > span {
    float: left;
    margin-right: 15px;
    padding-top: 2px;
  }
  .product-doc-steps h2 {
    margin: 0 0 9px;
  }
  .product-doc-steps p {
    margin-left: 32px;
  }
  .product-doc-section {
    padding: 19px 15px;
    border-radius: 15px;
  }
  .product-doc-section-heading {
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 15px;
  }
  .product-doc-section h2 {
    font-size: 17px;
  }
  .product-doc-tabs {
    width: 100%;
  }
  .product-doc-tabs button {
    flex: 1;
  }
  .product-doc-code header {
    gap: 7px;
    padding: 12px 11px;
    flex-wrap: wrap;
  }
  .product-doc-code header span {
    font-size: 9px;
  }
  .product-doc-code header b {
    width: 100%;
  }
  .product-doc-code header button {
    font-size: 9px;
  }
  .product-doc-page pre {
    padding: 15px 12px;
    font-size: 10px;
  }
  .product-doc-facts {
    gap: 10px;
    line-height: 1.7;
  }
  .product-doc-footer {
    align-items: flex-start;
    flex-direction: column;
  }
  .product-doc-rules article {
    gap: 12px;
  }
  .product-doc-rules p {
    font-size: 11px;
  }
}
</style>
