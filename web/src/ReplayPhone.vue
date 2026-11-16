<script setup>
import { computed } from "vue";
import Icon from "./Icon.vue";
import { number, label } from "./audit-format";
const props = defineProps({
  screen: String,
  event: Object,
  touches: Array,
  credit: Number,
});
const section = computed(() =>
  ["shop", "product_detail"].includes(props.screen)
    ? "shop"
    : ["mine", "withdrawal", "settings", "invite"].includes(props.screen)
      ? "mine"
      : [
            "hall",
            "packet",
            "reward_video",
            "reward_result",
            "rain",
            "check_in",
          ].includes(props.screen)
        ? "hall"
        : "other",
);
const overlay = computed(() =>
  ["packet", "reward_video", "reward_result"].includes(props.screen),
);
const active = (control) => props.event?.control_id === control;
</script>
<template>
  <div class="replay-phone">
    <div class="replay-phone-camera"></div>
    <div class="replay-phone-screen">
      <div class="replay-status"><b>小橙有喜</b><span>● ▰</span></div>
      <header>
        <b>{{ label(section === "other" ? screen : section) }}</b><Icon
          name="coin"
          :size="18"
        />
      </header>
      <main>
        <template v-if="section === 'hall'"
          ><article class="replay-hall-banner">
            <h3>福利大厅</h3>
            <img src="/assets/mascot.png" alt="" />
          </article>
          <div class="replay-check" :class="{ focused: active('check_in') }">
            <Icon name="calendar" :size="19" /><b>签到领金币</b
            ><Icon name="arrow" :size="13" />
          </div>
          <div class="replay-chat-avatar">
            <img src="/assets/mascot.png" alt="" /><span>小橙助手</span>
          </div>
          <div class="replay-packet" :class="{ focused: active('red_packet') }">
            <img src="/assets/packet.png" alt="" />
            <div><b>红包任务</b><span>点击领取</span></div>
          </div></template
        >
        <template v-else-if="section === 'shop'"
          ><article class="replay-shop-banner">
            <b>好物捡漏</b><img src="/assets/shopping.png" alt="" />
          </article>
          <div class="replay-search">
            <Icon name="search" :size="14" />发现好物
          </div>
          <div class="replay-products">
            <article
              v-for="index in 2"
              :key="index"
              :class="{ focused: active('product') }"
            >
              <div><Icon name="bag" :size="35" /></div>
              <b>商品卡片</b><span>商品详情</span>
            </article>
          </div></template
        >
        <template v-else-if="section === 'mine'"
          ><div class="replay-user">
            <img src="/assets/mascot.png" alt="" /><b>我的账号</b>
          </div>
          <div class="replay-wallet">
            <span>我的金币</span><Icon name="coin" :size="33" /><b>金币账户</b>
            <div :class="{ focused: active('withdraw') }">提现</div>
          </div>
          <div
            v-for="row in ['邀请好友', '收支明细', '提现记录', '账号设置']"
            :key="row"
            class="replay-mine-row"
          >
            <span>{{ row }}</span><Icon name="arrow" :size="12" /></div
        ></template>
        <div v-else class="replay-other-page">
          <Icon name="list" :size="30" /><b>{{
            screen === "unknown" ? "页面未记录" : label(screen)
          }}</b>
        </div>
      </main>
      <nav>
        <div
          v-for="item in [
            { id: 'hall', icon: 'hall' },
            { id: 'shop', icon: 'bag' },
            { id: 'mine', icon: 'user' },
          ]"
          :key="item.id"
          :class="{ active: section === item.id }"
        >
          <Icon :name="item.icon" :size="20" /><span>{{ label(item.id) }}</span>
        </div>
      </nav>
      <div v-if="overlay" class="replay-phone-overlay">
        <article v-if="screen === 'packet'" class="replay-phone-modal">
          <h3>红包</h3>
          <img src="/assets/packet.png" alt="" /><b>开启红包</b>
        </article>
        <article v-else-if="screen === 'reward_video'" class="replay-video">
          <Icon name="play" :size="43" /><b>激励视频</b>
        </article>
        <article v-else class="replay-phone-modal reward">
          <h3>奖励结果</h3>
          <img src="/assets/reward.png" alt="" /><b>{{
            credit > 0 ? "+" + number(credit) + " 金币" : "等待确认"
          }}</b
          ><span v-if="credit > 0">已核对到账</span>
        </article>
      </div>
      <div v-if="screen === 'background'" class="replay-background">
        <Icon name="pause" :size="30" /><span>应用在后台</span>
      </div>
      <i
        v-for="touch in touches"
        :key="touch.id"
        class="replay-touch"
        :style="{ left: touch.x / 100 + '%', top: touch.y / 100 + '%' }"
        ><b></b
      ></i>
    </div>
  </div>
</template>
<style scoped>
.replay-phone {
  width: 292px;
  background: #fffdf8;
  border: 7px solid #e9e2d8;
  border-radius: 35px;
  box-shadow: 0 15px 35px #6e4b2420;
  position: relative;
  padding: 2px;
  flex-shrink: 0;
}
.replay-phone-camera {
  position: absolute;
  top: 3px;
  left: 50%;
  transform: translateX(-50%);
  width: 78px;
  height: 13px;
  border-radius: 0 0 11px 11px;
  background: #e9e2d8;
  z-index: 5;
}
.replay-phone-screen {
  position: relative;
  border-radius: 25px;
  overflow: hidden;
  background: #fff8f1;
  height: 552px;
  display: flex;
  flex-direction: column;
}
.replay-status {
  display: flex;
  justify-content: space-between;
  font-size: 8px;
  color: #856b55;
  padding: 16px 14px 9px;
}
.replay-status b {
  font-weight: 500;
}
.replay-phone-screen > header {
  padding: 11px 15px 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: #9b7150;
}
.replay-phone-screen > header b {
  font-size: 16px;
  color: #5f452f;
}
.replay-phone-screen > main {
  padding: 0 13px;
  flex: 1;
  overflow: hidden;
}
.replay-other-page {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
  gap: 18px;
  color: #c1a889;
  font-size: 13px;
}
.replay-hall-banner {
  height: 95px;
  background: linear-gradient(110deg, #ffecd7, #ffdbb1);
  border-radius: 13px;
  display: flex;
  align-items: center;
  padding: 16px;
  position: relative;
}
.replay-hall-banner h3 {
  font-size: 16px;
  color: #9c6339;
}
.replay-hall-banner img {
  position: absolute;
  right: 0;
  bottom: -6px;
  width: 115px;
}
.replay-check {
  display: flex;
  align-items: center;
  gap: 7px;
  color: #cb9a4e;
  background: #fff2d6;
  border: 1px solid #efdcad;
  border-radius: 10px;
  padding: 10px;
  margin: 13px 0 20px;
}
.replay-check b {
  font-size: 10px;
  flex: 1;
}
.replay-chat-avatar {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 9px;
  color: #b29880;
  margin-bottom: 9px;
}
.replay-chat-avatar img {
  width: 27px;
  height: 27px;
  background: #fff0de;
  border-radius: 8px;
}
.replay-packet {
  margin-left: 35px;
  display: flex;
  align-items: center;
  gap: 6px;
  border: 1px solid #ffc183;
  border-radius: 11px;
  background: linear-gradient(120deg, #ffad65, #f78e4b);
  height: 93px;
  color: #fff6df;
  padding: 8px;
}
.replay-packet > img {
  width: 74px;
}
.replay-packet b {
  font-size: 12px;
  display: block;
}
.replay-packet span {
  font-size: 9px;
  display: block;
  margin-top: 9px;
}
.replay-phone-screen > nav {
  height: 51px;
  display: flex;
  background: #fffdf9;
  border-top: 1px solid #efe5d9;
  justify-content: space-around;
  align-items: center;
  color: #c0a78e;
}
.replay-phone-screen > nav > div {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
}
.replay-phone-screen > nav span {
  font-size: 8px;
}
.replay-phone-screen > nav .active {
  color: #ed9660;
}
.replay-shop-banner {
  height: 100px;
  background: #ffead1;
  border-radius: 12px;
  display: flex;
  align-items: center;
  padding: 15px;
  position: relative;
  color: #a16c40;
  font-size: 16px;
}
.replay-shop-banner img {
  position: absolute;
  right: 2px;
  bottom: 0;
  width: 113px;
}
.replay-search {
  margin: 13px 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px;
  border: 1px solid #efdfcc;
  border-radius: 9px;
  background: #fffdfa;
  color: #b9a087;
  font-size: 9px;
}
.replay-products {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.replay-products article {
  border: 1px solid #ede0d0;
  border-radius: 10px;
  background: #fffefb;
  overflow: hidden;
}
.replay-products article > div {
  height: 100px;
  background: #f4eadc;
  color: #ccbaa0;
  display: grid;
  place-items: center;
}
.replay-products b {
  font-size: 9px;
  display: block;
  color: #a5896b;
  margin: 12px 9px 8px;
}
.replay-products span {
  font-size: 8px;
  display: block;
  color: #c7ad8d;
  margin: 0 9px 12px;
}
.replay-user {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 9px 0 18px;
}
.replay-user img {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: #ffecd7;
}
.replay-user b {
  font-size: 14px;
  color: #8d6647;
}
.replay-wallet {
  background: linear-gradient(120deg, #ffac6f, #f38c58);
  color: #fff8e9;
  padding: 17px;
  border-radius: 14px;
}
.replay-wallet > span {
  font-size: 9px;
  display: block;
}
.replay-wallet > svg {
  margin: 12px 0 5px;
}
.replay-wallet > b {
  display: block;
  font-size: 13px;
}
.replay-wallet > div {
  text-align: center;
  background: #fff6e8;
  color: #d09059;
  border-radius: 7px;
  padding: 8px;
  margin-top: 14px;
  font-size: 10px;
}
.replay-mine-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 15px 4px;
  border-bottom: 1px solid #f2e5d7;
  font-size: 10px;
  color: #ac8c6c;
}
.focused {
  outline: 2px solid #e1ac67;
  outline-offset: 2px;
}
.replay-phone-overlay,
.replay-background {
  position: absolute;
  inset: 0;
  background: #5b382b80;
  display: flex;
  align-items: center;
  justify-content: center;
}
.replay-phone-modal {
  width: 220px;
  background: linear-gradient(155deg, #ffb276, #f57848);
  border: 1px solid #ffd6a4;
  border-radius: 18px;
  text-align: center;
  padding: 24px 18px;
  color: #fff3d5;
  box-shadow: 0 10px 25px #63341825;
}
.replay-phone-modal h3 {
  font-size: 16px;
}
.replay-phone-modal > img {
  width: 155px;
  margin: 9px auto;
}
.replay-phone-modal > b {
  display: block;
  font-size: 17px;
}
.replay-phone-modal > span {
  font-size: 9px;
  display: block;
  margin-top: 10px;
}
.replay-phone-modal.reward {
  background: #fff6dd;
  color: #b38241;
}
.replay-video {
  position: absolute;
  inset: 0;
  background: #4d4541;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 19px;
  color: #d6c6b4;
}
.replay-video b {
  font-size: 14px;
}
.replay-background {
  z-index: 3;
  background: #5d554bdd;
  flex-direction: column;
  gap: 20px;
  color: #e3d6c5;
  font-size: 12px;
}
.replay-touch {
  position: absolute;
  width: 34px;
  height: 34px;
  margin: -17px 0 0 -17px;
  pointer-events: none;
  z-index: 7;
  border: 2px solid #ef9a35;
  border-radius: 50%;
  background: #ffd38144;
  animation: replayRipple 0.85s ease-out both;
  box-shadow: 0 0 0 5px #fff1c966;
}
.replay-touch > b {
  position: absolute;
  inset: 11px;
  background: #fff8d7;
  border-radius: 50%;
}
@keyframes replayRipple {
  from {
    transform: scale(0.45);
    opacity: 1;
  }
  to {
    transform: scale(1.5);
    opacity: 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .replay-touch {
    animation: none;
    opacity: 0.8;
  }
}
</style>
