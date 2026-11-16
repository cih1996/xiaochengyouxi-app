<script setup>
import { ref } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function, code: String });
const phone = ref(""),
  agreed = ref(false),
  busy = ref(false),
  error = ref(""),
  result = ref(null);
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
          ></template
        ><template v-else
          ><span class="invite-success-mark"
            ><Icon name="check" :size="32"
          /></span>
          <h2>邀请已绑定</h2>
          <p>
            下载 App，使用同一手机号登录，<br />和朋友一起开启今日的小欢喜。
          </p>
          <a class="primary full invite-download" :href="result.download_url"
            >下载小橙有喜 App <Icon name="arrow" :size="17" /></a
          ><span class="invite-join-note"
            >邀请码 {{ result.invite_code }}</span
          ></template
        >
      </section>
    </main>
    <footer class="invite-landing-footer">小橙有喜 · 小小福利，暖暖日常</footer>
  </div>
</template>
