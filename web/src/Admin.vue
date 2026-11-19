<script setup>
import { ref, watch, onMounted } from "vue";
import Icon from "./Icon.vue";
import UserDetail from "./UserDetail.vue";
import RainAdmin from "./RainAdmin.vue";
import RewardPolicy from "./RewardPolicy.vue";
import AdminAccount from "./AdminAccount.vue";
import AppRelease from "./AppRelease.vue";
import ProductAdmin from "./ProductAdmin.vue";
import UserCenter from "./UserCenter.vue";
import InvitationGraph from "./InvitationGraph.vue";
import BehaviorAdmin from "./BehaviorAdmin.vue";
import WithdrawRulesAdmin from "./WithdrawRulesAdmin.vue";
import AdConfigAdmin from "./AdConfigAdmin.vue";
import AdStats from "./AdStats.vue";
import SmsAdmin from "./SmsAdmin.vue";
import TraceLookup from "./TraceLookup.vue";
import CosAdmin from "./CosAdmin.vue";
import NewbieAdmin from "./NewbieAdmin.vue";
import WheelAdmin from "./WheelAdmin.vue";
import ReminderAdmin from "./ReminderAdmin.vue";
import ApiTokenAdmin from "./ApiTokenAdmin.vue";
import CaptchaAdmin from "./CaptchaAdmin.vue";
import { openMailWindow } from "./mail-window";
const props = defineProps({ api: Function, config: Object });
// WeChat 收款码 for a withdrawal: re-rendered from its decoded content (no image is stored).
const qrImage = ref(""), qrCode = ref("");
async function showQr(code) {
  try {
    const m = await props.api("/admin/payout-qr", { method: "POST", body: { code } });
    const quiet = 3, n = m.size + quiet * 2;
    let cells = "";
    m.modules.forEach((row, y) => { for (let x = 0; x < row.length; x++) if (row[x] === "1") cells += `M${x + quiet} ${y + quiet}h1v1h-1z`; });
    qrImage.value = "data:image/svg+xml;utf8," + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n} ${n}" shape-rendering="crispEdges"><rect width="${n}" height="${n}" fill="#fff"/><path d="${cells}" fill="#000"/></svg>`);
    qrCode.value = code;
  } catch (e) {
    alert(e.message);
  }
}
defineEmits(["back"]);
const locked = ref(
    props.config.admin_auth_required &&
      !sessionStorage.getItem("orange-admin-token"),
  ),
  adminUsername = ref("admin"),
  adminPassword = ref(""),
  adminPasswordVisible = ref(false),
  adminAccount = ref(null),
  showAccount = ref(false),
  loginNotice = ref("");
const tab = ref("overview"),
  overview = ref({}),
  users = ref([]),
  rewards = ref([]),
  withdrawals = ref([]),
  accountDeletions = ref([]),
  settings = ref({}),
  strategy = ref({}),
  invitations = ref({ records: [] }),
  platformCode = ref(""),
  error = ref(""),
  toast = ref(""),
  busy = ref(false);
const selectedUserId = ref(null),
  graphUserId = ref(null),
  behaviorUserId = ref(null),
  userDetail = ref(null),
  userSearch = ref(""),
  userPage = ref(1),
  userTotal = ref(0),
  userLoading = ref(false);
const userPageSize = 20;
const rainAdmin = ref(null),
  rewardPolicy = ref(null),
  releasePanel = ref(null),
  productPanel = ref(null),
  userCenter = ref(null),
  graphPanel = ref(null),
  behaviorPanel = ref(null);
const withdrawRulesPanel = ref(null);
const adConfigPanel = ref(null);
const adStatsPanel = ref(null);
const smsPanel = ref(null);
const cosPanel = ref(null);
const newbiePanel = ref(null);
const wheelPanel = ref(null);
const reminderPanel = ref(null);
const apiTokenPanel = ref(null);
const mobileNav = ref(false),
  expandedGroups = ref(["people"]);
const userAdvanced = ref(false),
  userFilters = ref({
    invitation_status: "",
    account_status: "",
    from: "",
    to: "",
    min_amount: "",
    max_amount: "",
  });
const items = [
  { id: "overview", label: "数据概览", icon: "grid" },
  { id: "user-overview", label: "用户概览", icon: "grid" },
  { id: "users", label: "用户列表", icon: "user" },
  { id: "registrations", label: "注册分析", icon: "calendar" },
  { id: "activity", label: "活跃分析", icon: "spark" },
  { id: "relations", label: "邀请关系", icon: "relation" },
  { id: "behavior", label: "行为审计", icon: "list" },
  { id: "rewards", label: "奖励记录", icon: "gift" },
  { id: "withdrawals", label: "提现管理", icon: "wallet" },
  { id: "account-deletions", label: "注销审核", icon: "shield" },
  { id: "products", label: "商品库", icon: "bag" },
  { id: "strategy", label: "推送策略", icon: "bell" },
  { id: "rain", label: "红包雨", icon: "gift" },
  { id: "wheel", label: "幸运转盘", icon: "spark" },
  { id: "reminders", label: "通知提醒", icon: "bell" },
  { id: "newbie", label: "新人礼包", icon: "gift" },
  { id: "withdraw-rules", label: "提现档位", icon: "wallet" },
  { id: "ad-config", label: "广告配置", icon: "play" },
  { id: "ad-stats", label: "广告统计", icon: "list" },
  { id: "sms", label: "短信通道", icon: "mail" },
  { id: "cos", label: "COS 存储", icon: "download" },
  { id: "invitations", label: "邀请管理", icon: "send" },
  { id: "settings", label: "运营设置", icon: "settings" },
  { id: "release", label: "版本发布", icon: "download" },
  { id: "api-tokens", label: "接口令牌", icon: "shield" },
];
// Deep link: /admin?tab=ad-config opens that page directly.
const initialTab = new URLSearchParams(location.search).get("tab");
if (initialTab && items.some((i) => i.id === initialTab)) tab.value = initialTab;
const groups = [
  {
    id: "people",
    label: "用户中心",
    icon: "user",
    children: [
      "user-overview",
      "users",
      "registrations",
      "activity",
      "relations",
      "behavior",
    ],
  },
  {
    id: "operations",
    label: "运营管理",
    icon: "bag",
    children: [
      "products",
      "strategy",
      "rain",
      "newbie",
      "wheel",
      "reminders",
      "invitations",
    ],
  },
  {
    id: "funds",
    label: "资金管理",
    icon: "wallet",
    children: ["rewards", "withdrawals", "withdraw-rules", "account-deletions"],
  },
  {
    id: "ads",
    label: "广告管理",
    icon: "play",
    children: ["ad-config", "ad-stats"],
  },
  {
    id: "system",
    label: "系统设置",
    icon: "settings",
    children: ["settings", "sms", "cos", "release", "api-tokens"],
  },
];
const userCenterTabs = ["user-overview", "registrations", "activity"];
function toggleGroup(id) {
  expandedGroups.value = expandedGroups.value.includes(id)
    ? expandedGroups.value.filter((value) => value !== id)
    : [...expandedGroups.value, id];
}
const taskLabel = (id) =>
  ({
    welcome: "见面红包",
    afternoon: "午后小惊喜",
    lucky: "好运加倍",
    daily: "每日福利",
    1: "见面红包",
    2: "午后小惊喜",
    3: "好运加倍",
  })[id] || id;
const label = (id) => items.find((i) => i.id === id)?.label;
const channelLabel = (channel) =>
  ({ alipay: "支付宝", wechat: "微信" })[channel] || channel;
const money = (n) => Number(n || 0).toFixed(2),
  number = (n) => Number(n || 0).toLocaleString("zh-CN");
function notify(t) {
  toast.value = t;
  setTimeout(() => (toast.value = ""), 3000);
}
function time(s) {
  if (!s) return "—";
  return new Date(
    typeof s === "number" ? (s < 1e12 ? s * 1000 : s) : s,
  ).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
async function load() {
  if (locked.value) return;
  error.value = "";
  try {
    const results = await Promise.all([
      props.api("/admin/overview"),
      props.api("/admin/rewards"),
      props.api("/admin/withdrawals"),
      props.api("/admin/account-deletions?status=pending"),
      props.api("/admin/settings"),
      props.api("/admin/hall-strategy"),
      props.api("/admin/invitations"),
      props.api("/admin/account"),
    ]);
    if (locked.value) return;
    [
      overview.value,
      rewards.value,
      withdrawals.value,
      accountDeletions.value,
      settings.value,
      strategy.value,
      invitations.value,
      adminAccount.value,
    ] = results;
    platformCode.value = invitations.value.platform_code;
  } catch (e) {
    fail(e);
  }
}
async function loadUsers(page = 1) {
  error.value = "";
  userLoading.value = true;
  users.value = [];
  userTotal.value = 0;
  try {
    const query = new URLSearchParams({
      q: userSearch.value.trim(),
      page: String(page),
      page_size: String(userPageSize),
    });
    for (const [key, value] of Object.entries(userFilters.value))
      if (value !== "") query.set(key, String(value));
    const [result, counts] = await Promise.all([
      props.api("/admin/users/search?" + query),
      props.api("/admin/overview"),
    ]);
    if (locked.value) return;
    overview.value = counts;
    users.value = result.items;
    userTotal.value = result.total;
    userPage.value = result.page;
  } catch (failure) {
    fail(failure);
  } finally {
    userLoading.value = false;
  }
}
async function loadAccountDeletions() {
  accountDeletions.value = await props.api("/admin/account-deletions?status=pending");
}
async function reviewAccountDeletion(request, decision) {
  if (!window.confirm(decision === "approve" ? "确认通过这笔注销申请？" : "确认驳回这笔注销申请？")) return;
  busy.value = true;
  error.value = "";
  try {
    await props.api(`/admin/account-deletions/${request.id}/review`, {
      method: "POST",
      body: { decision, note: "客服审核" },
    });
    notify(decision === "approve" ? "注销申请已通过" : "注销申请已驳回");
    await loadAccountDeletions();
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
function openUser(id) {
  tab.value = "users";
  selectedUserId.value = Number(id);
}
function openRelations(id) {
  graphUserId.value = Number(id);
  tab.value = "relations";
}
function openBehavior(id) {
  behaviorUserId.value = Number(id);
  tab.value = "behavior";
}
function selectTab(id) {
  if (id === "users") selectedUserId.value = null;
  tab.value = id;
}
function resetUserSearch() {
  userSearch.value = "";
  for (const key of Object.keys(userFilters.value)) userFilters.value[key] = "";
  loadUsers(1);
}
function quickUserStatus(status) {
  userSearch.value = "";
  for (const key of Object.keys(userFilters.value)) userFilters.value[key] = "";
  userFilters.value.account_status = status;
  userAdvanced.value = true;
  loadUsers(1);
}
async function refreshCurrent() {
  if (tab.value === "rain") await rainAdmin.value.refresh();
  else if (tab.value === "products") await productPanel.value.refresh();
  else if (tab.value === "relations") await graphPanel.value.refresh();
  else if (tab.value === "behavior") await behaviorPanel.value.refresh();
  else if (tab.value === "withdraw-rules") await withdrawRulesPanel.value?.refresh();
  else if (tab.value === "ad-config") await adConfigPanel.value?.refresh();
  else if (tab.value === "ad-stats") await adStatsPanel.value?.refresh();
  else if (tab.value === "sms") await smsPanel.value?.refresh();
  else if (tab.value === "cos") await cosPanel.value?.refresh();
  else if (tab.value === "newbie") await newbiePanel.value?.refresh();
  else if (tab.value === "wheel") await wheelPanel.value?.refresh();
  else if (tab.value === "reminders") await reminderPanel.value?.refresh();
  else if (tab.value === "api-tokens") await apiTokenPanel.value?.refresh();
  else if (userCenterTabs.includes(tab.value)) await userCenter.value.refresh();
  else if (tab.value === "release") await releasePanel.value.refresh();
  else if (tab.value === "settings")
    await Promise.all([load(), rewardPolicy.value.refresh()]);
  else if (tab.value === "users") {
    if (selectedUserId.value) await userDetail.value.refresh();
    else await loadUsers(userPage.value);
  } else if (tab.value === "account-deletions") await loadAccountDeletions();
  else await load();
}
watch(tab, (value) => {
  mobileNav.value = false;
  const group = groups.find((group) => group.children.includes(value));
  if (group && !expandedGroups.value.includes(group.id))
    expandedGroups.value.push(group.id);
  if (value !== "users") selectedUserId.value = null;
  if (value === "users" && !selectedUserId.value) loadUsers(1);
  if (value === "account-deletions") loadAccountDeletions();
});
async function save() {
  busy.value = true;
  error.value = "";
  try {
    await props.api("/admin/settings", { method: "PUT", body: settings.value });
    notify("设置已保存");
    await load();
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
async function saveStrategy() {
  busy.value = true;
  error.value = "";
  try {
    await props.api("/admin/hall-strategy", {
      method: "PUT",
      body: strategy.value,
    });
    notify("推送策略已保存");
    await load();
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
async function review(w, status) {
  busy.value = true;
  error.value = "";
  try {
    await props.api("/admin/withdrawals/" + w.id, {
      method: "PUT",
      body: { status },
    });
    notify(status === "approved" ? "已完成模拟提现" : "已驳回并退回金币");
    await load();
  } catch (e) {
    fail(e);
  } finally {
    busy.value = false;
  }
}
function lockAdmin() {
  mobileNav.value = false;
  sessionStorage.removeItem("orange-admin-token");
  locked.value = true;
  overview.value = {};
  users.value = [];
  selectedUserId.value = null;
  graphUserId.value = null;
  behaviorUserId.value = null;
  userTotal.value = 0;
  userSearch.value = "";
  userPage.value = 1;
  userAdvanced.value = false;
  for (const key of Object.keys(userFilters.value)) userFilters.value[key] = "";
  tab.value = "overview";
  rewards.value = [];
  withdrawals.value = [];
  settings.value = {};
  strategy.value = {};
  invitations.value = { records: [] };
  platformCode.value = "";
  adminAccount.value = null;
  showAccount.value = false;
  adminPassword.value = "";
  adminPasswordVisible.value = false;
  loginNotice.value = "";
  toast.value = "";
}
async function logout() {
  busy.value = true;
  error.value = "";
  try {
    await props.api("/admin/logout", { method: "POST" });
    lockAdmin();
    loginNotice.value = "已退出管理中心";
  } catch (failure) {
    fail(failure);
  } finally {
    busy.value = false;
  }
}
function accountChanged(result) {
  lockAdmin();
  adminUsername.value = result.username;
  error.value = "";
  loginNotice.value = "账号已更新，请重新登录";
}
async function savePlatformCode() {
  busy.value = true;
  error.value = "";
  try {
    invitations.value = await props.api("/admin/invitations", {
      method: "PUT",
      body: { platform_code: platformCode.value.trim().toUpperCase() },
    });
    platformCode.value = invitations.value.platform_code;
    notify("平台邀请码已保存");
  } catch (failure) {
    fail(failure);
  } finally {
    busy.value = false;
  }
}
async function copyInviteLink() {
  try {
    await navigator.clipboard.writeText(invitations.value.platform_url);
    notify("邀请链接已复制");
  } catch (failure) {
    error.value = failure.message;
  }
}
function fail(failure) {
  if (failure.status === 401) lockAdmin();
  error.value = failure.message;
}
async function openMail() {
  error.value = "";
  try {
    await openMailWindow(props.api);
  } catch (failure) {
    fail(failure);
  }
}
async function login() {
  busy.value = true;
  error.value = "";
  loginNotice.value = "";
  try {
    const result = await props.api("/admin/login", {
      method: "POST",
      body: {
        username: adminUsername.value.trim(),
        password: adminPassword.value,
      },
    });
    sessionStorage.setItem("orange-admin-token", result.token);
    adminUsername.value = result.username;
    adminPassword.value = "";
    adminPasswordVisible.value = false;
    locked.value = false;
    await load();
  } catch (failure) {
    fail(failure);
  } finally {
    busy.value = false;
  }
}
onMounted(load);
</script>
<template>
  <div v-if="locked" class="admin-login-page">
    <section class="admin-login-card">
      <a class="brand" href="/" @click.prevent="$emit('back')"
        ><span class="brand-symbol">橙</span><b>小橙有喜</b></a
      ><img src="/assets/mascot.png" alt="" class="admin-login-art" />
      <h1>欢迎回到管理中心</h1>
      <form @submit.prevent="login">
        <label
          >管理账号<input
            v-model="adminUsername"
            autocomplete="username"
            placeholder="请输入管理账号"
            required
        /></label>
        <label
          >管理密码
          <div class="admin-password-field">
            <input
              v-model="adminPassword"
              :type="adminPasswordVisible ? 'text' : 'password'"
              autocomplete="current-password"
              placeholder="请输入管理密码"
              required
            /><button
              type="button"
              :aria-label="
                adminPasswordVisible ? '隐藏管理密码' : '显示管理密码'
              "
              :aria-pressed="adminPasswordVisible"
              @click="adminPasswordVisible = !adminPasswordVisible"
            >
              <Icon
                :name="adminPasswordVisible ? 'eye-off' : 'eye'"
                :size="18"
              />
            </button></div
        ></label>
        <div v-if="error" class="form-error">{{ error }}</div>
        <div v-if="loginNotice" class="admin-login-notice" role="status">
          <Icon name="check" :size="15" />{{ loginNotice }}
        </div>
        <button class="primary full" :disabled="busy">
          {{ busy ? "登录中…" : "登录管理中心"
          }}<Icon name="arrow" :size="16" />
        </button>
      </form>
      <button class="admin-login-back" @click="$emit('back')">
        <Icon name="back" :size="14" />返回小橙有喜
      </button>
    </section>
  </div>
  <div v-else class="admin-layout" :class="{ 'mobile-nav-open': mobileNav }">
    <button
      v-if="mobileNav"
      class="admin-nav-backdrop"
      aria-label="关闭导航"
      @click="mobileNav = false"
    ></button>
    <aside class="admin-sidebar">
      <a class="brand" href="/" @click.prevent="$emit('back')"
        ><span class="brand-symbol">橙</span><b>小橙有喜</b></a
      >
      <p class="admin-label">管理中心</p>
      <nav aria-label="后台导航">
        <button
          :class="{ active: tab === 'overview' }"
          @click="selectTab('overview')"
        >
          <Icon name="grid" :size="19" />数据概览
        </button>
        <section
          v-for="group in groups"
          :key="group.id"
          class="admin-nav-group"
        >
          <button
            class="admin-nav-group-toggle"
            :class="{ current: group.children.includes(tab) }"
            :aria-expanded="expandedGroups.includes(group.id)"
            @click="toggleGroup(group.id)"
          >
            <Icon :name="group.icon" :size="18" /><span>{{ group.label }}</span
            ><Icon name="chevron" :size="13" class="admin-group-chevron" />
          </button>
          <div
            v-if="expandedGroups.includes(group.id)"
            class="admin-nav-children"
          >
            <button
              v-for="id in group.children"
              :key="id"
              :class="{ active: tab === id }"
              @click="selectTab(id)"
            >
              {{ label(id) }}
            </button>
          </div>
        </section>
      </nav>
      <div class="admin-sidebar-footer">
        <span
          ><i></i>
          {{
            config.environment === "local"
              ? "本地体验"
              : config.sms_mode === "mock" || config.payment_mode === "mock"
                ? "测试版"
                : "运营中心"
          }}</span
        ><button @click="$emit('back')">
          <Icon name="back" :size="17" />返回应用预览
        </button>
        <button
          v-if="config.admin_auth_required"
          class="admin-logout"
          :disabled="busy"
          @click="logout"
        >
          <Icon name="logout" :size="17" />退出管理中心
        </button>
      </div>
    </aside>
    <main class="admin-main">
      <header class="admin-topbar">
        <button
          class="admin-mobile-menu"
          aria-label="打开导航"
          @click="mobileNav = !mobileNav"
        >
          <Icon name="list" :size="20" />
        </button>
        <span>工作台 <Icon name="arrow" :size="13" /> {{ label(tab) }}</span>
        <div class="admin-topbar-actions">
          <TraceLookup :api="api" @open-user="openUser" @unauthorized="fail" />
          <button
            v-if="adminAccount?.auth_required"
            class="admin-account-entry"
            @click="showAccount = true"
          >
            <Icon name="user" :size="16" /><span>{{
              adminAccount.username
            }}</span>
          </button>
          <button class="admin-mail-entry" @click="openMail">
            <Icon name="mail" :size="17" />邮件
          </button>
          <button class="outline" @click="refreshCurrent">
            <Icon name="refresh" :size="16" /> 刷新数据</button
          ><button
            v-if="config.admin_auth_required"
            class="admin-topbar-logout"
            :disabled="busy"
            @click="logout"
          >
            <Icon name="logout" :size="15" />退出
          </button>
        </div>
      </header>
      <div class="admin-content">
        <div v-if="!(tab === 'users' && selectedUserId)" class="admin-heading">
          <h1>{{ label(tab) }}</h1>
          <div class="admin-date">
            {{
              new Date().toLocaleDateString("zh-CN", {
                year: "numeric",
                month: "long",
                day: "numeric",
              })
            }}
          </div>
        </div>
        <div v-if="error" class="form-error">{{ error }}</div>
        <AppRelease
          v-if="tab === 'release'"
          ref="releasePanel"
          :api="api"
          @unauthorized="fail"
        />
        <ProductAdmin
          v-if="tab === 'products'"
          ref="productPanel"
          :api="api"
          @unauthorized="fail"
        />
        <UserCenter
          v-if="userCenterTabs.includes(tab)"
          ref="userCenter"
          :view="tab"
          :api="api"
          @users="tab = 'users'"
          @unauthorized="fail"
        />
        <InvitationGraph
          v-if="tab === 'relations'"
          ref="graphPanel"
          :api="api"
          :user-id="graphUserId"
          @detail="openUser"
          @behavior="openBehavior"
          @unauthorized="fail"
        />
        <BehaviorAdmin
          v-if="tab === 'behavior'"
          ref="behaviorPanel"
          :api="api"
          :user-id="behaviorUserId"
          @detail="openUser"
          @relations="openRelations"
          @unauthorized="fail"
        />
        <AdConfigAdmin
          v-if="tab === 'ad-config'"
          ref="adConfigPanel"
          :api="api"
          @unauthorized="fail"
        />
        <AdStats
          v-if="tab === 'ad-stats'"
          ref="adStatsPanel"
          :api="api"
          @unauthorized="fail"
        />
        <ApiTokenAdmin
          v-if="tab === 'api-tokens'"
          ref="apiTokenPanel"
          :api="api"
          @unauthorized="fail"
        />
        <ReminderAdmin
          v-if="tab === 'reminders'"
          ref="reminderPanel"
          :api="api"
          @unauthorized="fail"
        />
        <WheelAdmin
          v-if="tab === 'wheel'"
          ref="wheelPanel"
          :api="api"
          @unauthorized="fail"
        />
        <NewbieAdmin
          v-if="tab === 'newbie'"
          ref="newbiePanel"
          :api="api"
          @unauthorized="fail"
        />
        <CosAdmin
          v-if="tab === 'cos'"
          ref="cosPanel"
          :api="api"
          @unauthorized="fail"
        />
        <SmsAdmin
          v-if="tab === 'sms'"
          ref="smsPanel"
          :api="api"
          @unauthorized="fail"
        />
        <CaptchaAdmin v-if="tab === 'sms'" :api="api" @unauthorized="fail" />
        <WithdrawRulesAdmin
          v-if="tab === 'withdraw-rules'"
          ref="withdrawRulesPanel"
          :api="api"
          @unauthorized="fail"
        />
        <RainAdmin
          v-if="tab === 'rain'"
          ref="rainAdmin"
          :api="api"
          @unauthorized="fail"
        />
        <UserDetail
          v-if="tab === 'users' && selectedUserId"
          ref="userDetail"
          :user-id="selectedUserId"
          :api="api"
          @back="
            selectedUserId = null;
            loadUsers(userPage);
          "
          @navigate="openUser"
          @relations="openRelations"
          @behavior="openBehavior"
          @unauthorized="fail"
        />
        <template v-if="tab === 'overview'"
          ><div class="stat-grid">
            <div
              v-for="(s, i) in [
                {
                  label: '注册用户',
                  value: overview.total_users,
                  unit: '人',
                  icon: 'user',
                },
                {
                  label: '累计发放金币',
                  value: overview.coins_issued,
                  unit: '金币',
                  icon: 'coin',
                },
                {
                  label: '奖励领取次数',
                  value: overview.rewards,
                  unit: '次',
                  icon: 'gift',
                },
                {
                  label: '提现申请',
                  value: overview.withdrawal_count,
                  unit: '笔',
                  icon: 'wallet',
                },
              ]"
              class="stat-card"
              :class="{ featured: i === 1 }"
            >
              <div>
                <span>{{ s.label }}</span><Icon :name="s.icon" :size="20" />
              </div>
              <b
                >{{ number(s.value) }}<small>{{ s.unit }}</small></b
              >
              <p>
                {{ i === 1 ? "每一份奖励，都是一点小欢喜" : "实时数据" }}
              </p>
            </div>
          </div>
          <div class="admin-two-cols">
            <section class="admin-card">
              <div class="admin-card-header">
                <h2>最近的奖励</h2>
                <button @click="tab = 'rewards'">
                  查看全部 <Icon name="arrow" :size="14" />
                </button>
              </div>
              <div class="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th>用户</th>
                      <th>奖励内容</th>
                      <th>金币</th>
                      <th>时间</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="r in rewards.slice(0, 6)" :key="r.id">
                      <td>{{ r.phone }}</td>
                      <td>{{ taskLabel(r.task_id) }}</td>
                      <td class="accent">+{{ r.reward }}</td>
                      <td>{{ time(r.completed_at || r.started_at) }}</td>
                    </tr>
                    <tr v-if="!rewards.length">
                      <td colspan="4" class="empty-table">还没有奖励记录</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </section>
            <section class="admin-card warm-card">
              <span class="mini-badge">运营小贴士</span>
              <h2>一点惊喜，<br />恰好来到你身边。</h2>
              <p>安排红包与好物的出现节奏，让每一次相遇都值得期待。</p>
              <img src="/assets/mascot.png" alt="小橙助手" /><button
                class="primary"
                @click="tab = 'strategy'"
              >
                调整推送策略 <Icon name="send" :size="16" />
              </button>
            </section>
          </div>
          <section class="admin-card platform-card">
            <div class="admin-card-header">
              <h2>平台连接</h2>
              <span class="table-count">{{
                config.ad_mode === "csj"
                  ? "广告已连接，短信与提现测试中"
                  : "广告、短信与提现测试中"
              }}</span>
            </div>
            <div class="platform-grid">
              <div v-for="a in overview.ad_platforms">
                <span class="platform-dot"></span><b>{{ a.name }}</b
                ><span>{{ a.status }}</span>
              </div>
            </div>
          </section></template
        >
        <section
          v-if="
            ['users', 'rewards', 'withdrawals'].includes(tab) && !selectedUserId
          "
          class="admin-card"
        >
          <div class="admin-card-header">
            <h2>
              {{
                {
                  users: "全部用户",
                  rewards: "全部奖励",
                  withdrawals: "提现申请",
                }[tab]
              }}
            </h2>
            <span class="table-count"
              >共
              {{
                tab === "users"
                  ? userTotal
                  : { rewards, withdrawals }[tab].length
              }}
              条</span
            >
          </div>
          <div v-if="tab === 'users'" class="user-warning-summary">
            <button @click="quickUserStatus('review')">
              <span>待核查人数</span
              ><b>{{
                overview.risk_review_users == null
                  ? "—"
                  : number(overview.risk_review_users)
              }}</b
              ><Icon name="arrow" :size="13" /></button
            ><button @click="quickUserStatus('restricted')">
              <span>受限人数</span
              ><b>{{
                overview.risk_restricted_users == null
                  ? "—"
                  : number(overview.risk_restricted_users)
              }}</b
              ><Icon name="arrow" :size="13" />
            </button>
            <div>
              <span>近24小时核验失败</span
              ><b>{{
                overview.risk_assessment_errors_24h == null
                  ? "—"
                  : number(overview.risk_assessment_errors_24h)
              }}</b
              ><small>次</small>
            </div>
          </div>
          <form
            v-if="tab === 'users'"
            class="user-list-search"
            @submit.prevent="loadUsers(1)"
          >
            <div class="user-search-input">
              <Icon name="search" :size="17" /><input
                v-model="userSearch"
                aria-label="搜索用户"
                placeholder="手机号、昵称、用户编号或邀请码"
              />
            </div>
            <button class="primary" :disabled="userLoading">查询</button
            ><button
              type="button"
              class="outline"
              :class="{ 'filters-active': userAdvanced }"
              @click="userAdvanced = !userAdvanced"
            >
              <Icon name="settings" :size="14" />筛选</button
            ><button type="button" class="outline" @click="resetUserSearch">
              重置
            </button>
            <div v-if="userAdvanced" class="user-list-filters">
              <label
                >邀请关系<select
                  v-model="userFilters.invitation_status"
                  aria-label="邀请关系"
                >
                  <option value="">全部</option>
                  <option value="bound">已绑定</option>
                  <option value="unbound">待绑定</option>
                </select></label
              ><label
                >账号状态<select
                  v-model="userFilters.account_status"
                  aria-label="账号状态"
                >
                  <option value="">全部</option>
                  <option value="active">正常</option>
                  <option value="review">待核查</option>
                  <option value="restricted">受限</option>
                </select></label
              ><label
                >注册开始日期<input
                  v-model="userFilters.from"
                  type="date" /></label
              ><label
                >注册结束日期<input
                  v-model="userFilters.to"
                  type="date" /></label
              ><label
                >最低余额（元）<input
                  v-model="userFilters.min_amount"
                  type="number"
                  min="0"
                  step="0.01"
                  placeholder="不限" /></label
              ><label
                >最高余额（元）<input
                  v-model="userFilters.max_amount"
                  type="number"
                  min="0"
                  step="0.01"
                  placeholder="不限"
              /></label>
            </div>
          </form>
          <div class="table-scroll">
            <table v-if="tab === 'users'">
              <thead>
                <tr>
                  <th>用户 ID</th>
                  <th>昵称</th>
                  <th>手机号</th>
                  <th>金币余额</th>
                  <th>账号状态</th>
                  <th>风险记录</th>
                  <th>注册时间</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="u in users"
                  :key="u.id"
                  class="user-list-row"
                  tabindex="0"
                  @click="openUser(u.id)"
                  @keydown.enter="openUser(u.id)"
                >
                  <td>#{{ u.id }}</td>
                  <td>
                    <span class="table-avatar">{{
                      u.nickname?.slice(0, 1)
                    }}</span>{{ u.nickname }}
                  </td>
                  <td>{{ u.phone }}</td>
                  <td class="accent">{{ number(u.coins) }}</td>
                  <td>
                    <span
                      class="user-account-status"
                      :class="u.account_status"
                      >{{
                        {
                          active: "正常",
                          review: "待核查",
                          restricted: "受限",
                        }[u.account_status] || "未记录"
                      }}</span
                    >
                  </td>
                  <td>
                    <div v-if="u.risk_count" class="user-risk-summary">
                      <b>{{ u.risk_count }} 条记录</b
                      ><span
                        >{{
                          { observe: "观察", review: "待核查", hold: "暂缓" }[
                            u.last_risk_decision
                          ] ||
                          u.last_risk_decision ||
                          "待核验"
                        }}{{
                          u.last_risk_score == null
                            ? ""
                            : " · " + u.last_risk_score + " 分"
                        }}</span
                      ><small>{{ time(u.latest_risk_at) }}</small>
                    </div>
                    <span v-else class="user-no-risk">暂无记录</span>
                  </td>
                  <td>{{ time(u.created_at) }}</td>
                  <td>
                    <button
                      class="user-graph-open"
                      :aria-label="'查看邀请关系 ' + u.phone"
                      @click.stop="openRelations(u.id)"
                    >
                      <Icon name="relation" :size="13" />关系图
                    </button>
                    <button
                      class="user-open"
                      :aria-label="'查看用户 ' + u.phone"
                      @click.stop="openUser(u.id)"
                    >
                      查看明细 <Icon name="arrow" :size="13" />
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
            <table v-if="tab === 'rewards'">
              <thead>
                <tr>
                  <th>记录 ID</th>
                  <th>用户</th>
                  <th>任务</th>
                  <th>金币</th>
                  <th>状态</th>
                  <th>时间</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in rewards" :key="r.id">
                  <td>#{{ r.id }}</td>
                  <td>{{ r.phone }}</td>
                  <td>{{ taskLabel(r.task_id) }}</td>
                  <td class="accent">+{{ r.reward }}</td>
                  <td>
                    <span class="status-tag">{{ r.status }}</span>
                  </td>
                  <td>{{ time(r.completed_at || r.started_at) }}</td>
                </tr>
              </tbody>
            </table>
            <table v-if="tab === 'withdrawals'">
              <thead>
                <tr>
                  <th>申请 ID</th>
                  <th>用户</th>
                  <th>金额</th>
                  <th>收款方式</th>
                  <th>收款账号</th>
                  <th>状态</th>
                  <th>时间</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="w in withdrawals" :key="w.id">
                  <td>#{{ w.id }}</td>
                  <td>{{ w.phone }}</td>
                  <td class="accent">¥{{ money(w.amount_yuan) }}</td>
                  <td>{{ channelLabel(w.channel) }}</td>
                  <td>
                    {{ w.account }}
                    <button v-if="w.payout_qr" type="button" class="qr-link" @click="showQr(w.payout_qr)">查看收款码</button>
                  </td>
                  <td>
                    <span class="status-tag">{{ w.status }}</span>
                  </td>
                  <td>{{ time(w.created_at) }}</td>
                  <td>
                    <div v-if="w.status === '模拟申请'" class="review-actions">
                      <button :disabled="busy" @click="review(w, 'approved')">
                        模拟通过</button
                      ><button :disabled="busy" @click="review(w, 'rejected')">
                        驳回
                      </button>
                    </div>
                    <span v-else>—</span>
                  </td>
                </tr>
              </tbody>
            </table>
            <div
              v-if="
                !{ users, rewards, withdrawals }[tab].length && !userLoading
              "
              class="empty-state"
            >
              <Icon name="list" :size="36" />
              <p>暂无记录</p>
            </div>
          </div>
          <div v-if="tab === 'users'" class="user-list-pagination">
            <span>{{
              userLoading
                ? "正在查询…"
                : "第 " +
                  userPage +
                  " / " +
                  Math.max(1, Math.ceil(userTotal / userPageSize)) +
                  " 页"
            }}</span>
            <div>
              <button
                :disabled="userLoading || userPage === 1"
                @click="loadUsers(userPage - 1)"
              >
                上一页</button
              ><button
                :disabled="userLoading || userPage * userPageSize >= userTotal"
                @click="loadUsers(userPage + 1)"
              >
                下一页
              </button>
            </div>
          </div>
        </section>
        <section v-if="tab === 'account-deletions'" class="admin-card">
          <div class="admin-card-header">
            <span class="table-count">{{ accountDeletions.length }} 条待审核</span>
          </div>
          <div class="table-scroll">
            <table>
              <thead>
                <tr><th>申请时间</th><th>用户</th><th>手机号</th><th>申请说明</th><th>操作</th></tr>
              </thead>
              <tbody>
                <tr v-for="request in accountDeletions" :key="request.id">
                  <td>{{ time(request.created_at) }}</td>
                  <td>#{{ request.user_id }} {{ request.nickname }}</td>
                  <td>{{ request.phone }}</td>
                  <td>{{ request.reason || "用户未填写" }}</td>
                  <td>
                    <div class="review-actions">
                      <button :disabled="busy" @click="reviewAccountDeletion(request, 'approve')">通过</button>
                      <button :disabled="busy" @click="reviewAccountDeletion(request, 'reject')">驳回</button>
                    </div>
                  </td>
                </tr>
                <tr v-if="!accountDeletions.length"><td colspan="5" class="empty-table">暂无待审核申请</td></tr>
              </tbody>
            </table>
          </div>
        </section>
        <template v-if="tab === 'invitations'"
          ><section class="admin-card admin-form invitation-platform-card">
            <h2>平台邀请入口</h2>
            <p>分享邀请链接，朋友填写手机号后即可保留邀请关系。</p>
            <form
              class="platform-invite-form"
              @submit.prevent="savePlatformCode"
            >
              <label
                >平台邀请码<input
                  :value="platformCode"
                  @input="platformCode = $event.target.value.toUpperCase()"
                  class="invite-code-input"
                  pattern="[A-Z0-9]{6}"
                  minlength="6"
                  maxlength="6"
                  required /></label
              ><button class="primary" :disabled="busy">保存</button>
            </form>
            <div class="platform-invite-link">
              <span>{{ invitations.platform_url }}</span><button
                class="outline"
                @click="copyInviteLink"
              >
                复制链接
              </button>
            </div>
          </section>
          <div class="stat-grid invitation-stat-grid">
            <div
              v-for="s in [
                { label: '已邀请', value: invitations.invited_count },
                { label: '已注册', value: invitations.registered_count },
                { label: '待加入', value: invitations.pending_count },
                { label: '待绑定账号', value: invitations.unbound_count },
              ]"
              class="stat-card"
            >
              <div>{{ s.label }}</div>
              <b>{{ number(s.value) }}<small>人</small></b>
            </div>
          </div>
          <section class="admin-card">
            <div class="admin-card-header">
              <h2>邀请记录</h2>
              <span class="table-count"
                >{{ invitations.records.length }} 条</span
              >
            </div>
            <div class="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th>手机号</th>
                    <th>来源邀请码</th>
                    <th>本人邀请码</th>
                    <th>状态</th>
                    <th>登记时间</th>
                    <th>绑定时间</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="r in invitations.records" :key="r.phone">
                    <td>{{ r.phone }}</td>
                    <td>{{ r.inviter_code || "—" }}</td>
                    <td>{{ r.invite_code || "—" }}</td>
                    <td>
                      <span class="status-tag">{{ r.status }}</span>
                    </td>
                    <td>{{ time(r.created_at) }}</td>
                    <td>{{ time(r.bound_at) }}</td>
                  </tr>
                  <tr v-if="!invitations.records.length">
                    <td colspan="6" class="empty-table">暂无邀请记录</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section></template
        >
        <section
          v-if="tab === 'strategy'"
          class="admin-card admin-form settings-form"
        >
          <div class="strategy-heading">
            <div>
              <p>红包与好物，按你设定的节奏出现。</p>
            </div>
            <label class="toggle-label"
              ><input v-model="strategy.enabled" type="checkbox" /><span
                class="toggle-track"
              ></span
              >{{ strategy.enabled ? "已开启" : "已暂停" }}</label
            >
          </div>
          <form @submit.prevent="saveStrategy">
            <label
              >间隔（秒）<input
                v-model.number="strategy.interval_seconds"
                type="number"
                min="1"
                required
            /></label>
            <div class="form-two-cols">
              <label
                >红包占比<input
                  v-model.number="strategy.video_weight"
                  type="number"
                  min="0"
                  required /></label
              ><label
                >商品占比<input
                  v-model.number="strategy.product_weight"
                  type="number"
                  min="0"
                  required
              /></label>
            </div>
            <label
              >信息流占比<input
                v-model.number="strategy.native_ad_weight"
                type="number"
                min="0"
                required
            /></label>
            <label
              >欢迎语<textarea
                v-model="strategy.welcome_text"
                rows="2"
                required
                maxlength="200"
              ></textarea>
            </label>
            <label
              >红包提示<textarea
                v-model="strategy.packet_text"
                rows="2"
                required
                maxlength="200"
              ></textarea>
            </label>
            <label
              >好物提示<textarea
                v-model="strategy.product_text"
                rows="2"
                required
                maxlength="200"
              ></textarea>
            </label>
            <button class="primary" :disabled="busy">
              <Icon name="check" :size="17" />保存策略
            </button>
          </form>
        </section>
        <div v-if="tab === 'settings'" class="settings-page">
        <RewardPolicy
          ref="rewardPolicy"
          :api="api"
          @unauthorized="fail"
        />
        <div class="settings-grid">
        <section class="admin-card admin-form settings-form">
          <h2>领取与提现</h2>
          <p>每天可领取的激励红包次数和新用户的注册方式。</p>
          <form @submit.prevent="save">
            <div class="form-two-cols">
              <label
                >每日领取上限<input
                  v-model.number="settings.daily_reward_limit"
                  type="number"
                  min="1"
                  required
              /></label><label
                >新用户注册方式<select v-model="settings.register_mode">
                  <option value="phone">手机号 + 短信验证码</option>
                  <option value="username">用户名 + 密码</option>
                </select></label
              >
            </div>
            <div class="form-two-cols">
              <label
                >提现门槛
                <div class="readonly-input">在“资金管理 → 提现档位”设置</div></label
              ><label
                >金币兑换比例
                <div class="readonly-input">1,000 金币 = 1 元</div></label
              >
            </div>
            <button class="primary" :disabled="busy">
              <Icon name="check" :size="17" />保存设置
            </button>
          </form>
        </section>
        </div>
        </div>
      </div>
    </main>
    <AdminAccount
      v-if="showAccount"
      :api="api"
      @close="showAccount = false"
      @changed="accountChanged"
      @unauthorized="fail"
    />
    <div v-if="toast" class="admin-toast">
      <Icon name="check" :size="17" />{{ toast }}
    </div>
  </div>
  <div v-if="qrImage" class="qr-modal" @click="qrImage = ''">
    <div class="qr-box" @click.stop><img :src="qrImage" alt="微信收款码" /><small class="qr-text">{{ qrCode }}</small><small>用微信“扫一扫”识别后向其转账</small><button type="button" class="outline" @click="qrImage = ''">关闭</button></div>
  </div>
</template>
<style scoped>
.qr-link { margin-left: 6px; border: 0; background: #eefaf1; color: #07a35a; border-radius: 8px; padding: 2px 8px; font-size: 12px; cursor: pointer; }
.qr-modal { position: fixed; inset: 0; z-index: 80; background: rgba(0, 0, 0, 0.55); display: grid; place-items: center; }
.qr-box { background: #fff; border-radius: 16px; padding: 16px; display: grid; gap: 12px; justify-items: center; }
.qr-box img { width: min(70vw, 300px); height: auto; image-rendering: pixelated; }
.qr-box small { color: #8a6f61; font-size: 12px; }
.qr-box .qr-text { font-family: ui-monospace, monospace; word-break: break-all; max-width: 300px; text-align: center; }
.qr-box .outline { width: auto; padding: 8px 22px; margin: 0; }

.user-graph-open {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12.5px;
  color: #bb9166;
  padding: 8px 0;
  margin-right: 15px;
  white-space: nowrap;
}
.admin-sidebar nav {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  gap: 5px;
  padding-bottom: 12px;
  scrollbar-width: thin;
}
.admin-sidebar-footer {
  margin-top: 13px;
  padding-top: 17px;
}
.admin-label {
  margin-top: 29px;
}
.admin-nav-group-toggle {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 13px 16px;
  width: 100%;
  border-radius: 10px;
  color: #a18b75;
  font-size: 13.5px;
  text-align: left;
}
.admin-nav-group-toggle > span {
  flex: 1;
}
.admin-nav-group-toggle:hover {
  background: #fff4e8;
}
.admin-nav-group-toggle.current {
  color: #c38a5c;
}
.admin-group-chevron {
  color: #c3aa91;
  transition: transform 0.18s;
}
.admin-nav-group-toggle[aria-expanded="true"] .admin-group-chevron {
  transform: rotate(180deg);
}
.admin-nav-children {
  margin: 1px 0 9px 25px;
  padding-left: 16px;
  border-left: 1px solid #eddfd0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.admin-nav-children > button {
  display: block;
  width: 100%;
  text-align: left;
  padding: 10px 12px;
  border-radius: 8px;
  font-size: 13px;
  color: #b49a80;
  position: relative;
}
.admin-nav-children > button:hover {
  background: #fff5e8;
}
.admin-nav-children > button.active {
  background: #fff0df;
  color: #d59058;
  font-weight: 600;
}
.admin-nav-children > button.active:before {
  content: "";
  position: absolute;
  left: -19px;
  top: calc(50% - 3px);
  width: 5px;
  height: 5px;
  background: #e9aa71;
  border-radius: 50%;
}
.admin-mobile-menu,
.admin-nav-backdrop {
  display: none;
}
@media (max-width: 720px) {
  .admin-sidebar {
    width: 225px;
    padding: 24px 18px;
    z-index: 61;
    transform: translateX(-100%);
    transition: transform 0.2s;
    background: #fffcf8;
  }
  .mobile-nav-open .admin-sidebar {
    transform: translateX(0);
  }
  .admin-sidebar .brand {
    justify-content: flex-start;
    padding: 0 10px;
  }
  .admin-sidebar .brand b,
  .admin-sidebar .admin-label,
  .admin-sidebar-footer > span {
    display: block;
  }
  .admin-sidebar .brand b {
    font-size: 18px;
  }
  .admin-sidebar .admin-label {
    margin-top: 29px;
  }
  .admin-sidebar nav {
    margin-top: 0;
    gap: 5px;
  }
  .admin-sidebar nav button {
    font-size: 13.5px;
    gap: 12px;
    justify-content: flex-start;
    padding: 12px 16px;
  }
  .admin-sidebar nav button svg {
    width: 18px;
  }
  .admin-sidebar nav .admin-nav-children > button {
    font-size: 13px;
    padding: 10px 12px;
  }
  .admin-sidebar nav .admin-group-chevron {
    width: 13px;
  }
  .admin-sidebar-footer > button {
    font-size: 12.5px;
    justify-content: flex-start;
  }
  .admin-sidebar-footer > span {
    display: flex;
  }
  .admin-sidebar-footer svg {
    width: 17px;
  }
  .admin-sidebar-footer {
    padding-top: 17px;
  }
  .admin-main {
    margin-left: 0;
    width: 100%;
  }
  .admin-mobile-menu {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: #af8e6f;
    padding: 3px;
  }
  .admin-topbar {
    gap: 13px;
    padding: 0 15px;
  }
  .admin-topbar > span {
    display: none;
  }
  .admin-topbar .admin-topbar-actions {
    width: auto;
    flex: 1;
    min-width: 0;
    gap: 11px;
  }
  .admin-topbar .admin-account-entry {
    max-width: 95px;
  }
  .admin-nav-backdrop {
    display: block;
    position: fixed;
    inset: 0;
    background: #4e362754;
    backdrop-filter: blur(2px);
    z-index: 60;
  }
}
.admin-mail-entry {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: #b08d6d;
  font-size: 13px;
  padding: 8px 0;
  white-space: nowrap;
  flex-shrink: 0;
}
@media (max-width: 650px) {
  .admin-topbar > span {
    display: none;
  }
  .admin-topbar-actions {
    width: 100%;
    gap: 10px;
    justify-content: flex-end;
  }
  .admin-account-entry {
    margin-right: auto;
    flex: 1;
    min-width: 0;
    max-width: 120px;
  }
  .admin-topbar .outline,
  .admin-topbar-logout {
    flex-shrink: 0;
    white-space: nowrap;
  }
  .admin-topbar .outline {
    font-size: 0;
    padding: 10px;
    gap: 0;
  }
}
.settings-form > h2 {
  margin-bottom: 6px;
  font-size: 17px;
  color: #6f4b33;
}
.settings-form > p {
  margin: 0 0 22px;
  font-size: 14px;
  line-height: 1.7;
  color: #a8907f;
}
/* 运营设置: one full-width policy card, then the two small forms side by side. */
.settings-page {
  display: grid;
  gap: 12px;
}
.settings-page .reward-policy {
  max-width: none;
  margin-bottom: 0;
}
.settings-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 12px;
  align-items: stretch;
}
.settings-grid .settings-form {
  max-width: none;
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
}
.settings-grid .settings-form form {
  display: flex;
  flex-direction: column;
  flex: 1;
}
.settings-grid .settings-form .primary {
  margin-top: auto;
  align-self: flex-start;
  width: auto;
  padding-left: 24px;
  padding-right: 24px;
}
@media (max-width: 1100px) {
  .settings-grid {
    grid-template-columns: 1fr;
  }
}
.user-list-search {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  padding: 0 23px 22px;
  align-items: center;
}
.user-list-search > .user-search-input {
  display: flex;
  gap: 9px;
  align-items: center;
  max-width: 530px;
  flex: 1;
  color: #b49777;
  border: 1px solid #eadbc7;
  border-radius: 10px;
  background: #fffaf3;
  padding: 0 12px;
}
.user-list-search input {
  border: 0;
  box-shadow: none;
  background: transparent;
  padding: 12px 0;
  font-size: 13.5px;
  min-width: 0;
}
.user-list-search > .primary,
.user-list-search > .outline {
  font-size: 13px;
  border-radius: 9px;
  padding: 11px 17px;
  flex-shrink: 0;
}
.user-list-row {
  cursor: pointer;
  transition: background 0.15s;
}
.user-list-row:hover {
  background: #fff5e7;
}
.user-list-row:focus-visible {
  outline: 2px solid #edb878;
  outline-offset: -2px;
}
.user-open {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: #c59059;
  font-size: 12.5px;
  white-space: nowrap;
}
.user-list-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 23px;
  border-top: 1px solid #f1e7da;
  font-size: 12.5px;
  color: #b29a7b;
}
.user-list-pagination > div {
  display: flex;
  gap: 8px;
}
.user-list-pagination button {
  padding: 8px 12px;
  border: 1px solid #eaddca;
  border-radius: 7px;
  background: #fffaf2;
  color: #ab8d68;
  font-size: 12.5px;
}
.user-list-search .user-list-filters {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 12px;
  flex-basis: 100%;
  max-width: none;
  border: 0;
  border-top: 1px solid #f0e4d3;
  border-radius: 0;
  background: transparent;
  padding: 18px 0 0;
  margin-top: 7px;
}
.user-list-filters label {
  font-size: 12px;
  color: #b59a76;
  margin: 0;
  min-width: 0;
}
.user-list-filters input,
.user-list-filters select {
  font-size: 12.5px;
  padding: 10px 8px;
  border: 1px solid #e9d9c2;
  background: #fffdf7;
  border-radius: 8px;
  box-shadow: none;
  min-width: 0;
  min-height: 37px;
  width: 100%;
  margin-top: 6px;
}
.user-list-search .filters-active {
  background: #fff0db;
  border-color: #eecda1;
  color: #bd8d54;
}
.user-account-status {
  font-size: 12px;
  border: 1px solid #eaddc8;
  border-radius: 5px;
  padding: 4px 7px;
  white-space: nowrap;
  background: #f9f2e8;
  color: #b49a78;
}
.user-account-status.review {
  color: #b98c3e;
  background: #fff2d1;
  border-color: #ebd1a0;
}
.user-account-status.restricted {
  color: #bd7157;
  background: #fff0e6;
  border-color: #eccab6;
}
.user-risk-summary {
  display: flex;
  flex-direction: column;
  gap: 4px;
  white-space: nowrap;
}
.user-risk-summary > b {
  font-size: 12.5px;
  font-weight: 500;
  color: #b88a55;
}
.user-risk-summary > span {
  font-size: 12px;
  color: #b69a76;
}
.user-risk-summary > small {
  font-size: 11px;
  color: #c7af92;
}
.user-no-risk {
  font-size: 12.5px;
  color: #c3ab8c;
}
@media (max-width: 1200px) {
  .user-list-search .user-list-filters {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
@media (max-width: 650px) {
  .user-list-search .user-list-filters {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .user-list-filters input,
  .user-list-filters select {
    font-size: 12.5px;
    padding: 9px 7px;
  }
}
@media (max-width: 650px) {
  .user-list-search {
    padding: 0 17px 18px;
    gap: 7px;
    flex-wrap: wrap;
  }
  .user-list-search > .user-search-input {
    max-width: unset;
    flex-basis: 100%;
  }
  .user-list-search input {
    font-size: 13px;
  }
  .user-list-search > .primary,
  .user-list-search > .outline {
    padding: 9px 14px;
  }
  .user-list-pagination {
    padding: 15px 17px;
  }
  .user-list-pagination button {
    padding: 8px 9px;
  }
}
.user-warning-summary {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  border-block: 1px solid #ecdfcc;
  background: #fff8eb;
  padding: 17px 23px;
  gap: 15px;
  margin-bottom: 22px;
}
.user-warning-summary > button,
.user-warning-summary > div {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 5px 17px 5px 0;
  min-width: 0;
  text-align: left;
  border-right: 1px solid #ecdcc2;
}
.user-warning-summary > div:last-child {
  border: 0;
  padding-right: 0;
}
.user-warning-summary span {
  font-size: 12.5px;
  color: #b39770;
  line-height: 1.7;
}
.user-warning-summary b {
  font-size: 23px;
  font-weight: 500;
  color: #bd9259;
  margin-left: auto;
}
.user-warning-summary svg {
  color: #c6a477;
}
.user-warning-summary small {
  font-size: 12px;
  color: #bda582;
}
.user-warning-summary > button:hover span {
  color: #c48c4a;
}
@media (max-width: 800px) {
  .user-warning-summary {
    padding: 15px 17px;
    gap: 12px;
  }
  .user-warning-summary > button,
  .user-warning-summary > div {
    gap: 8px;
    padding-right: 12px;
    flex-wrap: wrap;
  }
  .user-warning-summary span {
    font-size: 12px;
    flex-basis: 100%;
  }
  .user-warning-summary b {
    font-size: 21px;
    margin-left: 0;
  }
  .user-warning-summary svg {
    margin-left: auto;
  }
  .user-warning-summary small {
    align-self: flex-end;
    padding-bottom: 3px;
  }
}
</style>
