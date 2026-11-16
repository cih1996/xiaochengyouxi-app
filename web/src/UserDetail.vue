<script setup>
import { ref, computed, watch, onUnmounted } from "vue";
import Icon from "./Icon.vue";
const props = defineProps({ api: Function, userId: Number });
const emit = defineEmits([
  "back",
  "navigate",
  "unauthorized",
  "relations",
  "behavior",
]);
const detail = ref(null),
  section = ref("overview"),
  error = ref(""),
  notice = ref(""),
  loading = ref(false),
  recordsLoading = ref(false),
  selectedRecord = ref(null),
  showHandling = ref(false),
  saving = ref(false);
const result = ref({ items: [], total: 0, page: 1, page_size: 20 });
const filters = ref({
    from: "",
    to: "",
    kind: "",
    status: "",
    min_amount: "",
    max_amount: "",
    related: "",
  }),
  pageSize = ref(20);
const handling = ref({ status: "active", reason: "" });
let detailController, recordsController, noticeTimer;
const sections = [
  { id: "overview", label: "资料概况", icon: "user" },
  { id: "invitations", label: "邀请关系", icon: "send" },
  { id: "ledger", label: "金币账本", icon: "coin" },
  { id: "ads", label: "广告会话", icon: "gift" },
  { id: "callbacks", label: "广告回调", icon: "check" },
  { id: "withdrawals", label: "提现记录", icon: "wallet" },
  { id: "timeline", label: "行为时间线", icon: "list" },
  { id: "risk", label: "风险证据", icon: "shield" },
  { id: "devices", label: "关联设备", icon: "grid" },
  { id: "audit", label: "管理审计", icon: "settings" },
];
const sectionTitle = computed(
  () => sections.find((item) => item.id === section.value).label,
);
const kindsBySection = {
  ledger: [
    "reward",
    "check_in",
    "rain_reward",
    "withdrawal",
    "withdrawal_refund",
    "other",
  ],
  ads: ["rewarded_video"],
  callbacks: ["csj_callback"],
  withdrawals: ["withdrawal"],
  invitations: ["invitation"],
  risk: ["login", "reward", "check_in", "withdraw"],
  devices: ["device"],
  audit: [
    "admin_access",
    "admin_review",
    "restriction_change",
    "reward_policy",
    "app_release_update",
    "app_runtime_update",
  ],
  timeline: [
    "register",
    "login",
    "logout",
    "password_reset",
    "invite_prebind",
    "invite_bind",
    "reward_start",
    "reward_complete",
    "check_in",
    "withdrawal",
    "admin_access",
    "admin_review",
    "restriction_change",
    "risk_assessment",
    "rain_join",
    "rain_small_claim",
    "rain_large_open",
  ],
};
const statusesBySection = {
  ledger: ["credited", "debited"],
  ads: ["started", "verified", "completed"],
  callbacks: ["accepted", "rejected"],
  withdrawals: ["模拟申请", "模拟已完成", "模拟已退回"],
  invitations: ["bound", "pending"],
  risk: ["pending", "reported", "assessment_error"],
  devices: ["signature_only", "tee", "strongbox"],
  audit: ["requested", "success", "failed"],
  timeline: [
    "success",
    "failed",
    "pending",
    "completed",
    "credited",
    "debited",
    "reported",
    "assessment_error",
    "bound",
  ],
};
const kindChoices = computed(() => [
  ...new Set(
    [
      ...(kindsBySection[section.value] || []),
      ...result.value.items.map((item) => item.kind),
      filters.value.kind,
    ].filter(Boolean),
  ),
]);
const statusChoices = computed(() => [
  ...new Set(
    [
      ...(statusesBySection[section.value] || []),
      ...result.value.items.map((item) => item.status),
      filters.value.status,
    ].filter(Boolean),
  ),
]);
const riskLabels = {
  debugger: "调试连接",
  emulator_hint: "模拟器线索",
  vpn: "VPN连接",
  package_mismatch: "应用包名不符",
  multiple_accounts_24h: "同设备多账号",
  frequent_events_1m: "短时事件频繁",
  root_indicators: "系统环境线索",
  account_device_changes: "账号设备频繁变化",
  registration_device_cluster: "同设备集中注册",
  registration_ip_cluster: "同IP集中注册",
  campaign_device_accounts: "同设备多账号领福利",
  campaign_shared_recipient: "多个账号共用收款标识",
  campaign_repeated_claims: "短时重复领取",
  new_account_campaign_cluster: "新账号集中参与",
  payment_attempt_velocity: "短时支付频繁",
  payment_method_rotation: "频繁更换支付方式",
  payment_shared_method: "多个账号共用支付方式",
  payment_failure_history: "支付失败记录",
  payment_chargeback_history: "支付拒付记录",
  client: "设备信号",
  business: "业务关联",
  outcome: "结果反馈",
  campaignId: "活动编号",
  recipientId: "收款标识",
  paymentMethodId: "支付方式标识",
  orderId: "订单编号",
  accountCreatedAt: "账号创建时间",
  signature_only: "持钥签名",
  tee: "TEE证明",
  strongbox: "StrongBox证明",
};
const riskName = (value) => riskLabels[value] || value || "未提供";
const tierName = (value) =>
  ({
    new_device: "新设备档",
    active_device: "活跃设备档",
    mature_device: "成熟设备档",
    unknown_device: "未知设备保守档",
    fixed: "固定奖励",
  })[value] ||
  value ||
  "历史记录未留存";
const identityName = (value) =>
  ({
    stable_device_id: "平台验签关联",
    verified_key: "已验签设备密钥",
    unknown: "未确认设备",
    server_verified_device_history: "服务端验签记录",
    unverified_current_device: "本次设备未确认",
  })[value] ||
  value ||
  "未留存";
const money = (value) => (value == null ? "—" : Number(value).toFixed(2)),
  number = (value) =>
    value == null ? "—" : Number(value).toLocaleString("zh-CN");
const statusName = (value) =>
  ({
    credited: "已入账",
    debited: "已扣减",
    started: "已开始",
    accepted: "已接受",
    reported: "已核验",
    assessment_error: "核验失败",
    requested: "已发起",
    observe: "观察",
    verify: "待核查",
    hold: "暂缓",
    risk_assessment: "风险评估",
    rain_reward: "红包雨奖励",
    rain_join: "参加红包雨",
    rain_small_claim: "领取雨中红包",
    rain_large_open: "打开大奖红包",
    device_trust: "设备核验",
    withdraw: "提现",
    recorded: "已记录",
    signature_only: "持钥签名",
    tee: "TEE证明",
    strongbox: "StrongBox证明",
    withdrawal_refund: "提现退回",
    other: "其他",
    csj_callback: "广告回调",
    invitation: "邀请",
    device: "设备记录",
    admin_access: "查看记录",
    admin_review: "审核提现",
    restriction_change: "账户处理",
    reward_policy: "阶梯奖励配置",
    app_release_update: "版本发布配置",
    app_runtime_update: "大厅公告配置",
    password_reset: "重置密码",
    invite_prebind: "手机号邀请登记",
    reward_start: "发起领奖",
    reward_complete: "完成领奖",
    welcome: "见面红包",
    afternoon: "午后小惊喜",
    lucky: "好运加倍",
    active: "正常",
    review: "待核查",
    restricted: "受限",
    pending: section.value === "risk" ? "待核验" : "待处理",
    completed: "已完成",
    verified: "已确认",
    verified_state: "已有确认记录",
    rejected: "已拒绝",
    approved: "已通过",
    success: "成功",
    failed: "失败",
    bound: "已绑定",
    unbound: "待绑定",
    allow: "通过",
    deny: "拒绝",
    block: "拦截",
    mock: "测试",
    csj: "穿山甲",
    captured: "实际接收",
    business: "业务回溯",
    upstream: "上级邀请",
    downstream: "已邀请",
    withdraw_refund: "提现退回",
    check_in: "每日签到",
    checkin: "每日签到",
    reward: "红包奖励",
    rewarded_video: "激励视频",
    withdrawal: "提现申请",
    register: "注册",
    login: "登录",
    logout: "退出登录",
    reset_password: "重置密码",
    invite_bind: "绑定邀请",
    alipay: "支付宝",
    wechat: "微信",
  })[value] ||
  value ||
  "未记录";
function time(value) {
  if (!value) return "未记录";
  return new Date(
    typeof value === "number" && value < 1e12 ? value * 1000 : value,
  ).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}
function short(value) {
  if (!value) return "—";
  const text = String(value);
  return text.length > 24 ? text.slice(0, 10) + "…" + text.slice(-8) : text;
}
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function copy(value) {
  try {
    await navigator.clipboard.writeText(String(value));
    notice.value = "完整编号已复制";
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice.value = ""), 2400);
  } catch (failure) {
    fail(failure);
  }
}
function resetFilters() {
  for (const key of Object.keys(filters.value)) filters.value[key] = "";
  if (section.value !== "overview") loadRecords(1);
}
async function loadRecords(page = 1) {
  recordsController?.abort();
  const controller = new AbortController();
  recordsController = controller;
  recordsLoading.value = true;
  error.value = "";
  selectedRecord.value = null;
  result.value = { items: [], total: 0, page, page_size: pageSize.value };
  try {
    const query = new URLSearchParams({
      section: section.value,
      page: String(page),
      page_size: String(pageSize.value),
    });
    for (const [key, value] of Object.entries(filters.value))
      if (value !== "") query.set(key, String(value));
    result.value = await props.api(
      `/admin/users/${props.userId}/records?${query}`,
      { signal: controller.signal },
    );
  } catch (failure) {
    fail(failure);
  } finally {
    if (!controller.signal.aborted) recordsLoading.value = false;
  }
}
async function loadDetail() {
  detailController?.abort();
  recordsController?.abort();
  const controller = new AbortController();
  detailController = controller;
  loading.value = true;
  error.value = "";
  detail.value = null;
  try {
    detail.value = await props.api(`/admin/users/${props.userId}/detail`, {
      signal: controller.signal,
    });
    handling.value = { status: detail.value.restriction.status, reason: "" };
    if (section.value !== "overview") await loadRecords(result.value.page);
  } catch (failure) {
    fail(failure);
  } finally {
    if (!controller.signal.aborted) loading.value = false;
  }
}
async function saveHandling() {
  saving.value = true;
  error.value = "";
  try {
    await props.api(`/admin/users/${props.userId}/restriction`, {
      method: "PUT",
      body: handling.value,
    });
    showHandling.value = false;
    await loadDetail();
    notice.value = "账户处理已记录";
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice.value = ""), 2600);
  } catch (failure) {
    fail(failure);
  } finally {
    saving.value = false;
  }
}
watch(
  () => props.userId,
  () => {
    section.value = "overview";
    for (const key of Object.keys(filters.value)) filters.value[key] = "";
    result.value = { items: [], total: 0, page: 1, page_size: 20 };
    selectedRecord.value = null;
    showHandling.value = false;
    loadDetail();
  },
  { immediate: true },
);
watch(section, () => {
  for (const key of Object.keys(filters.value)) filters.value[key] = "";
  selectedRecord.value = null;
  if (section.value !== "overview" && detail.value) loadRecords(1);
});
onUnmounted(() => {
  detailController?.abort();
  recordsController?.abort();
  clearTimeout(noticeTimer);
});
defineExpose({ refresh: loadDetail });
</script>

<template>
  <div class="ud-workspace">
    <div class="ud-breadcrumb">
      <button @click="emit('back')">
        <Icon name="back" :size="16" />用户列表</button
      ><span>/</span><b>用户明细 #{{ userId }}</b>
    </div>
    <div v-if="error" class="form-error ud-error" role="alert">{{ error }}</div>
    <div v-if="loading" class="ud-loading">
      <Icon name="refresh" :size="22" /><span>正在读取用户资料…</span>
    </div>
    <template v-if="detail && !loading">
      <section class="ud-profile">
        <div class="ud-identity">
          <span class="ud-avatar">{{ detail.user.nickname.slice(0, 1) }}</span>
          <div>
            <div class="ud-name">
              <h1>{{ detail.user.nickname }}</h1>
              <span class="ud-status" :class="detail.restriction.status">{{
                statusName(detail.restriction.status)
              }}</span><span class="ud-status subtle">{{
                detail.user.invitation_required ? "待绑定邀请" : "邀请已绑定"
              }}</span>
            </div>
            <div class="ud-identity-meta">
              <button @click="copy(detail.user.phone)">
                {{ detail.user.phone }}</button><span
                >用户 #{{ detail.user.id }}</span
              ><span>注册 {{ time(detail.user.created_at) }}</span>
            </div>
          </div>
        </div>
        <div class="ud-profile-actions">
          <button @click="emit('relations', userId)">
            <Icon name="relation" :size="15" />关系图</button
          ><button @click="emit('behavior', userId)">
            <Icon name="list" :size="15" />行为画像</button
          ><button
            class="ud-handle-button"
            @click="showHandling = !showHandling"
          >
            <Icon name="shield" :size="16" />账户处理<Icon
              name="chevron"
              :size="13"
            />
          </button>
        </div>
      </section>
      <form
        v-if="showHandling"
        class="ud-handling"
        @submit.prevent="saveHandling"
      >
        <label
          >账户状态<select v-model="handling.status" aria-label="账户状态">
            <option value="active">正常</option>
            <option value="review">待核查</option>
            <option value="restricted">受限</option>
          </select></label
        ><label class="ud-handling-reason"
          >处理原因<input
            v-model="handling.reason"
            placeholder="填写本次处理的原因"
            required
            maxlength="500" /></label
        ><button class="primary" :disabled="saving">
          {{ saving ? "正在提交…" : "保存处理" }}
        </button>
      </form>
      <div class="ud-stats">
        <div class="ud-stat primary">
          <span>金币余额<Icon name="coin" :size="18" /></span
          ><b>{{ number(detail.user.coins) }}<small>金币</small></b>
          <p>折合 ¥{{ money(detail.user.cash_yuan) }}</p>
        </div>
        <div class="ud-stat">
          <span>累计获得<Icon name="gift" :size="18" /></span
          ><b>{{ number(detail.user.total_earned) }}<small>金币</small></b>
          <p>今日 +{{ number(detail.user.today_earned) }}</p>
        </div>
        <div class="ud-stat">
          <span>广告会话<Icon name="list" :size="18" /></span
          ><b>{{ number(detail.summary.ad_sessions) }}<small>次</small></b>
          <p>
            已确认 {{ number(detail.summary.ad_verified) }} · 已入账
            {{ number(detail.summary.ad_completed) }}
          </p>
        </div>
        <div class="ud-stat">
          <span>累计提现申请<Icon name="wallet" :size="18" /></span
          ><b
            ><small>¥</small
            >{{ money(detail.summary.withdrawal_total_yuan) }}</b
          >
          <p>待处理 ¥{{ money(detail.summary.withdrawal_pending_yuan) }}</p>
        </div>
      </div>
      <nav class="ud-tabs" aria-label="用户明细分类">
        <button
          v-for="item in sections"
          :key="item.id"
          :class="{ active: section === item.id }"
          @click="section = item.id"
        >
          <Icon :name="item.icon" :size="16" />{{ item.label }}
        </button>
      </nav>
      <template v-if="section === 'overview'">
        <div class="ud-overview-grid">
          <section class="ud-card">
            <div class="ud-card-title">
              <h2>账号资料</h2>
              <span>#{{ detail.user.id }}</span>
            </div>
            <dl class="ud-facts">
              <div>
                <dt>手机号</dt>
                <dd>{{ detail.user.phone }}</dd>
              </div>
              <div>
                <dt>昵称</dt>
                <dd>{{ detail.user.nickname }}</dd>
              </div>
              <div>
                <dt>注册时间</dt>
                <dd>{{ time(detail.user.created_at) }}</dd>
              </div>
              <div>
                <dt>最近登录</dt>
                <dd>{{ time(detail.user.last_login_at) }}</dd>
              </div>
              <div>
                <dt>今日签到</dt>
                <dd>
                  {{ detail.user.checked_in_today ? "已签到" : "未签到" }}
                </dd>
              </div>
              <div>
                <dt>本人邀请码</dt>
                <dd>
                  <button class="ud-id" @click="copy(detail.user.invite_code)">
                    {{ detail.user.invite_code }}<small>复制</small>
                  </button>
                </dd>
              </div>
              <div>
                <dt>上级邀请人</dt>
                <dd>
                  <button
                    v-if="detail.inviter?.id"
                    class="ud-account-link"
                    @click="emit('navigate', detail.inviter.id)"
                  >
                    {{ detail.inviter.nickname }} · {{ detail.inviter.phone
                    }}<Icon name="arrow" :size="12" /></button
                  ><span v-else>{{
                    detail.inviter?.nickname ||
                    (detail.user.invitation_required ? "尚未绑定" : "平台邀请")
                  }}</span>
                </dd>
              </div>
              <div>
                <dt>来源邀请码</dt>
                <dd>{{ detail.user.inviter_code || "—" }}</dd>
              </div>
            </dl>
          </section>
          <section class="ud-card">
            <div class="ud-card-title">
              <h2>核查概况</h2>
              <Icon name="shield" :size="18" />
            </div>
            <div class="ud-evidence-summary">
              <div>
                <span class="ud-evidence-icon"
                  ><Icon name="grid" :size="19"
                /></span>
                <div>
                  <b>设备与风险服务</b>
                  <p>
                    {{
                      detail.availability.device_trust_configured
                        ? "已配置"
                        : "尚未接入"
                    }}
                  </p>
                </div>
                <button @click="section = 'devices'">
                  查看设备 <Icon name="arrow" :size="12" />
                </button>
              </div>
              <div>
                <span class="ud-evidence-icon"
                  ><Icon name="shield" :size="19"
                /></span>
                <div>
                  <b>风险证据</b>
                  <p>
                    {{
                      detail.summary.risk_count
                        ? number(detail.summary.risk_count) + " 条记录"
                        : "尚无上报记录"
                    }}
                  </p>
                </div>
                <button @click="section = 'risk'">
                  查看证据 <Icon name="arrow" :size="12" />
                </button>
              </div>
              <div>
                <span class="ud-evidence-icon"
                  ><Icon name="send" :size="19"
                /></span>
                <div>
                  <b>邀请关系</b>
                  <p>
                    已注册 {{ number(detail.summary.invite_registered) }} ·
                    待加入 {{ number(detail.summary.invite_pending) }}
                  </p>
                </div>
                <button @click="section = 'invitations'">
                  查看关系 <Icon name="arrow" :size="12" />
                </button>
              </div>
            </div>
            <div class="ud-collection-status">
              <span
                >行为采集
                <b>{{ time(detail.availability.behavior_since) }}</b></span
              ><span
                >回调采集
                <b>{{ time(detail.availability.callbacks_since) }}</b></span
              ><span
                >管理审计
                <b>{{ time(detail.availability.audit_since) }}</b></span
              >
            </div>
          </section>
        </div>
        <section class="ud-card ud-last-handling">
          <div class="ud-card-title">
            <h2>最近账户处理</h2>
            <span>{{ statusName(detail.restriction.status) }}</span>
          </div>
          <div v-if="detail.restriction.updated_at" class="ud-handling-record">
            <p>{{ detail.restriction.reason }}</p>
            <span
              >{{ time(detail.restriction.updated_at) }} ·
              {{ detail.restriction.updated_by }}</span
            ><button
              v-if="detail.restriction.trace_id"
              class="ud-id"
              @click="copy(detail.restriction.trace_id)"
              :title="detail.restriction.trace_id"
            >
              {{ short(detail.restriction.trace_id) }}<small>复制追踪号</small>
            </button>
          </div>
          <div v-else class="ud-no-handling">暂无账户处理记录</div>
        </section>
        <div
          v-if="detail.availability.notes.length"
          class="ud-availability-notes"
        >
          <Icon name="bell" :size="15" />
          <div>
            <p v-for="note in detail.availability.notes" :key="note">
              {{ note }}
            </p>
          </div>
        </div>
      </template>
      <section v-else class="ud-card ud-records">
        <div class="ud-card-title">
          <h2>{{ sectionTitle }}</h2>
          <span>共 {{ number(result.total) }} 条</span>
        </div>
        <form class="ud-filters" @submit.prevent="loadRecords(1)">
          <label>开始日期<input v-model="filters.from" type="date" /></label
          ><label>结束日期<input v-model="filters.to" type="date" /></label
          ><label
            >类型<select v-model="filters.kind" aria-label="类型">
              <option value="">全部类型</option>
              <option v-for="kind in kindChoices" :key="kind" :value="kind">
                {{ statusName(kind) }}
              </option>
            </select></label
          ><label
            >状态<select v-model="filters.status" aria-label="状态">
              <option value="">全部状态</option>
              <option
                v-for="status in statusChoices"
                :key="status"
                :value="status"
              >
                {{ statusName(status) }}
              </option>
            </select></label
          ><label
            >最低金额（元）<input
              v-model="filters.min_amount"
              type="number"
              min="0"
              step="0.01"
              placeholder="不限" /></label
          ><label
            >最高金额（元）<input
              v-model="filters.max_amount"
              type="number"
              min="0"
              step="0.01"
              placeholder="不限" /></label
          ><label class="ud-related-filter"
            >关联编号<input
              v-model="filters.related"
              placeholder="会话号、交易号、设备号等"
          /></label>
          <div class="ud-filter-actions">
            <button class="primary" :disabled="recordsLoading">筛选</button
            ><button type="button" class="outline" @click="resetFilters">
              重置
            </button>
          </div>
        </form>

        <div v-if="result.note" class="ud-record-note">
          <Icon name="bell" :size="14" /><span>{{ result.note }}</span>
        </div>
        <div v-if="recordsLoading" class="ud-loading ud-record-loading">
          正在读取记录…
        </div>
        <div v-else-if="!result.items.length" class="ud-empty">
          <Icon
            :name="sections.find((item) => item.id === section).icon"
            :size="30"
          /><b>{{
            ["risk", "devices"].includes(section) &&
            !detail.availability.device_trust_configured
              ? "设备与风险服务尚未接入"
              : "暂无符合条件的记录"
          }}</b
          ><span>{{
            ["risk", "devices"].includes(section)
              ? "以实际收到的上报为准"
              : "可调整筛选条件后查询"
          }}</span>
        </div>
        <div v-else-if="section === 'timeline'" class="ud-timeline">
          <article v-for="item in result.items" :key="item.id">
            <i></i>
            <div>
              <header>
                <h3>{{ statusName(item.title) }}</h3>
                <span class="ud-status subtle">{{
                  statusName(item.status)
                }}</span><time>{{ time(item.created_at) }}</time>
              </header>
              <p>
                <span v-if="item.kind === 'risk_assessment'"
                  >{{ statusName(item.decision) }} · {{ statusName(item.action)
                  }}<template v-if="item.score != null">
                    · 规则 {{ item.score }} 分</template
                  ></span
                ><span>来源 {{ statusName(item.source) }}</span
                ><span>IP {{ item.client_ip || "未记录" }}</span
                ><span v-if="item.sdk_kind"
                  >场景 {{ statusName(item.sdk_kind) }}</span
                ><button
                  v-if="
                    item.operation_ref && item.operation_ref !== item.related_id
                  "
                  class="ud-id"
                  :title="item.operation_ref"
                  @click="copy(item.operation_ref)"
                >
                  {{ short(item.operation_ref)
                  }}<small>复制收益关联号</small></button
                ><button
                  v-if="item.related_id"
                  class="ud-id"
                  :title="item.related_id"
                  @click="copy(item.related_id)"
                >
                  {{ short(item.related_id) }}<small>复制关联号</small>
                </button>
              </p>
              <button class="ud-view-evidence" @click="selectedRecord = item">
                查看详情 <Icon name="arrow" :size="12" />
              </button>
            </div>
          </article>
        </div>
        <div v-else class="ud-table-scroll">
          <table v-if="section === 'ledger'">
            <thead>
              <tr>
                <th>时间</th>
                <th>事项</th>
                <th>金币变动</th>
                <th>折合金额</th>
                <th>状态</th>
                <th>关联编号</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>{{ time(item.created_at) }}</td>
                <td>
                  <b>{{ item.title }}</b><small>{{
                    statusName(item.kind)
                  }}</small>
                </td>
                <td class="ud-amount" :class="{ negative: item.coins < 0 }">
                  {{ item.coins > 0 ? "+" : "" }}{{ number(item.coins) }}
                </td>
                <td>¥{{ money(item.amount_yuan) }}</td>
                <td>{{ statusName(item.status) }}</td>
                <td>
                  <button
                    class="ud-id"
                    :title="item.source"
                    @click="copy(item.source)"
                  >
                    {{ short(item.source) }}<small>复制</small>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <table v-if="section === 'ads'">
            <thead>
              <tr>
                <th>发起时间</th>
                <th>任务与来源</th>
                <th>奖励</th>
                <th>状态</th>
                <th>平台确认</th>
                <th>入账时间</th>
                <th>会话编号</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>{{ time(item.started_at) }}</td>
                <td>
                  <b>{{ statusName(item.task_id) }}</b><small>{{
                    statusName(item.provider)
                  }}</small>
                </td>
                <td class="ud-amount">
                  {{ number(item.reward) }} 金币
                  <small v-if="item.reward_tier"
                    >{{ tierName(item.reward_tier) }} · V{{
                      item.policy_version
                    }}</small
                  >
                </td>
                <td>
                  <span class="ud-status subtle">{{
                    statusName(item.status)
                  }}</span>
                </td>
                <td>{{ time(item.verified_at) }}</td>
                <td>{{ time(item.completed_at) }}</td>
                <td>
                  <button class="ud-id" :title="item.id" @click="copy(item.id)">
                    {{ short(item.id) }}<small>复制</small>
                  </button>
                </td>
                <td>
                  <button
                    class="ud-view-evidence"
                    @click="selectedRecord = item"
                  >
                    明细
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <table v-if="section === 'callbacks'">
            <thead>
              <tr>
                <th>记录时间</th>
                <th>来源</th>
                <th>校验结果</th>
                <th>原因</th>
                <th>奖励会话</th>
                <th>平台交易号</th>
                <th>来源 IP</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>{{ time(item.created_at) }}</td>
                <td>
                  <span
                    class="ud-status"
                    :class="item.source === 'business' ? 'subtle' : ''"
                    >{{ statusName(item.source) }}</span
                  >
                </td>
                <td>{{ statusName(item.status) }}</td>
                <td>{{ statusName(item.reason_code) }}</td>
                <td>
                  <button
                    v-if="item.reward_session_id"
                    class="ud-id"
                    :title="item.reward_session_id"
                    @click="copy(item.reward_session_id)"
                  >
                    {{ short(item.reward_session_id)
                    }}<small>复制</small></button
                  ><span v-else>未记录</span>
                </td>
                <td>
                  <button
                    v-if="item.trans_id"
                    class="ud-id"
                    :title="item.trans_id"
                    @click="copy(item.trans_id)"
                  >
                    {{ short(item.trans_id) }}<small>复制</small></button
                  ><span v-else>未记录</span>
                </td>
                <td>{{ item.client_ip || "未记录" }}</td>
                <td>
                  <button
                    class="ud-view-evidence"
                    @click="selectedRecord = item"
                  >
                    证据
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <table v-if="section === 'withdrawals'">
            <thead>
              <tr>
                <th>申请时间</th>
                <th>方式</th>
                <th>金额</th>
                <th>状态</th>
                <th>收款账号</th>
                <th>处理时间</th>
                <th>申请编号</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>{{ time(item.created_at) }}</td>
                <td>{{ statusName(item.channel) }}</td>
                <td class="ud-amount">¥{{ money(item.amount_yuan) }}</td>
                <td>
                  <span class="ud-status subtle">{{
                    statusName(item.status)
                  }}</span>
                </td>
                <td>{{ item.account }}</td>
                <td>{{ time(item.reviewed_at) }}</td>
                <td>
                  <button class="ud-id" :title="item.id" @click="copy(item.id)">
                    {{ short(item.id) }}<small>复制</small>
                  </button>
                </td>
                <td>
                  <button
                    class="ud-view-evidence"
                    @click="selectedRecord = item"
                  >
                    明细
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <table v-if="section === 'invitations'">
            <thead>
              <tr>
                <th>关系</th>
                <th>关联账号</th>
                <th>邀请码</th>
                <th>状态</th>
                <th>登记时间</th>
                <th>绑定时间</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>{{ statusName(item.direction) }}</td>
                <td>
                  <button
                    v-if="item.related_user_id"
                    class="ud-account-link"
                    @click="emit('navigate', item.related_user_id)"
                  >
                    {{ item.nickname }} · {{ item.phone
                    }}<Icon name="arrow" :size="12" /></button
                  ><span v-else>{{
                    item.nickname || item.phone || "平台邀请"
                  }}</span>
                </td>
                <td>
                  <button class="ud-id" @click="copy(item.invite_code)">
                    {{ item.invite_code }}<small>复制</small>
                  </button>
                </td>
                <td>{{ statusName(item.status) }}</td>
                <td>{{ time(item.created_at) }}</td>
                <td>{{ time(item.bound_at) }}</td>
              </tr>
            </tbody>
          </table>
          <table v-if="section === 'risk'">
            <thead>
              <tr>
                <th>上报时间</th>
                <th>核验结果</th>
                <th>处理动作</th>
                <th>规则分值</th>
                <th>报告编号</th>
                <th>来源 IP</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>{{ time(item.created_at) }}</td>
                <td>{{ statusName(item.decision) }}</td>
                <td>{{ statusName(item.action) }}</td>
                <td>{{ item.score == null ? "未提供" : item.score }}</td>
                <td>
                  <button
                    v-if="item.report_id"
                    class="ud-id"
                    :title="item.report_id"
                    @click="copy(item.report_id)"
                  >
                    {{ short(item.report_id) }}<small>复制</small></button
                  ><span v-else>未提供</span>
                </td>
                <td>{{ item.client_ip || "未记录" }}</td>
                <td>
                  <button
                    class="ud-view-evidence"
                    @click="selectedRecord = item"
                  >
                    查看证据
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <table v-if="section === 'devices'">
            <thead>
              <tr>
                <th>设备编号</th>
                <th>平台与机型</th>
                <th>系统</th>
                <th>首次出现</th>
                <th>最近出现</th>
                <th>关联账号</th>
                <th>上报状态</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>
                  <button
                    class="ud-id"
                    :title="item.device_id"
                    @click="copy(item.device_id)"
                  >
                    {{ short(item.device_id) }}<small>复制</small>
                  </button>
                </td>
                <td>
                  <b>{{ item.model || "未上报机型" }}</b><small>{{
                    item.platform || "未上报平台"
                  }}</small>
                </td>
                <td>{{ item.os_version || "未上报" }}</td>
                <td>{{ time(item.first_seen) }}</td>
                <td>{{ time(item.last_seen) }}</td>
                <td>
                  <div class="ud-related-accounts">
                    <button
                      v-for="account in item.related_users"
                      :key="account.id"
                      class="ud-account-link"
                      @click="emit('navigate', account.id)"
                    >
                      {{ account.phone || account.nickname
                      }}<Icon name="arrow" :size="12" />
                    </button>
                  </div>
                </td>
                <td>{{ riskName(item.evidence_status) }}</td>
              </tr>
            </tbody>
          </table>
          <table v-if="section === 'audit'">
            <thead>
              <tr>
                <th>操作时间</th>
                <th>操作人</th>
                <th>操作</th>
                <th>结果</th>
                <th>处理原因</th>
                <th>来源 IP</th>
                <th>请求编号</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in result.items" :key="item.id">
                <td>{{ time(item.created_at) }}</td>
                <td>{{ item.actor }}</td>
                <td>{{ statusName(item.action) }}</td>
                <td>
                  <span
                    class="ud-status"
                    :class="item.http_status >= 400 ? 'restricted' : 'subtle'"
                    >{{
                      item.http_status == null
                        ? statusName(item.status)
                        : item.http_status
                    }}</span
                  >
                </td>
                <td>{{ item.reason || "—" }}</td>
                <td>{{ item.client_ip || "未记录" }}</td>
                <td>
                  <button
                    v-if="item.request_id"
                    class="ud-id"
                    :title="item.request_id"
                    @click="copy(item.request_id)"
                  >
                    {{ short(item.request_id) }}<small>复制</small></button
                  ><span v-else>未记录</span>
                </td>
                <td>
                  <button
                    class="ud-view-evidence"
                    @click="selectedRecord = item"
                  >
                    明细
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="ud-pagination">
          <span
            >第 {{ result.page }} /
            {{ Math.max(1, Math.ceil(result.total / result.page_size)) }}
            页</span
          >
          <div>
            <label
              >每页<select
                v-model.number="pageSize"
                aria-label="每页条数"
                @change="loadRecords(1)"
              >
                <option :value="10">10 条</option>
                <option :value="20">20 条</option>
                <option :value="50">50 条</option>
                <option :value="100">100 条</option>
              </select></label
            ><button
              :disabled="recordsLoading || result.page === 1"
              @click="loadRecords(result.page - 1)"
            >
              上一页</button
            ><button
              :disabled="
                recordsLoading || result.page * result.page_size >= result.total
              "
              @click="loadRecords(result.page + 1)"
            >
              下一页
            </button>
          </div>
        </div>
      </section>
      <div
        v-if="selectedRecord"
        class="ud-record-overlay"
        @click.self="selectedRecord = null"
      >
        <section class="ud-record-detail">
          <header>
            <div>
              <h2>{{ sectionTitle }}明细</h2>
            </div>
            <button aria-label="关闭记录明细" @click="selectedRecord = null">
              <Icon name="close" :size="21" />
            </button>
          </header>
          <dl>
            <div>
              <dt>记录编号</dt>
              <dd>
                <button class="ud-id" @click="copy(selectedRecord.id)">
                  {{ selectedRecord.id }}<small>复制</small>
                </button>
              </dd>
            </div>
            <div>
              <dt>记录时间</dt>
              <dd>{{ time(selectedRecord.created_at) }}</dd>
            </div>
            <div>
              <dt>状态</dt>
              <dd>{{ statusName(selectedRecord.status) }}</dd>
            </div>
            <div v-if="selectedRecord.related_id">
              <dt>关联编号</dt>
              <dd>
                <button class="ud-id" @click="copy(selectedRecord.related_id)">
                  {{ selectedRecord.related_id }}<small>复制</small>
                </button>
              </dd>
            </div>
            <div v-if="selectedRecord.error || selectedRecord.detail?.error">
              <dt>错误信息</dt>
              <dd class="ud-upstream-error">
                {{ selectedRecord.error || selectedRecord.detail.error }}
              </dd>
            </div>
            <div v-if="selectedRecord.reason">
              <dt>原因</dt>
              <dd>{{ selectedRecord.reason }}</dd>
            </div>
          </dl>
          <section v-if="section === 'ads'" class="ud-reward-proof">
            <h3>奖励发放依据</h3>
            <div class="ud-proof-summary">
              <span
                >本次实际金币<b>{{ number(selectedRecord.reward) }}</b></span
              >
              <span
                >匹配档位<b>{{ tierName(selectedRecord.reward_tier) }}</b></span
              >
            </div>
            <dl>
              <div>
                <dt>策略版本</dt>
                <dd>
                  {{
                    selectedRecord.policy_version == null
                      ? "未留存"
                      : "V" + selectedRecord.policy_version
                  }}
                </dd>
              </div>
              <div>
                <dt>本次奖励区间</dt>
                <dd>
                  {{
                    selectedRecord.reward_min == null ||
                    selectedRecord.reward_max == null
                      ? "未留存"
                      : number(selectedRecord.reward_min) +
                        "–" +
                        number(selectedRecord.reward_max) +
                        " 金币"
                  }}
                </dd>
              </div>
              <div>
                <dt>首次验签时间</dt>
                <dd>{{ time(selectedRecord.device_first_seen) }}</dd>
              </div>
              <div>
                <dt>报价时设备天龄</dt>
                <dd>
                  {{
                    selectedRecord.device_age_days == null
                      ? "未确认"
                      : Number(selectedRecord.device_age_days).toFixed(2) +
                        " 天"
                  }}
                </dd>
              </div>
              <div>
                <dt>设备此前已完成</dt>
                <dd>
                  {{
                    selectedRecord.device_completed_before == null
                      ? "未确认"
                      : number(selectedRecord.device_completed_before) + " 次"
                  }}<small v-if="selectedRecord.device_completed_before != null"
                    >跨账号累计</small
                  >
                </dd>
              </div>
              <div v-if="selectedRecord.reward_basis">
                <dt>有效在途会话</dt>
                <dd>
                  {{ number(selectedRecord.reward_basis.reserved_before) }} 次
                </dd>
              </div>
              <div v-if="selectedRecord.reward_basis">
                <dt>此前累计名额</dt>
                <dd>
                  {{
                    number(selectedRecord.reward_basis.allocation_count_before)
                  }}
                  次
                </dd>
              </div>
              <div v-if="selectedRecord.reward_basis">
                <dt>设备依据</dt>
                <dd>
                  {{ identityName(selectedRecord.reward_basis.identity_kind) }}
                </dd>
              </div>
              <div v-if="selectedRecord.reward_basis?.evidence_source">
                <dt>记录来源</dt>
                <dd>
                  {{
                    identityName(selectedRecord.reward_basis.evidence_source)
                  }}
                </dd>
              </div>
              <div
                v-if="
                  selectedRecord.reward_basis?.device_identity &&
                  selectedRecord.reward_basis.device_identity !== 'unknown'
                "
              >
                <dt>报价设备关联</dt>
                <dd>
                  <button
                    class="ud-id"
                    @click="copy(selectedRecord.reward_basis.device_identity)"
                  >
                    {{ selectedRecord.reward_basis.device_identity
                    }}<small>复制</small>
                  </button>
                </dd>
              </div>
              <div
                v-if="
                  selectedRecord.reward_basis?.actual_device_identity &&
                  selectedRecord.reward_basis.actual_device_identity !==
                    'unknown'
                "
              >
                <dt>会话设备关联</dt>
                <dd>
                  <button
                    class="ud-id"
                    @click="
                      copy(selectedRecord.reward_basis.actual_device_identity)
                    "
                  >
                    {{ selectedRecord.reward_basis.actual_device_identity
                    }}<small>复制</small>
                  </button>
                </dd>
              </div>
              <div v-if="selectedRecord.reward_basis?.key_id">
                <dt>设备密钥标识</dt>
                <dd>
                  <button
                    class="ud-id"
                    @click="copy(selectedRecord.reward_basis.key_id)"
                  >
                    {{ selectedRecord.reward_basis.key_id }}<small>复制</small>
                  </button>
                </dd>
              </div>
              <div v-if="selectedRecord.quote_id">
                <dt>报价编号</dt>
                <dd>
                  <button class="ud-id" @click="copy(selectedRecord.quote_id)">
                    {{ selectedRecord.quote_id }}<small>复制</small>
                  </button>
                </dd>
              </div>
            </dl>
            <details v-if="selectedRecord.reward_basis" class="ud-raw-record">
              <summary>查看奖励依据快照</summary>
              <pre>{{
                JSON.stringify(selectedRecord.reward_basis, null, 2)
              }}</pre>
            </details>
          </section>
          <section v-if="section === 'risk'" class="ud-risk-proof">
            <h3>规则与证据</h3>
            <div class="ud-proof-summary">
              <span
                >规则分值
                <b>{{
                  selectedRecord.score == null ? "未提供" : selectedRecord.score
                }}</b></span
              ><span
                >证据类型
                <b>{{
                  riskName(selectedRecord.detail?.reportProof?.level)
                }}</b></span
              >
            </div>
            <template v-if="selectedRecord.detail?.hits?.length"
              ><article
                v-for="(hit, index) in selectedRecord.detail.hits"
                :key="index"
                class="ud-rule-hit"
              >
                <div>
                  <b>{{ riskName(hit.code) }}</b><span
                    >+{{ hit.points }} 分</span
                  >
                </div>
                <p>
                  {{ riskName(hit.source)
                  }}<span v-if="hit.observed"> · 实际 {{ hit.observed }}</span
                  ><span v-if="hit.threshold"> · 阈值 {{ hit.threshold }}</span
                  ><span v-if="hit.windowSeconds">
                    · {{ hit.windowSeconds }}秒内</span
                  >
                </p>
              </article></template
            >
            <p v-else class="ud-proof-empty">
              {{
                selectedRecord.status === "reported"
                  ? "本次核验未返回命中规则"
                  : "规则证据尚未返回"
              }}
            </p>
            <div
              v-if="selectedRecord.detail?.missingFields?.length"
              class="ud-missing-fields"
            >
              <span>未提供的业务信息</span
              ><b
                v-for="field in selectedRecord.detail.missingFields"
                :key="field"
                >{{ riskName(field) }}</b
              >
            </div>
            <div v-if="selectedRecord.key_id">
              <label>设备密钥标识</label
              ><button class="ud-id" @click="copy(selectedRecord.key_id)">
                {{ selectedRecord.key_id }}<small>复制</small>
              </button>
            </div>
          </section>
          <details class="ud-raw-record">
            <summary>查看原始记录</summary>
            <pre>{{ JSON.stringify(selectedRecord, null, 2) }}</pre>
          </details>
        </section>
      </div>
    </template>
    <div v-if="notice" class="ud-notice">
      <Icon name="check" :size="15" />{{ notice }}
    </div>
  </div>
</template>
<style scoped>
.ud-profile-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 14px;
}
.ud-profile-actions > button {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: #b78e62;
  white-space: nowrap;
}
.ud-workspace {
  min-width: 0;
}
.ud-breadcrumb {
  display: flex;
  align-items: center;
  gap: 11px;
  font-size: 11px;
  color: #c3ac90;
  margin: -3px 0 21px;
}
.ud-breadcrumb button {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #a98a68;
  padding: 3px 0;
}
.ud-breadcrumb b {
  color: #9d7e5c;
  font-weight: 500;
}
.ud-error {
  margin: 0 0 18px;
}
.ud-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 80px 20px;
  color: #b69a7a;
  font-size: 12px;
}
.ud-profile {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 23px 25px;
  background: #fffdf9;
  border: 1px solid #eaddcc;
  border-radius: 18px;
}
.ud-identity {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}
.ud-avatar {
  width: 53px;
  height: 53px;
  display: grid;
  place-items: center;
  background: linear-gradient(135deg, #ffe9c6, #f9cf95);
  border: 3px solid #fff4e3;
  color: #be854a;
  border-radius: 17px;
  font-size: 22px;
  font-weight: 600;
  flex-shrink: 0;
}
.ud-name {
  display: flex;
  align-items: center;
  gap: 9px;
  flex-wrap: wrap;
}
.ud-name h1 {
  font-size: 22px;
  color: #6d5037;
  letter-spacing: -0.5px;
  margin-right: 3px;
}
.ud-status {
  display: inline-block;
  color: #ad865a;
  background: #fff0d8;
  border: 1px solid #f0d7b2;
  border-radius: 5px;
  padding: 4px 7px;
  font-size: 9px;
  white-space: nowrap;
}
.ud-status.subtle,
.ud-status.active {
  color: #ad9372;
  background: #f8f1e7;
  border-color: #ece0cd;
}
.ud-status.review {
  color: #be8c37;
  background: #fff0ce;
  border-color: #efd19c;
}
.ud-status.restricted {
  color: #bd6d56;
  background: #fff0e7;
  border-color: #edc8b7;
}
.ud-identity-meta {
  display: flex;
  align-items: center;
  gap: 13px;
  flex-wrap: wrap;
  color: #b69b7a;
  font-size: 10px;
  margin-top: 9px;
}
.ud-identity-meta button {
  font-size: 12px;
  color: #a47a50;
  letter-spacing: 0.5px;
  padding: 0;
}
.ud-handle-button {
  display: flex;
  align-items: center;
  gap: 7px;
  border: 1px solid #e8d7c0;
  background: #fff7e9;
  color: #ba8a56;
  padding: 10px 13px;
  border-radius: 9px;
  font-size: 10px;
  white-space: nowrap;
}
.ud-handling {
  display: flex;
  align-items: flex-end;
  gap: 15px;
  border: 1px solid #ecd6b4;
  border-radius: 13px;
  padding: 19px 22px;
  background: #fff6e6;
  margin: 13px 0 0;
}
.ud-handling label {
  font-size: 10px;
  color: #ac8a62;
  margin: 0;
  width: 150px;
}
.ud-handling input,
.ud-handling select {
  font-size: 12px;
  border-color: #e8d4b7;
  background: #fffdf7;
  border-radius: 9px;
  padding: 11px 12px;
}
.ud-handling .ud-handling-reason {
  flex: 1;
}
.ud-handling .primary {
  font-size: 11px;
  border-radius: 9px;
  padding: 12px 16px;
  white-space: nowrap;
}
.ud-stats {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 14px;
  margin: 19px 0;
}
.ud-stat {
  border: 1px solid #ecdfce;
  border-radius: 15px;
  background: #fffdf9;
  padding: 18px 20px;
}
.ud-stat > span {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: center;
  color: #b0926f;
  font-size: 10px;
}
.ud-stat > span > svg {
  color: #c8a174;
}
.ud-stat > b {
  display: block;
  font-size: 30px;
  font-weight: 600;
  color: #715238;
  letter-spacing: -0.8px;
  margin-top: 17px;
  line-height: 1;
}
.ud-stat > b small {
  font-size: 10px;
  letter-spacing: 0;
  font-weight: 400;
  margin: 0 5px;
  color: #b39a79;
}
.ud-stat > p {
  font-size: 9px;
  color: #b69b79;
  margin-top: 13px;
  line-height: 1.8;
}
.ud-stat.primary {
  display: block;
  box-shadow: 0 6px 16px #e3925418;
  background: linear-gradient(125deg, #f9a967, #ed8250);
  border-color: #eea06b;
}
.ud-stat.primary > span,
.ud-stat.primary > span > svg,
.ud-stat.primary > b,
.ud-stat.primary > b small,
.ud-stat.primary > p {
  color: #fff4df;
}
.ud-tabs {
  display: flex;
  gap: 3px;
  overflow-x: auto;
  background: #fffaf3;
  border: 1px solid #eaddca;
  border-radius: 12px;
  padding: 6px;
  margin: 23px 0 20px;
  scrollbar-width: thin;
  scrollbar-color: #e4d0b6 transparent;
}
.ud-tabs button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 10px 11px;
  font-size: 11px;
  color: #ab8f6e;
  border-radius: 8px;
  white-space: nowrap;
  flex: 1;
}
.ud-tabs button:hover {
  background: #fbefdd;
}
.ud-tabs button.active {
  background: #ffead0;
  color: #c68c4e;
  font-weight: 600;
}
.ud-card {
  border: 1px solid #ecdfcc;
  background: #fffdf8;
  border-radius: 16px;
  min-width: 0;
  overflow: hidden;
}
.ud-card-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 21px 23px;
  gap: 15px;
  border-bottom: 1px solid #f3e9db;
}
.ud-card-title h2 {
  font-size: 14px;
  font-weight: 600;
  color: #7e5c3e;
}
.ud-card-title > span,
.ud-card-title > svg {
  font-size: 10px;
  color: #b49a78;
}
.ud-overview-grid {
  display: grid;
  grid-template-columns: 1.17fr 1fr;
  gap: 20px;
}
.ud-facts {
  display: grid;
  grid-template-columns: 1fr 1fr;
  margin: 0;
  padding: 4px 23px 16px;
  column-gap: 25px;
}
.ud-facts > div {
  padding: 15px 0 12px;
  border-bottom: 1px solid #f4ebde;
  min-width: 0;
}
.ud-facts > div:nth-last-child(-n + 2) {
  border: 0;
}
.ud-facts dt {
  font-size: 10px;
  color: #bea487;
  margin-bottom: 9px;
}
.ud-facts dd {
  font-size: 12px;
  color: #937251;
  margin: 0;
  overflow-wrap: anywhere;
  line-height: 1.8;
}
.ud-evidence-summary {
  padding: 1px 22px;
}
.ud-evidence-summary > div {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 17px 0;
  border-bottom: 1px solid #f2e8da;
}
.ud-evidence-summary > div:last-child {
  border: 0;
}
.ud-evidence-icon {
  display: grid;
  place-items: center;
  width: 35px;
  height: 35px;
  flex-shrink: 0;
  background: #fff0db;
  color: #c79b65;
  border: 1px solid #f2dfc1;
  border-radius: 11px;
}
.ud-evidence-summary b {
  font-size: 11px;
  color: #947353;
  font-weight: 500;
}
.ud-evidence-summary p {
  font-size: 9px;
  color: #bea282;
  line-height: 1.8;
  margin-top: 5px;
}
.ud-evidence-summary button {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 3px;
  font-size: 9px;
  color: #be9a70;
  white-space: nowrap;
}
.ud-collection-status {
  padding: 14px 22px;
  background: #fcf6ec;
  border-top: 1px solid #f1e5d3;
  display: grid;
  gap: 8px;
}
.ud-collection-status > span {
  display: flex;
  justify-content: space-between;
  font-size: 9px;
  color: #b59a78;
  gap: 12px;
}
.ud-collection-status b {
  font-size: 9px;
  font-weight: 400;
  color: #b99e7d;
}
.ud-last-handling {
  margin-top: 20px;
}
.ud-handling-record {
  padding: 20px 23px;
  display: flex;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}
.ud-handling-record p {
  font-size: 12px;
  color: #a5825e;
  flex-basis: 100%;
  line-height: 1.8;
}
.ud-handling-record > span {
  font-size: 10px;
  color: #b79a78;
}
.ud-no-handling {
  padding: 24px;
  font-size: 11px;
  color: #c1a98d;
}
.ud-availability-notes {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-top: 18px;
  padding: 14px 17px;
  background: #f7efdf;
  border: 1px solid #eddfc6;
  border-radius: 10px;
  color: #b99b75;
}
.ud-availability-notes > svg {
  flex-shrink: 0;
  margin-top: 2px;
}
.ud-availability-notes p {
  font-size: 10px;
  line-height: 1.9;
}
.ud-filters {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 12px;
  padding: 20px 23px;
  background: #fffcf6;
  border-bottom: 1px solid #f0e4d1;
}
.ud-filters label {
  font-size: 9px;
  color: #b39877;
  margin: 0;
  min-width: 0;
}
.ud-filters input,
.ud-filters select {
  font-size: 11px;
  padding: 10px 9px;
  min-height: 37px;
  border-radius: 8px;
  background: #fffdf8;
  border-color: #e9dbc6;
  width: 100%;
  min-width: 0;
  margin-top: 6px;
}
.ud-filters .ud-related-filter {
  grid-column: span 4;
}
.ud-filter-actions {
  grid-column: span 2;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  gap: 8px;
}
.ud-filter-actions > .primary,
.ud-filter-actions > .outline {
  font-size: 10px;
  padding: 10px 18px;
  border-radius: 8px;
  min-height: 37px;
}
.ud-record-note {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  font-size: 10px;
  color: #b59a78;
  line-height: 1.8;
  padding: 11px 23px;
  background: #fcf6eb;
  border-bottom: 1px solid #f0e4d2;
}
.ud-record-note > svg {
  flex-shrink: 0;
  margin-top: 2px;
}
.ud-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 55px 20px;
  gap: 13px;
  color: #d0b793;
  text-align: center;
}
.ud-empty b {
  font-size: 12px;
  font-weight: 400;
  color: #bfa17d;
}
.ud-empty > span {
  font-size: 10px;
  color: #c7ad8d;
}
.ud-table-scroll {
  overflow: auto;
  max-width: 100%;
}
.ud-table-scroll table {
  width: 100%;
  border-collapse: collapse;
  white-space: nowrap;
}
.ud-table-scroll th {
  font-size: 9px;
  color: #b69a75;
  padding: 13px 19px;
  background: #fbf4e9;
  border-top: 0;
}
.ud-table-scroll td {
  font-size: 10px;
  color: #a08361;
  padding: 17px 19px;
  line-height: 1.7;
  border-bottom-color: #f3e9d9;
}
.ud-table-scroll td > b {
  font-size: 11px;
  font-weight: 500;
  display: block;
  color: #8c6b49;
}
.ud-table-scroll td > small {
  font-size: 9px;
  display: block;
  color: #c2a887;
  margin-top: 3px;
}
.ud-table-scroll td.ud-amount {
  color: #cf9252;
  font-weight: 500;
  font-size: 12px;
}
.ud-table-scroll td.ud-amount.negative {
  color: #b19575;
}
.ud-id {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  flex-wrap: wrap;
  color: #b18e65;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 10px;
  padding: 0;
  text-align: left;
  overflow-wrap: anywhere;
  max-width: 100%;
}
.ud-id > small {
  font-family: Inter, "Microsoft YaHei", sans-serif;
  font-size: 8px;
  color: #c5ac8c;
  white-space: nowrap;
}
.ud-id:hover {
  color: #dc9a53;
}
.ud-account-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #c3925d;
  font-size: 11px;
  padding: 0;
  white-space: nowrap;
}
.ud-related-accounts {
  display: flex;
  flex-direction: column;
  gap: 7px;
}
.ud-related-accounts .ud-account-link {
  font-size: 10px;
}
.ud-view-evidence {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #be965f;
  font-size: 10px;
  white-space: nowrap;
  padding: 0;
}
.ud-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 23px;
  border-top: 1px solid #eee1cc;
  font-size: 10px;
  color: #b29a77;
  gap: 14px;
}
.ud-pagination > div {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ud-pagination label {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 9px;
  color: #b69b76;
  margin: 0;
}
.ud-pagination select {
  width: 70px;
  padding: 7px 5px;
  border: 1px solid #e5d5bb;
  border-radius: 7px;
  font-size: 10px;
  background: #fffaf1;
}
.ud-pagination button {
  border: 1px solid #e8d7bd;
  background: #fff9ee;
  color: #ad8d61;
  font-size: 10px;
  padding: 8px 12px;
  border-radius: 7px;
}
.ud-timeline {
  padding: 24px 25px 7px;
}
.ud-timeline article {
  display: flex;
  gap: 16px;
  position: relative;
  padding-bottom: 27px;
}
.ud-timeline article:not(:last-child):after {
  content: "";
  position: absolute;
  left: 5px;
  top: 13px;
  bottom: 0;
  width: 1px;
  background: #edddc6;
}
.ud-timeline article > i {
  width: 11px;
  height: 11px;
  border: 3px solid #f5e4c5;
  background: #d8b179;
  border-radius: 50%;
  flex-shrink: 0;
  margin-top: 5px;
}
.ud-timeline article > div {
  min-width: 0;
  flex: 1;
  border-bottom: 1px solid #f2e5d0;
  padding-bottom: 21px;
}
.ud-timeline article:last-child > div {
  border: 0;
}
.ud-timeline header {
  display: flex;
  align-items: center;
  gap: 9px;
  flex-wrap: wrap;
}
.ud-timeline h3 {
  font-size: 12px;
  color: #93704a;
  font-weight: 500;
}
.ud-timeline time {
  font-size: 10px;
  color: #b89c77;
  margin-left: auto;
}
.ud-timeline p {
  display: flex;
  align-items: center;
  gap: 13px;
  flex-wrap: wrap;
  font-size: 10px;
  color: #baa07e;
  margin: 11px 0 10px;
  line-height: 1.7;
}
.ud-timeline .ud-view-evidence {
  font-size: 9px;
}
.ud-record-overlay {
  position: fixed;
  inset: 0;
  background: #4a332746;
  backdrop-filter: blur(3px);
  z-index: 60;
  display: flex;
  justify-content: flex-end;
}
.ud-record-detail {
  width: 570px;
  max-width: 100%;
  height: 100%;
  overflow: auto;
  scrollbar-width: thin;
  background: #fffaf2;
  border-left: 1px solid #ead7b8;
  padding: 27px 30px;
  box-shadow: -10px 0 40px #77502a13;
}
.ud-record-detail > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #ebdbc3;
  padding-bottom: 21px;
  gap: 15px;
}
.ud-record-detail h2 {
  font-size: 21px;
  color: #8b603a;
}
.ud-record-detail > header button {
  padding: 8px;
  border-radius: 50%;
  color: #b89a75;
}
.ud-record-detail > dl {
  margin: 9px 0 25px;
}
.ud-record-detail > dl > div {
  padding: 16px 0;
  border-bottom: 1px solid #efe1cc;
}
.ud-record-detail dt {
  font-size: 10px;
  color: #baa07c;
  margin-bottom: 8px;
}
.ud-record-detail dd {
  font-size: 12px;
  color: #a08057;
  line-height: 1.8;
  margin: 0;
  overflow-wrap: anywhere;
}
.ud-record-detail .ud-upstream-error {
  color: #bb7256;
}
.ud-risk-proof,
.ud-reward-proof {
  border: 1px solid #ead8bb;
  background: #fffdf6;
  border-radius: 13px;
  padding: 19px;
  margin-bottom: 22px;
}
.ud-risk-proof > h3,
.ud-reward-proof > h3 {
  font-size: 13px;
  color: #9a754a;
  margin-bottom: 16px;
}
.ud-reward-proof dl {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 17px;
  margin: 19px 0;
}
.ud-reward-proof dd > small {
  display: block;
  color: #baa17f;
  font-size: 9px;
  margin-top: 2px;
}
.ud-reward-proof .ud-raw-record {
  margin-top: 18px;
}
.ud-proof-summary {
  display: flex;
  justify-content: space-between;
  gap: 15px;
  flex-wrap: wrap;
  padding-bottom: 15px;
  margin-bottom: 4px;
  border-bottom: 1px solid #efe1cb;
}
.ud-proof-summary > span {
  font-size: 10px;
  color: #b89b75;
}
.ud-proof-summary b {
  display: block;
  margin-top: 9px;
  font-size: 18px;
  font-weight: 500;
  color: #b98a4e;
}
.ud-proof-summary > span:nth-child(2) > b {
  font-size: 12px;
  margin-top: 12px;
}
.ud-rule-hit {
  padding: 15px 0 12px;
  border-bottom: 1px solid #f0e2cc;
}
.ud-rule-hit > div {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  font-size: 11px;
  color: #ae8757;
}
.ud-rule-hit b {
  font-size: 11px;
  font-weight: 500;
}
.ud-rule-hit p {
  font-size: 9px;
  line-height: 1.9;
  color: #bea481;
  margin-top: 7px;
}
.ud-proof-empty {
  font-size: 11px;
  color: #c0a582;
  padding: 20px 0;
}
.ud-missing-fields {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  padding: 15px 0;
}
.ud-missing-fields > span {
  flex-basis: 100%;
  font-size: 9px;
  color: #b89c75;
  margin-bottom: 4px;
}
.ud-missing-fields b {
  font-size: 9px;
  font-weight: 400;
  background: #f8edda;
  color: #be9c6e;
  border: 1px solid #eddbbb;
  border-radius: 5px;
  padding: 4px 6px;
}
.ud-risk-proof > div:last-child > label {
  display: block;
  font-size: 9px;
  color: #bba17b;
  font-weight: 400;
  margin: 16px 0 8px;
}
.ud-raw-record {
  border: 1px solid #e9d8be;
  border-radius: 10px;
  background: #fffdf7;
  overflow: hidden;
}
.ud-raw-record summary {
  font-size: 11px;
  color: #b2936a;
  padding: 14px 16px;
  cursor: pointer;
}
.ud-raw-record pre {
  margin: 0;
  border-top: 1px solid #eee0c9;
  background: #faf4e8;
  color: #a2835b;
  padding: 16px;
  overflow: auto;
  font-size: 10px;
  line-height: 1.9;
  max-height: 460px;
}
.ud-notice {
  position: fixed;
  bottom: 30px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 11px 19px;
  background: #735333ef;
  color: #fff0d9;
  border-radius: 100px;
  font-size: 11px;
  z-index: 80;
  box-shadow: 0 8px 25px #86613d1c;
}
@media (max-width: 1250px) {
  .ud-tabs button {
    padding: 9px;
    font-size: 10px;
  }
  .ud-tabs button svg {
    display: none;
  }
  .ud-stat {
    padding: 17px;
  }
  .ud-stat > b {
    font-size: 26px;
  }
  .ud-filters {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .ud-filters .ud-related-filter {
    grid-column: span 2;
  }
  .ud-filter-actions {
    grid-column: span 1;
  }
  .ud-overview-grid {
    grid-template-columns: 1fr 1fr;
  }
  .ud-facts {
    column-gap: 16px;
    padding-left: 19px;
    padding-right: 19px;
  }
  .ud-identity-meta {
    gap: 9px;
    font-size: 9px;
  }
  .ud-profile {
    padding: 21px;
  }
  .ud-evidence-summary {
    padding: 0 18px;
  }
  .ud-evidence-summary button {
    font-size: 8px;
  }
  .ud-evidence-summary > div {
    gap: 8px;
  }
  .ud-collection-status {
    padding: 14px 18px;
  }
  .ud-card-title {
    padding: 20px;
  }
  .ud-evidence-icon {
    width: 30px;
    height: 30px;
    border-radius: 9px;
  }
}
@media (max-width: 950px) {
  .ud-profile {
    align-items: flex-start;
  }
  .ud-name h1 {
    font-size: 20px;
    flex-basis: 100%;
    margin-bottom: 4px;
  }
  .ud-identity-meta > span:last-child {
    flex-basis: 100%;
  }
  .ud-stats {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  .ud-stat > b {
    font-size: 28px;
    margin-top: 14px;
  }
  .ud-stat > p {
    margin-top: 11px;
  }
  .ud-overview-grid {
    grid-template-columns: 1fr;
  }
  .ud-profile {
    padding: 20px;
  }
  .ud-handling {
    flex-wrap: wrap;
  }
  .ud-handling label {
    width: 135px;
  }
  .ud-handling-reason {
    min-width: 180px;
  }
  .ud-handling .primary {
    margin-left: auto;
  }
  .ud-tabs button {
    flex: initial;
    padding: 10px 12px;
  }
  .ud-tabs button svg {
    display: block;
  }
  .ud-timeline time {
    margin-left: 0;
    flex-basis: 100%;
    margin-top: 3px;
  }
}
@media (max-width: 650px) {
  .ud-breadcrumb {
    font-size: 10px;
    margin-bottom: 17px;
  }
  .ud-profile {
    padding: 17px;
    flex-direction: column;
    gap: 16px;
  }
  .ud-identity {
    gap: 10px;
  }
  .ud-avatar {
    height: 45px;
    width: 45px;
    border-radius: 15px;
    font-size: 19px;
  }
  .ud-name {
    gap: 6px;
  }
  .ud-name h1 {
    font-size: 18px;
  }
  .ud-status {
    font-size: 8px;
    padding: 3px 5px;
  }
  .ud-identity-meta {
    font-size: 9px;
    gap: 8px;
  }
  .ud-identity-meta button {
    font-size: 11px;
  }
  .ud-handle-button {
    align-self: flex-end;
    font-size: 9px;
    padding: 8px 11px;
  }
  .ud-stat {
    padding: 15px 13px;
  }
  .ud-stat > span {
    font-size: 9px;
    gap: 5px;
  }
  .ud-stat > span > svg {
    width: 15px;
  }
  .ud-stat > b {
    font-size: 24px;
    white-space: nowrap;
  }
  .ud-stat > b small {
    font-size: 8px;
    margin: 0 3px;
  }
  .ud-stat > p {
    font-size: 8px;
    line-height: 1.8;
  }
  .ud-stats {
    gap: 9px;
    margin: 14px 0;
  }
  .ud-tabs {
    margin: 19px 0 16px;
  }
  .ud-tabs button {
    font-size: 10px;
    padding: 9px 10px;
  }
  .ud-tabs button svg {
    width: 14px;
  }
  .ud-filters {
    padding: 17px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .ud-filters .ud-related-filter {
    grid-column: span 2;
  }
  .ud-filter-actions {
    grid-column: span 2;
    justify-content: flex-start;
    margin-top: 1px;
  }
  .ud-filters input,
  .ud-filters select {
    font-size: 10px;
    padding: 9px 7px;
  }
  .ud-filters label {
    font-size: 9px;
  }
  .ud-card-title {
    padding: 18px 17px;
  }
  .ud-card-title h2 {
    font-size: 13px;
  }
  .ud-facts {
    padding: 2px 17px 10px;
    column-gap: 16px;
  }
  .ud-facts dd {
    font-size: 11px;
  }
  .ud-facts dt {
    font-size: 9px;
  }
  .ud-evidence-summary {
    padding: 0 16px;
  }
  .ud-evidence-summary > div {
    gap: 9px;
  }
  .ud-evidence-summary b {
    font-size: 10px;
  }
  .ud-evidence-summary p {
    font-size: 9px;
  }
  .ud-collection-status {
    padding: 13px 16px;
  }
  .ud-last-handling {
    margin-top: 16px;
  }
  .ud-handling-record {
    padding: 17px;
  }
  .ud-handling-record p {
    font-size: 11px;
  }
  .ud-handling-record > span {
    font-size: 9px;
  }
  .ud-availability-notes {
    padding: 12px;
    gap: 7px;
  }
  .ud-availability-notes p {
    font-size: 9px;
  }
  .ud-pagination {
    padding: 14px 16px;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  .ud-pagination > div {
    width: 100%;
    justify-content: flex-end;
  }
  .ud-pagination label {
    margin-right: auto;
  }
  .ud-pagination button {
    font-size: 9px;
    padding: 7px 9px;
  }
  .ud-pagination select {
    font-size: 9px;
    width: 59px;
  }
  .ud-record-note {
    font-size: 9px;
    padding: 11px 17px;
  }
  .ud-table-scroll th,
  .ud-table-scroll td {
    padding: 13px 16px;
  }
  .ud-timeline {
    padding: 22px 17px 3px;
  }
  .ud-timeline article {
    gap: 12px;
    padding-bottom: 23px;
  }
  .ud-timeline header {
    gap: 7px;
  }
  .ud-timeline h3 {
    font-size: 11px;
  }
  .ud-timeline p {
    font-size: 9px;
    gap: 8px;
  }
  .ud-timeline time {
    font-size: 9px;
  }
  .ud-record-detail {
    padding: 23px 21px;
  }
  .ud-record-detail h2 {
    font-size: 19px;
  }
  .ud-handling {
    padding: 16px;
    gap: 12px;
  }
  .ud-handling label,
  .ud-handling .ud-handling-reason {
    width: 100%;
    flex-basis: 100%;
    min-width: 0;
  }
  .ud-handling .primary {
    font-size: 10px;
  }
  .ud-record-detail dd {
    font-size: 11px;
  }
  .ud-risk-proof,
  .ud-reward-proof {
    padding: 16px;
  }
  .ud-reward-proof dl {
    grid-template-columns: 1fr;
  }
  .ud-proof-summary b {
    font-size: 17px;
  }
  .ud-proof-summary > span {
    font-size: 9px;
  }
}
</style>
