<script setup>
import { ref, onMounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function, code: String });
const phone = ref(""),
  agreed = ref(false),
  busy = ref(false),
  error = ref(""),
  result = ref(null);
// WeChat's browser blocks both app links and APK downloads: guide the user to the system browser.
const inWeChat = /MicroMessenger/i.test(navigator.userAgent);
const guide = ref(false);
const promo = ref(null);
onMounted(async () => {
  try {
    const info = await props.api("/newbie/public");
    if (info.enabled) promo.value = info;
  } catch (e) {}
});
function openApp() {
  if (inWeChat) {
    guide.value = true;
    return;
  }
  const started = Date.now();
  location.href = "xiaochengyouxi://invite?code=" + encodeURIComponent(props.code);
  // Still here after a moment: App not installed.
  setTimeout(() => {
    if (!document.hidden && Date.now() - started < 2500)
      error.value = "没有检测到小橙有喜 App，请先输入手机号收下邀请并下载";
  }, 1600);
}
function download(event) {
  if (inWeChat) {
    event.preventDefault();
    guide.value = true;
  }
}
async function prebind() {
  busy.value = true;
  error.value = "";
  try {
    result.value = await props.api("/invite/prebind", {
      method: "POST",
      body: { phone: phone.value, invite_code: props.code },
    });
  } catch (failure) {
    error.value = failure.message;
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <div class="invite-landing-page">
    <header class="invite-landing-header">
      <a href="/" class="brand"
        ><span class="brand-symbol">橙</span><b>小橙有喜</b></a
      ><span>好友邀请</span>
    </header>
    <main class="invite-landing-main">
      <section v-if="promo" class="invite-promo" @click="openApp">
        <div class="invite-promo-rays"></div>
        <div class="invite-promo-env"><span>開</span></div>
        <div class="invite-promo-copy">
          <small>好友送你一个</small>
          <strong>新人大红包</strong>
          <em>最高 {{ promo.invited_max.toLocaleString("zh-CN") }} 金币</em>
        </div>
      </section>
      <section class="invite-landing-intro">
        <h1>好友邀请</h1>
        <div class="invite-landing-art">
          <img
            src="/assets/shopping.png"
            class="landing-shopping"
            alt="精选好物"
          /><img
            src="/assets/mascot.png"
            class="landing-mascot"
            alt="小橙助手"
          /><img
            src="/assets/packet.png"
            class="landing-packet"
            alt="红包福利"
          /><span class="landing-spark">✦</span>
        </div>
      </section>
      <section class="invite-join-card">
        <span class="invite-from"
          >来自朋友的邀请 <b>{{ code }}</b></span
        ><template v-if="!result"
          ><h2>接受邀请</h2>
          <p>输入手机号，收下朋友的邀请。</p>
          <form @submit.prevent="prebind">
            <label
              >手机号<input
                v-model="phone"
                type="tel"
                inputmode="tel"
                autocomplete="tel"
                placeholder="请输入你的手机号"
                pattern="1[0-9]{10}"
                maxlength="11"
                required /></label
            ><label class="invite-agreement"
              ><input v-model="agreed" type="checkbox" required /><span
                >我已阅读并同意
                <a href="/terms" target="_blank" rel="noopener">用户协议</a> 和
                <a href="/privacy" target="_blank" rel="noopener"
                  >隐私政策</a
                ></span
              ></label
            >
            <div v-if="error" class="form-error">{{ error }}</div>
            <button class="primary full" :disabled="busy">
              {{ busy ? "正在绑定…" : "收下邀请"
              }}<Icon name="arrow" :size="17" />
            </button>
          </form>
          <span class="invite-join-note"
            >绑定后，使用同一手机号登录 App。</span
          ><button type="button" class="invite-open-app" @click="openApp">
            已安装小橙有喜？打开 App 领红包
          </button></template
        ><template v-else
          ><span class="invite-success-mark"
            ><Icon name="check" :size="32"
          /></span>
          <h2>邀请已绑定</h2>
          <p>
            下载 App，使用同一手机号登录，<br />和朋友一起开启今日的小欢喜。
          </p>
          <a class="primary full invite-download" :href="result.download_url" @click="download"
            >下载小橙有喜 App <Icon name="arrow" :size="17" /></a
          ><span class="invite-join-note"
            >邀请码 {{ result.invite_code }}</span
          ></template
        >
      </section>
    </main>
    <footer class="invite-landing-footer">小橙有喜 · 小小福利，暖暖日常</footer>
    <div v-if="inWeChat" class="wechat-tip" @click="guide = true">
      在微信里无法下载或打开 App，<b>点右上角 ··· 选择「在浏览器打开」</b>
    </div>
    <div v-if="guide" class="wechat-guide" @click="guide = false">
      <svg class="wechat-guide-arrow" viewBox="0 0 120 120" fill="none" stroke="#fff" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 110 C 40 60, 70 40, 100 18"/><path d="M78 14 L102 16 L96 40"/></svg>
      <div class="wechat-guide-text">
        <p>① 点击右上角 <b>···</b></p>
        <p>② 选择 <b>在浏览器打开</b></p>
        <p class="small">在浏览器中即可下载 App 或直接打开 App 领红包</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.invite-promo{position:relative;overflow:hidden;display:flex;align-items:center;gap:16px;margin:0 0 18px;padding:18px 20px;border-radius:22px;
  background:radial-gradient(120% 140% at 20% 30%,#ff5a45 0%,#d8141d 55%,#8c0e15 100%);color:#fff;box-shadow:0 14px 30px rgba(200,20,20,.3);cursor:pointer}
.invite-promo-rays{position:absolute;left:-60px;top:-90px;width:260px;height:260px;border-radius:50%;
  background:repeating-conic-gradient(rgba(255,214,120,.22) 0 8deg,transparent 8deg 20deg);animation:invite-spin 14s linear infinite;
  -webkit-mask:radial-gradient(circle,#000 20%,transparent 70%);mask:radial-gradient(circle,#000 20%,transparent 70%)}
@keyframes invite-spin{to{transform:rotate(360deg)}}
.invite-promo-env{position:relative;flex:none;width:62px;height:78px;border-radius:12px;background:linear-gradient(170deg,#ff7a5c,#c4141c);box-shadow:0 8px 16px rgba(60,0,0,.35);animation:invite-shake 2.2s ease-in-out infinite}
.invite-promo-env::before{content:"";position:absolute;left:0;right:0;top:0;height:42%;border-radius:12px 12px 50% 50%/12px 12px 40% 40%;background:#ff8a6a}
.invite-promo-env span{position:absolute;left:50%;top:42%;width:30px;height:30px;margin:-15px 0 0 -15px;border-radius:50%;display:grid;place-items:center;
  background:radial-gradient(circle at 35% 30%,#fff6c8,#ffc531 50%,#d98a00);color:#8a2a00;font-weight:900;font-size:16px}
@keyframes invite-shake{0%,70%,100%{transform:none}75%{transform:rotate(-8deg) scale(1.06)}80%{transform:rotate(8deg) scale(1.06)}85%{transform:rotate(-4deg)}90%{transform:rotate(3deg)}}
.invite-promo-copy{position:relative;display:flex;flex-direction:column;gap:2px}
.invite-promo-copy small{font-size:13px;opacity:.9}
.invite-promo-copy strong{font-size:26px;font-weight:900;letter-spacing:1px;background:linear-gradient(180deg,#fffbe6,#ffd75e 60%,#ffad14);-webkit-background-clip:text;background-clip:text;color:transparent}
.invite-promo-copy em{font-style:normal;font-size:14px;font-weight:800;color:#ffe7a8}
.invite-open-app{display:block;width:100%;margin-top:12px;padding:12px;border:1.5px solid #f2452b;border-radius:14px;background:#fff5f0;color:#e2321f;font-size:15px;font-weight:800}
.wechat-tip{position:fixed;left:0;right:0;top:0;z-index:20;padding:9px 14px;background:rgba(30,10,5,.88);color:#fff;font-size:13px;text-align:center}
.wechat-tip b{color:#ffd23f}
.wechat-guide{position:fixed;inset:0;z-index:30;background:rgba(0,0,0,.82);color:#fff}
.wechat-guide-arrow{position:absolute;right:22px;top:12px;width:110px;height:110px}
.wechat-guide-text{position:absolute;right:30px;left:30px;top:140px;text-align:right;font-size:20px;line-height:2}
.wechat-guide-text b{color:#ffd23f}
.wechat-guide-text .small{font-size:14px;opacity:.8;line-height:1.6;margin-top:10px}
</style>
