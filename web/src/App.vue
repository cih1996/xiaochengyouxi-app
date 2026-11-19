<script setup>
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import Admin from "./Admin.vue";
import LandingPage from "./LandingPage.vue";
import ActivityWidget from "./ActivityWidget.vue";
import LegalPage from "./LegalPage.vue";
import ProductApiDocs from "./ProductApiDocs.vue";
import TestAccountsApiDocs from "./TestAccountsApiDocs.vue";
import BindInvitation from "./BindInvitation.vue";
import InviteLanding from "./InviteLanding.vue";
import InvitePoster from "./InvitePoster.vue";
import RainHall from "./RainHall.vue";
import HallNotice from "./HallNotice.vue";
import {
  prepareCoinSound,
  playCoinArrival,
  closeCoinSound,
} from "./coin-sound";
import { readHallStream } from "./hall-stream";
const tab = ref("hall"),
  route = ref(
    location.pathname.startsWith("/admin")
      ? new URLSearchParams(location.search).get("doc") === "test-accounts" &&
        sessionStorage.getItem("orange-admin-token")
        ? "test-accounts-docs"
        : "admin"
      : location.pathname === "/docs/product-api"
        ? sessionStorage.getItem("orange-admin-token")
          ? "product-docs"
          : "admin"
        : location.pathname.startsWith("/i/")
          ? "invite"
          : ["/privacy", "/terms"].includes(location.pathname)
            ? location.pathname.slice(1)
            : "app",
  ),
  config = ref(null),
  campaign = ref(null),
  adminSession = ref(Boolean(sessionStorage.getItem("orange-admin-token")));
const landingCode = location.pathname.startsWith("/i/")
  ? location.pathname.slice(3).toUpperCase()
  : "";
const token = ref(localStorage.getItem("orange-token") || ""),
  user = ref(null),
  tasks = ref([]),
  products = ref([]),
  productDisplay = ref(null),
  hallEvents = ref([]),
  ledger = ref([]),
  withdrawals = ref([]);
watch(token, (value) => { if (!value) campaign.value = null; });
const modal = ref(""),
  error = ref(""),
  busy = ref(false),
  toast = ref(""),
  category = ref("全部"),
  search = ref(""),
  selectedProduct = ref(null),
  selectedTask = ref(null);
const authMode = ref("login"),
  phone = ref(""),
  password = ref(""),
  code = ref(""),
  nickname = ref(""),
  smsSent = ref(false);
const passwordVisible = ref(false),
  welcomeSeen = ref(localStorage.getItem("welcome_seen") === "1");
const rewardStage = ref("closed"),
  rewardSession = ref(null),
  rewardQuote = ref(null),
  quoteBusy = ref(false),
  quoteExpired = ref(false),
  seconds = ref(5),
  latestReward = ref(0),
  celebrate = ref(false);
const withdrawAmount = ref(1),
  withdrawAccount = ref(""),
  withdrawChannel = ref("支付宝");
const hallStatus = ref("idle"),
  hallError = ref(""),
  appScroll = ref(null);
let timer, toastTimer, hallController, quoteController;
const coins = computed(() => user.value?.coins || 0),
  checkedIn = computed(() => Boolean(user.value?.checked_in_today)),
  filteredProducts = computed(() =>
    products.value.filter(
      (p) =>
        (category.value === "全部" || p.category === category.value) &&
        p.title.includes(search.value),
    ),
  );
const isRainLargeTask = computed(() =>
  selectedTask.value?.id?.startsWith("rain:"),
);
const appRewardsRequired = computed(
  () => config.value?.ad_mode === "csj" || config.value?.device_trust_enabled,
);
const greeting = computed(() => {
  let h = new Date().getHours();
  return h < 11 ? "早上好" : h < 14 ? "中午好" : h < 18 ? "下午好" : "晚上好";
});
const money = (n) => Number(n || 0).toFixed(2),
  number = (n) => Number(n || 0).toLocaleString("zh-CN");
const channelLabel = (channel) =>
  ({ alipay: "支付宝", wechat: "微信" })[channel] || channel;
const withdrawalStatus = (status) =>
  ({ 模拟申请: "审核中", 模拟已完成: "已完成", 模拟已退回: "已退回" })[
    status
  ] || status;
function rewardRange(task) {
  if (task?.id?.startsWith("rain:") && task.reward != null)
    return number(task.reward);
  if (task?.reward_min == null || task?.reward_max == null) return "待开启";
  return task.reward_min === task.reward_max
    ? number(task.reward_min)
    : `${number(task.reward_min)}–${number(task.reward_max)}`;
}
async function api(path, options = {}) {
  const multipart = options.body instanceof FormData;
  const res = await fetch("/api" + path, {
    ...options,
    headers: {
      ...(!multipart ? { "Content-Type": "application/json" } : {}),
      ...(token.value ? { Authorization: "Bearer " + token.value } : {}),
      ...(path.startsWith("/admin/") &&
      path !== "/admin/login" &&
      sessionStorage.getItem("orange-admin-token")
        ? { "x-admin-token": sessionStorage.getItem("orange-admin-token") }
        : {}),
      ...options.headers,
    },
    body: multipart
      ? options.body
      : options.body
        ? JSON.stringify(options.body)
        : undefined,
  });
  const raw = await res.text();
  let data;
  try {
    data = raw ? JSON.parse(raw) : {};
  } catch {
    data = { error: raw || res.statusText };
  }
  if (!res.ok) {
    const failure = new Error(data.error || res.statusText);
    failure.status = res.status;
    failure.code = data.code;
    if (data.code === "INVITE_REQUIRED") requireInvitation();
    throw failure;
  }
  return data;
}
function notify(text) {
  toast.value = text;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.value = ""), 3000);
}
function open(name) {
  error.value = "";
  modal.value = name;
}
function rememberWelcome() {
  welcomeSeen.value = true;
  localStorage.setItem("welcome_seen", "1");
}
function beginWelcome() {
  rememberWelcome();
  authMode.value = "login";
  open("auth");
}
function close() {
  if (rewardStage.value === "video") return;
  quoteController?.abort();
  modal.value = "";
  error.value = "";
}
function requireUser() {
  if (user.value) return !user.value.invitation_required;
  authMode.value = "login";
  open("auth");
  return false;
}
function requireInvitation() {
  stopHall();
  clearInterval(timer);
  rewardStage.value = "closed";
  rewardSession.value = null;
  rewardQuote.value = null;
  quoteController?.abort();
  modal.value = "";
  if (user.value) user.value = { ...user.value, invitation_required: true };
}
async function invitationBound(boundUser) {
  user.value = boundUser;
  error.value = "";
  await refresh();
}
async function refuseInvitation() {
  let failure;
  try {
    await api("/auth/logout", { method: "POST" });
  } catch (result) {
    failure = result;
  }
  token.value = "";
  localStorage.removeItem("orange-token");
  user.value = null;
  stopHall();
  authMode.value = "login";
  rememberWelcome();
  open("auth");
  if (failure) error.value = failure.message;
}
function openInvitation() {
  if (!requireUser()) return;
  open("invite");
}
function stopHall() {
  hallController?.abort();
  hallController = null;
  hallEvents.value = [];
  hallStatus.value = "idle";
  hallError.value = "";
}
function appendHallEvent(event) {
  const view = appScroll.value;
  const follow =
    tab.value === "hall" &&
    view &&
    view.scrollHeight - view.scrollTop - view.clientHeight < 80;
  hallEvents.value.push(event);
  if (follow)
    nextTick(() => {
      view.scrollTop = view.scrollHeight;
    });
}
function rainCredit(result) {
  user.value = result.user;
  if (
    result.credited !== false &&
    !result.already_claimed &&
    result.reward > 0
  ) {
    celebrate.value = true;
    setTimeout(() => (celebrate.value = false), 1600);
  }
}
function startHall() {
  stopHall();
  if (
    !token.value ||
    !user.value ||
    user.value.invitation_required ||
    route.value !== "app"
  )
    return;
  const controller = new AbortController();
  hallController = controller;
  hallStatus.value = "connecting";
  readHallStream(token.value, {
    signal: controller.signal,
    onOpen: () => {
      hallStatus.value = "connected";
    },
    onEvent: (event) => {
      if (controller.signal.aborted) return;
      appendHallEvent(event);
    },
  }).catch((failure) => {
    if (controller.signal.aborted) return;
    if (failure.code === "INVITE_REQUIRED") {
      requireInvitation();
      return;
    }
    hallStatus.value = "failed";
    hallError.value = failure.message;
    controller.abort();
  });
}
watch(
  [token, () => user.value?.id, () => user.value?.invitation_required, route],
  startHall,
);
async function refresh() {
  error.value = "";
  if (token.value) {
    try {
      user.value = await api("/me");
      if (user.value.invitation_required) return;
    } catch (e) {
      if (e.status === 401) {
        token.value = "";
        localStorage.removeItem("orange-token");
        user.value = null;
        notify("登录已过期，请重新登录");
      } else error.value = e.message;
    }
  }
  try {
    const results = await Promise.all([
      api("/tasks"),
      api("/products"),
      api("/products/runtime"),
    ]);
    tasks.value = results[0];
    const productResult = results[1];
    products.value = Array.isArray(productResult)
      ? productResult
      : productResult.products || productResult.items || [];
    productDisplay.value = results[2] || null;
    for (const product of products.value) {
      api(`/products/${product.id}/impression`, {
        method: "POST",
        body: {},
      }).catch((failure) => {
        error.value = failure.message;
      });
    }
  } catch (e) {
    error.value = e.message;
  }
}
async function authenticate() {
  busy.value = true;
  error.value = "";
  try {
    if (authMode.value === "reset") {
      await api("/auth/reset-password", {
        method: "POST",
        body: {
          phone: phone.value,
          password: password.value,
          code: code.value,
        },
      });
      token.value = "";
      localStorage.removeItem("orange-token");
      user.value = null;
      authMode.value = "login";
      rememberWelcome();
      notify("密码已更新，请登录");
    } else {
      const data = await api(
        "/auth/" + (authMode.value === "register" ? "register" : "login"),
        {
          method: "POST",
          body: {
            phone: phone.value,
            password: password.value,
            code: code.value,
            nickname: nickname.value || "小橙好友",
          },
        },
      );
      token.value = data.token;
      localStorage.setItem("orange-token", data.token);
      user.value = data.user;
      rememberWelcome();
      modal.value = "";
      await refresh();
      notify("欢迎回来，" + data.user.nickname);
    }
  } catch (e) {
    error.value = e.message;
  } finally {
    busy.value = false;
  }
}
async function sms() {
  error.value = "";
  try {
    const data = await api("/auth/sms", {
      method: "POST",
      body: {
        phone: phone.value,
        purpose: authMode.value === "reset" ? "reset" : "register",
      },
    });
    // Only the local test server returns the code; production delivers it by SMS.
    if (data.code) code.value = data.code;
    smsSent.value = true;
    notify("验证码已发送");
  } catch (e) {
    error.value = e.message;
  }
}
async function logout() {
  try {
    await api("/auth/logout", { method: "POST" });
    token.value = "";
    localStorage.removeItem("orange-token");
    user.value = null;
    authMode.value = "login";
    password.value = "";
    rememberWelcome();
    open("auth");
    notify("已退出登录");
  } catch (e) {
    error.value = e.message;
  }
}
async function selectPacket(task) {
  if (!appRewardsRequired.value && !requireUser()) return;
  selectedTask.value = task || tasks.value.find((t) => t.kind !== "checkin");
  if (!selectedTask.value) {
    notify("暂时没有可领取的红包");
    return;
  }
  rewardQuote.value = null;
  quoteExpired.value = false;
  rewardSession.value = null;
  rewardStage.value = "closed";
  open("packet");
  if (
    config.value?.ad_mode === "mock" &&
    !appRewardsRequired.value &&
    !isRainLargeTask.value
  )
    await quotePacket();
}
async function quotePacket() {
  quoteController?.abort();
  const controller = new AbortController();
  quoteController = controller;
  rewardQuote.value = null;
  quoteBusy.value = true;
  error.value = "";
  try {
    rewardQuote.value = await api("/rewards/quote", {
      method: "POST",
      body: { task_id: selectedTask.value.id },
      signal: controller.signal,
    });
    quoteExpired.value = false;
  } catch (failure) {
    if (failure.name !== "AbortError") error.value = failure.message;
  } finally {
    if (quoteController === controller) quoteBusy.value = false;
  }
}
async function playVideo() {
  if (!config.value) {
    error.value = "正在读取服务配置，请稍后再试";
    return;
  }
  if (appRewardsRequired.value || isRainLargeTask.value) {
    location.assign(
      config.value.app_download_url ||
        "https://orange.lingzidev.com/download/orange-rewards.apk",
    );
    return;
  }
  if (config.value.ad_mode !== "mock") {
    error.value = "广告服务配置不可用";
    return;
  }
  busy.value = true;
  error.value = "";
  try {
    if (
      quoteExpired.value ||
      !rewardQuote.value ||
      Date.parse(rewardQuote.value.expires_at) <= Date.now()
    ) {
      await quotePacket();
      return;
    }
    await prepareCoinSound();
    rewardSession.value = await api("/rewards/start", {
      method: "POST",
      signal: quoteController.signal,
      body: {
        task_id: selectedTask.value.id,
        quote_id: rewardQuote.value.quote_id,
      },
    });
    seconds.value = rewardSession.value.duration_seconds;
    rewardStage.value = "video";
    clearInterval(timer);
    timer = setInterval(() => {
      seconds.value--;
      if (seconds.value <= 0) {
        clearInterval(timer);
        finishVideo();
      }
    }, 1000);
  } catch (e) {
    if (e.name === "AbortError") return;
    if (e.code === "QUOTE_EXPIRED") {
      rewardQuote.value = null;
      quoteExpired.value = true;
    }
    error.value = e.message;
  } finally {
    busy.value = false;
  }
}
async function finishVideo() {
  if (config.value?.ad_mode !== "mock") {
    error.value = "请在 App 中领取广告奖励";
    return;
  }
  busy.value = true;
  error.value = "";
  try {
    const data = await api("/rewards/" + rewardSession.value.id + "/complete", {
      method: "POST",
    });
    if (data.already_completed) {
      user.value = data.user;
      rewardStage.value = "closed";
      close();
      notify("这份奖励已经到账");
      return;
    }
    latestReward.value = data.reward;
    user.value = data.user;
    rewardStage.value = "won";
    celebrate.value = true;
    if (data.reward > 0) playCoinArrival();
    setTimeout(() => (celebrate.value = false), 1600);
    tasks.value = await api("/tasks");
  } catch (e) {
    rewardStage.value = "failed";
    error.value = e.message;
  } finally {
    busy.value = false;
  }
}
async function checkIn() {
  if (!requireUser()) return;
  if (checkedIn.value) return;
  busy.value = true;
  error.value = "";
  try {
    await prepareCoinSound();
    const data = await api("/check-in", { method: "POST" });
    user.value = data.user;
    latestReward.value = data.reward;
    rewardStage.value = "won";
    open("packet");
    celebrate.value = true;
    if (data.reward > 0) playCoinArrival();
    setTimeout(() => (celebrate.value = false), 1600);
  } catch (e) {
    if (e.status === 409) {
      try {
        user.value = await api("/me");
      } catch (failure) {
        notify(failure.message);
        return;
      }
    }
    notify(e.message);
  } finally {
    busy.value = false;
  }
}
async function loadLedger() {
  if (!requireUser()) return;
  open("ledger");
  busy.value = true;
  try {
    ledger.value = await api("/ledger");
  } catch (e) {
    error.value = e.message;
  } finally {
    busy.value = false;
  }
}
async function loadWithdrawals() {
  if (!requireUser()) return;
  open("withdrawals");
  try {
    withdrawals.value = await api("/withdrawals");
  } catch (e) {
    error.value = e.message;
  }
}
function openWithdraw() {
  if (!requireUser()) return;
  withdrawAmount.value = user.value.withdraw_min_yuan || 1;
  open("withdraw");
}
async function withdraw() {
  busy.value = true;
  error.value = "";
  try {
    const data = await api("/withdrawals", {
      method: "POST",
      body: {
        amount_yuan: Number(withdrawAmount.value),
        channel: withdrawChannel.value,
        account: withdrawAccount.value,
      },
    });
    user.value = data.user;
    modal.value = "";
    notify("提现申请已提交");
  } catch (e) {
    error.value = e.message;
  } finally {
    busy.value = false;
  }
}
async function productDetail(p) {
  selectedProduct.value = null;
  open("product");
  busy.value = true;
  try {
    selectedProduct.value = await api("/products/" + p.id);
    await api(`/products/${p.id}/click`, {
      method: "POST",
      body: { kind: "detail" },
    });
  } catch (failure) {
    error.value = failure.message;
  } finally {
    busy.value = false;
  }
}
async function visitProduct() {
  busy.value = true;
  error.value = "";
  try {
    const product = await api("/products/" + selectedProduct.value.id);
    selectedProduct.value = product;
    await api(`/products/${product.id}/click`, {
      method: "POST",
      body: { kind: "outbound" },
    });
    window.location.assign(product.url);
  } catch (failure) {
    error.value = failure.message;
    if (failure.status === 404) selectedProduct.value = null;
  } finally {
    busy.value = false;
  }
}
function navigate(target) {
  route.value = target;
  history.pushState({}, "", target === "app" ? "/" : "/" + target);
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
async function initialize() {
  if (["privacy", "terms", "product-docs", "test-accounts-docs"].includes(route.value)) return;
  try {
    config.value = await api("/config");
    if (route.value === "invite") return;
    if (route.value === "app") {
      if (token.value) await loadCampaign();
      return;
    }
    if (route.value === "admin") return;
    if (config.value.environment === "local") {
      phone.value = "13800138000";
      password.value = "Orange123!";
    }
    await refresh();
  } catch (failure) {
    error.value = failure.message;
  }
}
async function loadCampaign() {
  try {
    campaign.value = await api("/campaign");
  } catch (failure) {
    if (failure.status !== 401) error.value = failure.message;
  }
}
async function spinCampaign() {
  const opportunity = campaign.value?.opportunities?.find((item) => !item.used);
  if (!opportunity)
    return { status: "unavailable", message: "今日没有可用机会" };
  const result = await api("/campaign/spin", {
    method: "POST",
    body: { opportunity_id: opportunity.id },
  });
  const delay = Math.max(0, Date.parse(result.settle_after || "") - Date.now());
  if (result.id)
    setTimeout(
      async () => {
        try {
          const settled = await api(`/campaign/rewards/${result.id}/settle`, {
            method: "POST",
          });
          if (settled.user) {
            user.value = settled.user;
            playCoinArrival();
          }
          await loadCampaign();
        } catch (failure) {
          if (failure.code !== "REWARD_PENDING") error.value = failure.message;
        }
      },
      Math.min(delay + 100, 3600000),
    );
  return result;
}
onMounted(initialize);
onUnmounted(() => {
  stopHall();
  quoteController?.abort();
  clearInterval(timer);
  clearTimeout(toastTimer);
  closeCoinSound();
});
</script>

<template>
  <LandingPage
    v-if="route === 'app'"
    :config="config"
    :campaign="campaign"
    :request-spin="spinCampaign"
  />
  <LegalPage
    v-else-if="route === 'privacy' || route === 'terms'"
    :type="route"
  />
  <ProductApiDocs v-else-if="route === 'product-docs' && adminSession" />
  <TestAccountsApiDocs v-else-if="route === 'test-accounts-docs' && adminSession" />
  <InviteLanding
    v-else-if="route === 'invite'"
    :api="api"
    :code="landingCode"
  />
  <Admin
    v-else-if="config"
    :api="api"
    :config="config"
    @back="navigate('app')"
  />
  <div v-else class="page-loading">{{ error || "正在打开管理中心…" }}</div>
</template>
