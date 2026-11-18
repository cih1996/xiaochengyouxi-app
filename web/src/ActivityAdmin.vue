<script setup>
import { ref, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function });
const emit = defineEmits(["unauthorized"]);
const campaign = ref(null),
  stats = ref(null),
  users = ref([]),
  selectedUser = ref(null),
  error = ref(""),
  notice = ref(""),
  saving = ref(false);
const controller = new AbortController();
function fail(e) {
  if (e.name === "AbortError") return;
  error.value = e.message;
  if (e.status === 401) emit("unauthorized", e);
}
async function refresh() {
  error.value = "";
  try {
    const [config, summary, list] = await Promise.all([
      props.api("/admin/campaign", { signal: controller.signal }),
      props.api("/admin/campaign/stats", { signal: controller.signal }),
      props.api("/admin/campaign/users?page=1&page_size=20", {
        signal: controller.signal,
      }),
    ]);
    campaign.value = config;
    stats.value = summary;
    users.value = list.items || [];
  } catch (e) {
    fail(e);
  }
}
async function showUser(user) {
  selectedUser.value = null;
  try {
    selectedUser.value = await props.api(`/admin/campaign/users/${user.user_id}`, {
      signal: controller.signal,
    });
  } catch (e) {
    fail(e);
  }
}
async function save() {
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    campaign.value = await props.api("/admin/campaign", {
      method: "PUT",
      body: campaign.value,
      signal: controller.signal,
    });
    notice.value = "活动设置已保存";
  } catch (e) {
    fail(e);
  } finally {
    saving.value = false;
  }
}
function addPrize() {
  campaign.value.prizes.push(10);
}
function removePrize(i) {
  campaign.value.prizes.splice(i, 1);
}
onMounted(refresh);
onUnmounted(() => controller.abort());
defineExpose({ refresh });
</script>
<template>
  <section class="admin-card activity-admin">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="notice" class="activity-notice">
      <Icon name="check" :size="15" />{{ notice }}
    </div>
    <p v-if="!campaign && !error" class="activity-loading">正在读取活动设置…</p>
    <form v-if="campaign" @submit.prevent="save">
      <header>
        <div>
          <p>机会与奖励由服务端统一确认。</p>
        </div>
        <label class="activity-switch"
          ><input
            v-model="campaign.enabled"
            type="checkbox"
            aria-label="开启活动"
          /><span></span>{{ campaign.enabled ? "已开启" : "已关闭" }}</label
        >
      </header>
      <div class="activity-fields">
        <label
          >活动标题<input
            v-model="campaign.title"
            maxlength="80"
            required /></label
        ><label
          >活动话术<textarea
            v-model="campaign.subtitle"
            maxlength="200"
            rows="2"
            required
          ></textarea>
        </label>
        <div class="activity-two">
          <label
            >到账等待（秒）<input
              v-model.number="campaign.settle_delay_seconds"
              type="number"
              min="0"
              max="3600"
              required /></label
          ><label
            >金币到账门槛<input
              v-model.number="campaign.arrival_threshold_coins"
              type="number"
              min="0"
              max="1000000"
              required /></label
          ><label
            >登录机会<input
              v-model.number="campaign.login_opportunities"
              type="number"
              min="0"
              max="1"
              required /></label
          ><label
            >视频机会/日<input
              v-model.number="campaign.video_opportunities_per_day"
              type="number"
              min="0"
              max="20"
              required /></label
          ><label
            >每位邀请机会<input
              v-model.number="campaign.invite_opportunities_per_invitee"
              type="number"
              min="0"
              max="10"
              required
          /></label>
        </div>
      </div>
      <section class="activity-tiers">
        <header>
          <div>
            <h3>奖励档位</h3>
            <span>实际结果与到账状态以服务端返回为准</span>
          </div>
          <button type="button" class="outline" @click="addPrize">
            <Icon name="gift" :size="14" />添加档位
          </button>
        </header>
        <div
          v-for="(prize, index) in campaign.prizes"
          :key="index"
          class="activity-tier"
        >
          <span class="activity-tier-index">{{ index + 1 }}</span><label
            >金币数量<input
              v-model.number="campaign.prizes[index]"
              type="number"
              min="0"
              max="100000"
              required /></label
          ><button
            type="button"
            aria-label="删除奖励档位"
            @click="removePrize(index)"
          >
            <Icon name="close" :size="15" />
          </button>
        </div>
        <p v-if="!campaign.prizes.length" class="activity-empty">
          请至少添加一个奖励档位
        </p>
      </section>
      <section class="activity-preview">
        <div>
          <span>活动预览</span>
          <h3>{{ campaign.title }}</h3>
          <p>{{ campaign.subtitle }}</p>
          <div class="activity-wheel">
            <span>✦</span><b>开始转盘</b><small>结果以服务端确认状态为准</small>
          </div>
        </div>
        <img src="/assets/reward.png" alt="奖励活动" />
      </section>
      <footer>
        <button class="primary" :disabled="saving || !campaign.prizes.length">
          <Icon name="check" :size="16" />{{
            saving ? "保存中…" : "保存活动设置"
          }}
        </button>
      </footer>
    </form>
    <section v-if="stats" class="activity-insights">
      <header>
        <div>
          <h2>活动数据</h2>
          <p>机会、转盘和奖励以服务端记录为准。</p>
        </div>
      </header>
      <div class="activity-metrics">
        <div><b>{{ stats.users }}</b><span>参与用户</span></div>
        <div><b>{{ stats.opportunities }}</b><span>活动机会</span></div>
        <div><b>{{ stats.spins }}</b><span>转盘次数</span></div>
        <div><b>{{ stats.pending }}</b><span>待结算</span></div>
        <div><b>{{ stats.settled }}</b><span>已到账</span></div>
        <div><b>{{ stats.settled_coins }}</b><span>到账金币</span></div>
      </div>
      <div class="activity-user-list">
        <div class="activity-list-title">用户活动明细</div>
        <div v-if="!users.length" class="activity-empty">暂时没有活动记录</div>
        <button
          v-for="user in users"
          :key="user.user_id"
          type="button"
          class="activity-user-row"
          @click="showUser(user)"
        >
          <span><b>{{ user.nickname || "未设置昵称" }}</b><small>{{ user.phone }}</small></span>
          <span><b>{{ user.coins }} 金币</b><small>机会 {{ user.opportunities }} · 转盘 {{ user.spins }} · 待结算 {{ user.pending }}</small></span>
          <Icon name="arrow" :size="15" />
        </button>
      </div>
      <section v-if="selectedUser" class="activity-user-detail">
        <h3>{{ selectedUser.user.nickname || "用户详情" }}</h3>
        <p>当前金币 {{ selectedUser.user.coins }} · 门槛 {{ selectedUser.settlement.arrival_threshold_coins }} · {{ selectedUser.settlement.threshold_met ? "已达标" : "未达标" }}</p>
        <div class="activity-detail-records">
          <span v-for="item in selectedUser.spins" :key="item.id">{{ item.prize_coins }} 金币 · {{ item.status === "settled" ? "已到账" : "待结算" }}</span>
        </div>
      </section>
    </section>
  </section>
</template>
<style scoped>
.activity-admin {
  max-width: 1000px;
  padding: 18px 20px;
}
.activity-insights {
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid #eee2d2;
}
.activity-insights > header { display: flex; justify-content: space-between; align-items: center; }
.activity-metrics { display: grid; grid-template-columns: repeat(6, 1fr); gap: 10px; margin: 16px 0; }
.activity-metrics div { padding: 13px 10px; border-radius: 12px; background: #fff4e5; }
.activity-metrics b, .activity-metrics span { display: block; }
.activity-metrics b { font-size: 18px; color: #874f32; }
.activity-metrics span { margin-top: 5px; font-size: 12.5px; color: #b08d6d; }
.activity-list-title { margin: 18px 0 8px; font-size: 14px; color: #785438; font-weight: 700; }
.activity-user-row { width: 100%; display: grid; grid-template-columns: 1fr 1fr 20px; gap: 12px; align-items: center; text-align: left; padding: 12px 10px; border-top: 1px solid #f0e6db; color: #79563e; }
.activity-user-row b, .activity-user-row small { display: block; }
.activity-user-row small { margin-top: 4px; color: #b59b7b; font-size: 12.5px; }
.activity-user-detail { margin-top: 16px; padding: 14px; border-radius: 12px; background: #fff8ef; }
.activity-user-detail h3 { color: #785438; }
.activity-user-detail p { margin: 7px 0; color: #a88467; font-size: 13px; }
.activity-detail-records { display: flex; gap: 7px; flex-wrap: wrap; }
.activity-detail-records span { padding: 6px 8px; border-radius: 7px; background: #f9ead9; color: #9b694b; font-size: 12.5px; }
.activity-admin > form > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding-bottom: 22px;
  border-bottom: 1px solid #eee2d2;
}
.activity-admin h2 {
  font-size: 19px;
  color: #785438;
}
.activity-admin header p {
  font-size: 12.5px;
  color: #b59b7b;
  margin-top: 8px;
}
.activity-switch {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  color: #b0865d;
  margin: 0;
}
.activity-switch input {
  position: absolute;
  opacity: 0;
  width: 1px;
  height: 1px;
  margin: 0;
  padding: 0;
}
.activity-switch span {
  width: 33px;
  height: 18px;
  border-radius: 20px;
  background: #dfd3c4;
  position: relative;
}
.activity-switch span:after {
  content: "";
  position: absolute;
  width: 14px;
  height: 14px;
  top: 2px;
  left: 2px;
  border-radius: 50%;
  background: #fff;
  transition: 0.2s;
}
.activity-switch input:checked + span {
  background: #e9935b;
}
.activity-switch input:checked + span:after {
  transform: translateX(15px);
}
.activity-fields {
  padding: 25px 0 6px;
}
.activity-fields > label {
  display: block;
}
.activity-fields input,
.activity-fields textarea {
  margin-top: 8px;
}
.activity-two {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 17px;
}
.activity-two label {
  font-size: 13px;
  font-weight: 400;
  color: #aa8b6c;
}
.activity-tiers {
  border-top: 1px solid #eee2d2;
  padding-top: 23px;
  margin-top: 13px;
}
.activity-tiers > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}
.activity-tiers h3 {
  font-size: 14px;
  color: #96704d;
}
.activity-tiers header span {
  display: block;
  font-size: 12px;
  color: #bba183;
  margin-top: 6px;
}
.activity-tiers .outline {
  font-size: 12.5px;
  padding: 9px 11px;
}
.activity-tier {
  display: grid;
  grid-template-columns: 25px 1fr 30px;
  align-items: end;
  gap: 12px;
  padding: 13px 0;
  border-top: 1px solid #f2e7da;
}
.activity-tier-index {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  background: #fff0de;
  color: #c48a59;
  font-size: 12.5px;
}
.activity-tier label {
  font-size: 12.5px;
  font-weight: 400;
  color: #ab8c6b;
  margin: 0;
}
.activity-tier input {
  padding: 10px;
  font-size: 13px;
  margin-top: 7px;
}
.activity-tier > button {
  color: #bc9a79;
  padding: 8px;
}
.activity-empty {
  text-align: center;
  color: #bba181;
  font-size: 12.5px;
  padding: 23px;
}
.activity-preview {
  margin-top: 25px;
  padding: 22px;
  border-radius: 14px;
  background: linear-gradient(125deg, #fff0dc, #ffe3be);
  display: flex;
  align-items: center;
  justify-content: space-between;
  overflow: hidden;
}
.activity-preview > div > span {
  font-size: 12px;
  color: #ba8558;
  letter-spacing: 1px;
}
.activity-preview h3 {
  color: #955b38;
  font-size: 20px;
  margin: 10px 0 7px;
}
.activity-preview p {
  font-size: 12.5px;
  color: #b3815f;
  max-width: 310px;
  line-height: 1.8;
}
.activity-preview > img {
  width: 140px;
  margin: -25px 5% -30px 0;
}
.activity-wheel {
  width: 120px;
  height: 120px;
  border-radius: 50%;
  margin-top: 14px;
  background: conic-gradient(#ffbb73, #ef7650, #ffd18f, #f48853, #ffbb73);
  border: 7px solid #fff7e9;
  box-shadow: 0 5px 15px #a85f2926;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: #fff9e9;
  text-align: center;
}
.activity-wheel span {
  font-size: 20px;
}
.activity-wheel b {
  font-size: 13px;
}
.activity-wheel small {
  font-size: 7px;
  margin-top: 5px;
  max-width: 90px;
}
.activity-admin footer {
  display: flex;
  justify-content: flex-end;
  border-top: 1px solid #eee2d2;
  margin-top: 24px;
  padding-top: 20px;
}
.activity-notice {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 11px;
  border-radius: 9px;
  background: #edf5e8;
  color: #8fa178;
  font-size: 13px;
  margin-bottom: 17px;
}
.activity-loading {
  padding: 55px;
  text-align: center;
  color: #bca27f;
  font-size: 13px;
}
@media (max-width: 650px) {
  .activity-admin {
    padding: 21px 16px;
  }
  .activity-admin > form > header {
    align-items: flex-start;
    flex-direction: column;
  }
  .activity-two {
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .activity-two label:last-child {
    grid-column: 1 / -1;
  }
  .activity-preview > img {
    width: 95px;
    margin-right: 0;
  }
  .activity-preview {
    padding: 17px;
  }
  .activity-preview h3 {
    font-size: 17px;
  }
}
</style>
