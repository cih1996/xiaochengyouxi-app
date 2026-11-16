<script setup>
import { computed, onMounted, ref } from "vue";
import Icon from "./Icon.vue";
import ActivityWidget from "./ActivityWidget.vue";

const props = defineProps({
  config: Object,
  campaign: Object,
  requestSpin: Function,
});
const platform = ref("desktop");
const downloadUrl = computed(
  () =>
    props.config?.app_download_url ||
    "https://orange.lingzidev.com/download/orange-rewards.apk",
);
const isIos = computed(() => platform.value === "ios");
const isAndroid = computed(() => platform.value === "android");
const platformLabel = computed(() =>
  isIos.value ? "iOS 版本暂未开放" : "下载安卓版",
);
function detectPlatform() {
  const ua = navigator.userAgent || "";
  const ipad = /Macintosh/i.test(ua) && navigator.maxTouchPoints > 1;
  platform.value = /Android/i.test(ua)
    ? "android"
    : /iPhone|iPad|iPod/i.test(ua) || ipad
      ? "ios"
      : "desktop";
}
onMounted(detectPlatform);
</script>

<template>
  <div class="landing-page">
    <header class="landing-header">
      <a class="brand" href="/">
        <span class="brand-symbol">橙</span><b>小橙有喜</b>
      </a>
      <a class="landing-admin-link" href="/admin"
        >管理入口 <Icon name="arrow" :size="15"
      /></a>
    </header>
    <main>
      <section class="landing-hero">
        <div class="landing-hero-copy">
          <span class="landing-eyebrow">小橙有喜</span>
          <h1>把每天的<br /><em>小欢喜</em>装进口袋</h1>
          <p>
            福利大厅、精选好物和金币记录，<br />在一个轻松的入口里随时查看。
          </p>
          <div class="landing-actions">
            <a
              v-if="!isIos"
              class="primary landing-download"
              :href="downloadUrl"
              ><Icon name="download" :size="18" />{{ platformLabel }}</a
            >
            <button v-else class="primary landing-download" disabled>
              {{ platformLabel }}
            </button>
            <span v-if="platform === 'desktop'" class="landing-device-note"
              >手机打开页面即可下载</span
            >
            <span v-else-if="isIos" class="landing-device-note"
              >请使用 Android 设备下载</span
            >
          </div>
        </div>
        <div class="landing-art" aria-hidden="true">
          <div class="landing-art-glow"></div>
          <img class="landing-art-mascot" src="/assets/mascot.png" alt="" />
          <img class="landing-art-reward" src="/assets/reward.png" alt="" />
          <img class="landing-art-packet" src="/assets/packet.png" alt="" />
          <span class="landing-spark spark-a">✦</span>
          <span class="landing-spark spark-b">✦</span>
        </div>
      </section>
      <section class="landing-features">
        <article>
          <span class="landing-feature-icon"
            ><Icon name="gift" :size="21"
          /></span>
          <div>
            <h2>福利大厅</h2>
            <p>每日查看可参与的红包和福利。</p>
          </div>
        </article>
        <article>
          <span class="landing-feature-icon"
            ><Icon name="bag" :size="21"
          /></span>
          <div>
            <h2>精选好物</h2>
            <p>查看价格、预计金币和商品状态。</p>
          </div>
        </article>
        <article>
          <span class="landing-feature-icon"
            ><Icon name="coin" :size="21"
          /></span>
          <div>
            <h2>金币账户</h2>
            <p>奖励、签到和提现记录清楚可见。</p>
          </div>
        </article>
      </section>
      <section class="landing-invite">
        <div>
          <span>小橙有喜</span>
          <h2>每天打开，<br />看看今天有什么收获</h2>
        </div>
        <img src="/assets/shopping.png" alt="精选好物" />
      </section>
      <ActivityWidget
        v-if="campaign?.enabled"
        :activity="campaign"
        :request-spin="requestSpin"
      />
    </main>
    <footer class="landing-footer">
      <div><b>小橙有喜</b></div>
      <nav><a href="/privacy">隐私政策</a><a href="/terms">用户协议</a></nav>
      <span>© 2026</span>
    </footer>
  </div>
</template>

<style scoped>
.landing-page {
  min-height: 100vh;
  background: #fff8f2;
  color: #4a3427;
}
.landing-header {
  height: 82px;
  max-width: 1180px;
  margin: auto;
  padding: 0 28px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.landing-header .brand b {
  font-size: 19px;
}
.landing-admin-link {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: #a77e5d;
}
.landing-page main {
  max-width: 1180px;
  margin: auto;
  padding: 0 28px 54px;
}
.landing-hero {
  min-height: 560px;
  display: grid;
  grid-template-columns: 1fr 1fr;
  align-items: center;
  gap: 20px;
}
.landing-hero-copy {
  padding: 30px 0;
}
.landing-eyebrow {
  font-size: 11px;
  letter-spacing: 3px;
  color: #c08353;
}
.landing-hero h1 {
  font-size: 56px;
  line-height: 1.18;
  letter-spacing: -2px;
  color: #643f29;
  margin: 19px 0 18px;
}
.landing-hero h1 em {
  font-style: normal;
  color: #ee633e;
}
.landing-hero-copy > p {
  font-size: 14px;
  line-height: 2;
  color: #aa8a72;
}
.landing-actions {
  display: flex;
  align-items: center;
  gap: 15px;
  flex-wrap: wrap;
  margin-top: 31px;
}
.landing-download {
  padding: 14px 22px;
  border-radius: 11px;
  font-size: 12px;
}
.landing-device-note {
  font-size: 10px;
  color: #b59a83;
}
.landing-company {
  font-size: 10px;
  color: #bca38e;
  margin-top: 25px;
}
.landing-art {
  height: 500px;
  position: relative;
  display: grid;
  place-items: center;
}
.landing-art-glow {
  width: 370px;
  height: 370px;
  position: absolute;
  border-radius: 50%;
  background: radial-gradient(circle, #ffd7a766, #fff8f200 70%);
}
.landing-art-mascot {
  width: 310px;
  position: absolute;
  z-index: 2;
  filter: drop-shadow(0 17px 15px #99572b26);
  animation: landingFloat 5s ease-in-out infinite;
}
.landing-art-reward {
  width: 182px;
  position: absolute;
  right: 5%;
  top: 16%;
  z-index: 3;
  transform: rotate(10deg);
  filter: drop-shadow(0 10px 10px #9d572a26);
  animation: landingReward 4.5s ease-in-out infinite;
}
.landing-art-packet {
  width: 142px;
  position: absolute;
  left: 5%;
  bottom: 14%;
  z-index: 3;
  transform: rotate(-14deg);
  filter: drop-shadow(0 10px 10px #9d572a26);
  animation: landingPacket 4s ease-in-out infinite;
}
.landing-spark {
  position: absolute;
  color: #ed9a4e;
  font-size: 25px;
  z-index: 4;
}
.spark-a {
  left: 19%;
  top: 19%;
}
.spark-b {
  right: 17%;
  bottom: 23%;
  font-size: 16px;
}
@keyframes landingFloat {
  50% {
    transform: translateY(-10px);
  }
}
@keyframes landingReward {
  50% {
    transform: rotate(5deg) translateY(-8px);
  }
}
@keyframes landingPacket {
  50% {
    transform: rotate(-7deg) translateY(-8px);
  }
}
.landing-features {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 17px;
}
.landing-features article {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 22px;
  border: 1px solid #eddfd1;
  border-radius: 15px;
  background: #fffdf9;
}
.landing-feature-icon {
  width: 39px;
  height: 39px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  border-radius: 12px;
  background: #fff0df;
  color: #e9965e;
}
.landing-features h2 {
  font-size: 14px;
  color: #795137;
  margin-bottom: 8px;
}
.landing-features p {
  font-size: 10px;
  color: #b0957d;
  line-height: 1.8;
}
.landing-invite {
  margin-top: 24px;
  padding: 26px 31px;
  border-radius: 18px;
  background: linear-gradient(110deg, #ffe9d1, #fff2df);
  display: flex;
  align-items: center;
  justify-content: space-between;
  overflow: hidden;
}
.landing-invite span {
  font-size: 10px;
  letter-spacing: 2px;
  color: #bb8258;
}
.landing-invite h2 {
  font-size: 24px;
  line-height: 1.45;
  color: #865337;
  margin-top: 9px;
}
.landing-invite img {
  width: 150px;
  margin: -15px 10% -15px 0;
}
.landing-footer {
  max-width: 1180px;
  margin: auto;
  padding: 23px 28px 32px;
  display: flex;
  align-items: center;
  gap: 30px;
  border-top: 1px solid #eee0d4;
  color: #b89d86;
  font-size: 10px;
}
.landing-footer > div {
  display: flex;
  gap: 11px;
  align-items: center;
  margin-right: auto;
}
.landing-footer b {
  font-size: 12px;
  color: #896247;
}
.landing-footer nav {
  display: flex;
  gap: 18px;
}
.landing-footer a:hover {
  color: #df8a55;
}
@media (max-width: 700px) {
  .landing-header {
    height: 68px;
    padding: 0 18px;
  }
  .landing-header .brand b {
    font-size: 17px;
  }
  .landing-admin-link {
    font-size: 10px;
  }
  .landing-page main {
    padding: 0 18px 34px;
  }
  .landing-hero {
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
    align-items: stretch;
  }
  .landing-hero-copy {
    padding: 42px 2px 8px;
  }
  .landing-hero h1 {
    font-size: 39px;
    letter-spacing: -1px;
    margin: 14px 0;
  }
  .landing-hero-copy > p {
    font-size: 12px;
  }
  .landing-actions {
    margin-top: 24px;
    gap: 9px;
  }
  .landing-download {
    padding: 12px 18px;
  }
  .landing-device-note {
    font-size: 9px;
  }
  .landing-company {
    margin-top: 19px;
  }
  .landing-art {
    height: 325px;
    order: -1;
    margin: 0 -4px;
  }
  .landing-art-glow {
    width: 270px;
    height: 270px;
  }
  .landing-art-mascot {
    width: 220px;
  }
  .landing-art-reward {
    width: 128px;
    right: 2%;
    top: 9%;
  }
  .landing-art-packet {
    width: 103px;
    left: 1%;
    bottom: 10%;
  }
  .landing-features {
    grid-template-columns: 1fr;
    gap: 10px;
    margin-top: 25px;
  }
  .landing-features article {
    padding: 16px;
  }
  .landing-invite {
    margin-top: 17px;
    padding: 20px;
  }
  .landing-invite h2 {
    font-size: 18px;
  }
  .landing-invite img {
    width: 108px;
    margin-right: 0;
  }
  .landing-footer {
    padding: 20px 18px 28px;
    display: block;
  }
  .landing-footer > div {
    margin: 0 0 14px;
  }
  .landing-footer nav {
    gap: 13px;
    margin-bottom: 13px;
  }
}
@media (prefers-reduced-motion: reduce) {
  .landing-art-mascot,
  .landing-art-reward,
  .landing-art-packet {
    animation: none;
  }
}
</style>
