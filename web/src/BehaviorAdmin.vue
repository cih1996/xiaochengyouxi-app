<script setup>
import { ref, watch, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import BehaviorPolicy from "./BehaviorPolicy.vue";
import BehaviorProfile from "./BehaviorProfile.vue";
import { number, dateTime, label } from "./audit-format";
const props = defineProps({ api: Function, userId: Number });
const emit = defineEmits(["unauthorized", "detail", "relations"]);
const tab = ref(props.userId ? "profile" : "alerts"),
  activeUser = ref(props.userId || null),
  initialSession = ref(""),
  profilePanel = ref(null),
  policyPanel = ref(null);
const result = ref({ items: [], total: 0, page: 1, page_size: 20 }),
  filters = ref({ user_id: "", action: "", from: "", to: "" }),
  selectedAlert = ref(null),
  error = ref(""),
  loading = ref(false);
const search = ref(""),
  searchResults = ref([]),
  searched = ref(false);
let controller, searchController;
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function loadAlerts(page = 1) {
  controller?.abort();
  controller = new AbortController();
  loading.value = true;
  error.value = "";
  try {
    const query = new URLSearchParams({ page: String(page), page_size: "20" });
    for (const [key, value] of Object.entries(filters.value))
      if (value !== "")
        query.set(
          key,
          ["from", "to"].includes(key) ? new Date(value).toISOString() : value,
        );
    result.value = await props.api("/admin/behavior/alerts?" + query, {
      signal: controller.signal,
    });
  } catch (failure) {
    fail(failure);
  } finally {
    loading.value = false;
  }
}
async function findUsers() {
  searchController?.abort();
  searchController = new AbortController();
  error.value = "";
  try {
    const data = await props.api(
      "/admin/users/search?" +
        new URLSearchParams({
          q: search.value.trim(),
          page: "1",
          page_size: "10",
        }),
      { signal: searchController.signal },
    );
    searchResults.value = data.items;
    searched.value = true;
  } catch (failure) {
    fail(failure);
  }
}
function viewProfile(id, session = "") {
  activeUser.value = Number(id);
  initialSession.value = session;
  tab.value = "profile";
  searchResults.value = [];
  searched.value = false;
  selectedAlert.value = null;
}
async function refresh() {
  if (tab.value === "policy") return policyPanel.value?.refresh();
  if (tab.value === "profile" && activeUser.value)
    return profilePanel.value?.refresh();
  return loadAlerts(result.value.page);
}
watch(
  () => props.userId,
  (id) => {
    if (id) viewProfile(id);
  },
);
watch(tab, (value) => {
  error.value = "";
  if (value === "alerts") loadAlerts(1);
});
onMounted(() => {
  if (tab.value === "alerts") loadAlerts();
});
onUnmounted(() => {
  controller?.abort();
  searchController?.abort();
});
defineExpose({ refresh });
</script>
<template>
  <div class="behavior-admin">
    <nav class="behavior-admin-tabs" aria-label="行为审计分类">
      <button :class="{ active: tab === 'alerts' }" @click="tab = 'alerts'">
        <Icon name="bell" :size="16" />告警记录</button
      ><button :class="{ active: tab === 'profile' }" @click="tab = 'profile'">
        <Icon name="user" :size="16" />用户画像</button
      ><button :class="{ active: tab === 'policy' }" @click="tab = 'policy'">
        <Icon name="settings" :size="16" />审核策略
      </button>
    </nav>
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <template v-if="tab === 'alerts'"
      ><section class="admin-card behavior-alerts">
        <form @submit.prevent="loadAlerts(1)">
          <label
            >用户编号<input
              v-model="filters.user_id"
              aria-label="告警用户编号"
              type="number"
              min="1"
              placeholder="全部" /></label
          ><label
            >处置<select v-model="filters.action" aria-label="告警处置方式">
              <option value="">全部</option>
              <option value="observe">观察</option>
              <option value="review">待核查</option>
              <option value="hold">暂缓</option>
            </select></label
          ><label
            >开始时间<input
              v-model="filters.from"
              type="datetime-local"
              step="1"
              aria-label="告警开始时间" /></label
          ><label
            >结束时间<input
              v-model="filters.to"
              type="datetime-local"
              step="1"
              aria-label="告警结束时间" /></label
          ><button class="outline" :disabled="loading">
            <Icon name="search" :size="14" />查询
          </button>
        </form>
        <div class="behavior-alert-table">
          <table>
            <thead>
              <tr>
                <th>时间</th>
                <th>用户</th>
                <th>行为分 / 处置</th>
                <th>命中证据</th>
                <th>策略</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="alert in result.items" :key="alert.id">
                <td>{{ dateTime(alert.created_at) }}</td>
                <td>
                  <button @click="viewProfile(alert.user_id)">
                    用户 #{{ alert.user_id }}
                  </button>
                </td>
                <td>
                  <b>{{ alert.score }} 分</b
                  ><span class="behavior-alert-action" :class="alert.action">{{
                    label(alert.action)
                  }}</span>
                </td>
                <td>
                  <button
                    class="behavior-alert-evidence"
                    @click="selectedAlert = alert"
                  >
                    {{ alert.evidence.length }} 条<Icon
                      name="arrow"
                      :size="11"
                    />
                  </button>
                </td>
                <td>V{{ alert.policy_revision }}</td>
                <td>
                  <button
                    class="behavior-alert-replay"
                    @click="viewProfile(alert.user_id, alert.session_id)"
                  >
                    <Icon name="play" :size="12" />查看回放
                  </button>
                </td>
              </tr>
              <tr v-if="!result.items.length">
                <td colspan="6" class="behavior-alert-empty">
                  {{ loading ? "正在读取告警…" : "暂无告警记录" }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <footer>
          <span>共 {{ number(result.total) }} 条 · 第 {{ result.page }} 页</span
          ><button
            :disabled="result.page <= 1 || loading"
            @click="loadAlerts(result.page - 1)"
          >
            上一页</button
          ><button
            :disabled="result.page * 20 >= result.total || loading"
            @click="loadAlerts(result.page + 1)"
          >
            下一页
          </button>
        </footer>
      </section></template
    >
    <template v-if="tab === 'profile'"
      ><form
        class="admin-card behavior-user-search"
        @submit.prevent="findUsers"
      >
        <Icon name="search" :size="17" /><input
          v-model="search"
          aria-label="查找行为用户"
          placeholder="手机号、昵称或用户编号"
        /><button class="outline">查找用户</button>
        <div v-if="searched" class="behavior-user-results">
          <button
            v-for="user in searchResults"
            :key="user.id"
            type="button"
            @click="viewProfile(user.id)"
          >
            <b>{{ user.nickname }}</b
            ><span>{{ user.phone }}</span
            ><small>#{{ user.id }}</small>
          </button>
          <p v-if="!searchResults.length">没有找到用户</p>
        </div>
      </form>
      <BehaviorProfile
        v-if="activeUser"
        ref="profilePanel"
        :api="api"
        :user-id="activeUser"
        :initial-session="initialSession"
        @unauthorized="fail"
        @detail="(id) => emit('detail', id)"
        @relations="(id) => emit('relations', id)"
      />
      <div v-else class="behavior-select-user">
        <Icon name="user" :size="30" />选择一位用户查看画像
      </div></template
    >
    <BehaviorPolicy
      v-if="tab === 'policy'"
      ref="policyPanel"
      :api="api"
      @unauthorized="fail"
    />
    <div
      v-if="selectedAlert"
      class="behavior-evidence-overlay"
      @click.self="selectedAlert = null"
    >
      <section
        class="behavior-evidence-panel"
        role="dialog"
        aria-modal="true"
        aria-label="告警证据"
      >
        <header>
          <h2>告警证据</h2>
          <button aria-label="关闭告警证据" @click="selectedAlert = null">
            <Icon name="close" :size="20" />
          </button>
        </header>
        <div class="behavior-evidence-summary">
          <b>{{ selectedAlert.score }}<small>分</small></b
          ><span
            >{{ label(selectedAlert.action) }} · 策略 V{{
              selectedAlert.policy_revision
            }}</span
          >
        </div>
        <article
          v-for="evidence in selectedAlert.evidence"
          :key="evidence.code"
        >
          <header>
            <h3>{{ evidence.label }}</h3>
            <b>+{{ evidence.points }}</b>
          </header>
          <dl>
            <div>
              <dt>命中值</dt>
              <dd>{{ evidence.observed }}</dd>
            </div>
            <div>
              <dt>阈值</dt>
              <dd>{{ evidence.threshold }}</dd>
            </div>
          </dl>
        </article>
        <footer>
          <span>{{ dateTime(selectedAlert.created_at) }}</span
          ><button
            @click="
              viewProfile(selectedAlert.user_id, selectedAlert.session_id)
            "
          >
            查看回放<Icon name="arrow" :size="13" />
          </button>
        </footer>
      </section>
    </div>
  </div>
</template>
<style scoped>
.behavior-admin {
  max-width: 1400px;
}
.behavior-admin-tabs {
  display: flex;
  align-items: center;
  gap: 5px;
  border-bottom: 1px solid #e9dcc8;
  margin-bottom: 22px;
  padding-bottom: 13px;
}
.behavior-admin-tabs > button {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 10px 15px;
  border-radius: 9px;
  font-size: 11px;
  color: #b39876;
}
.behavior-admin-tabs > button.active {
  background: #fff0dc;
  color: #c2935d;
}
.behavior-alerts > form {
  display: flex;
  gap: 15px;
  align-items: flex-end;
  flex-wrap: wrap;
  padding: 22px;
}
.behavior-alerts label {
  font-size: 9px;
  font-weight: 400;
  color: #b19978;
  margin: 0;
  flex: 1;
  min-width: 110px;
}
.behavior-alerts input,
.behavior-alerts select {
  padding: 10px;
  font-size: 10px;
  border-radius: 8px;
  margin-top: 7px;
  color: #a5845e;
  background: #fffaf2;
}
.behavior-alerts form > .outline {
  padding: 10px 13px;
  font-size: 10px;
  border-radius: 8px;
  height: 37px;
}
.behavior-alert-table {
  overflow-x: auto;
}
.behavior-alert-table table {
  width: 100%;
  border-collapse: collapse;
  font-size: 10px;
  white-space: nowrap;
}
.behavior-alert-table th {
  background: #fcf5e9;
  color: #b39871;
  font-size: 9px;
  font-weight: 500;
  padding: 13px 21px;
  text-align: left;
}
.behavior-alert-table td {
  padding: 17px 21px;
  color: #a78962;
  border-top: 1px solid #f0e2cd;
}
.behavior-alert-table td > b {
  font-weight: 500;
}
.behavior-alert-table td > button {
  font-size: 10px;
  color: #b78d5c;
}
.behavior-alert-action {
  padding: 4px 7px;
  border-radius: 5px;
  background: #f4ecd9;
  color: #ae946d;
  font-size: 8px;
  margin-left: 8px;
}
.behavior-alert-action.review {
  background: #fff0d2;
  color: #c59a50;
}
.behavior-alert-action.hold {
  background: #f8e5d6;
  color: #c18b67;
}
.behavior-alert-evidence,
.behavior-alert-replay {
  display: flex;
  gap: 5px;
  align-items: center;
  font-size: 10px;
}
.behavior-alert-empty {
  height: 180px;
  text-align: center;
}
.behavior-alerts > footer {
  display: flex;
  justify-content: flex-end;
  gap: 18px;
  padding: 18px 22px;
  border-top: 1px solid #ecdec6;
  color: #b79a72;
  font-size: 10px;
}
.behavior-alerts > footer > span {
  margin-right: auto;
}
.behavior-user-search {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 19px;
  margin-bottom: 21px;
  position: relative;
}
.behavior-user-search > svg {
  color: #c6a37c;
}
.behavior-user-search > input {
  flex: 1;
  min-width: 0;
  padding: 10px;
  font-size: 11px;
  border-radius: 8px;
}
.behavior-user-search > .outline {
  padding: 10px 13px;
  font-size: 10px;
  border-radius: 8px;
  white-space: nowrap;
}
.behavior-user-results {
  position: absolute;
  top: 64px;
  left: 20px;
  right: 20px;
  z-index: 22;
  background: #fffdf7;
  border: 1px solid #e7d5b9;
  border-radius: 10px;
  box-shadow: 0 10px 25px #8d60321a;
  max-height: 360px;
  overflow: auto;
}
.behavior-user-results > button {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 12px;
  padding: 14px;
  border-bottom: 1px solid #f1e4d1;
  text-align: left;
  font-size: 10px;
}
.behavior-user-results b {
  font-weight: 500;
  color: #a17b4e;
}
.behavior-user-results span {
  color: #b69b76;
}
.behavior-user-results small {
  margin-left: auto;
  color: #c3ab89;
  font-size: 9px;
}
.behavior-user-results p {
  padding: 22px;
  text-align: center;
  color: #b69e7b;
  font-size: 11px;
}
.behavior-select-user {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 17px;
  padding: 95px 15px;
  color: #bda27d;
  font-size: 12px;
}
.behavior-evidence-overlay {
  position: fixed;
  inset: 0;
  background: #503b2c70;
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  z-index: 85;
}
.behavior-evidence-panel {
  width: 540px;
  max-width: 100%;
  max-height: 90vh;
  overflow: auto;
  background: #fffaf1;
  border: 1px solid #e8d6b8;
  border-radius: 17px;
  padding: 25px;
}
.behavior-evidence-panel > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.behavior-evidence-panel h2 {
  font-size: 19px;
  color: #956e42;
}
.behavior-evidence-panel > header > button {
  color: #b89a70;
}
.behavior-evidence-summary {
  display: flex;
  align-items: center;
  gap: 19px;
  margin: 25px 0;
}
.behavior-evidence-summary > b {
  font-size: 33px;
  color: #bd8c51;
  font-weight: 500;
}
.behavior-evidence-summary small {
  font-size: 10px;
  margin-left: 6px;
  color: #baa179;
}
.behavior-evidence-summary > span {
  font-size: 10px;
  color: #b2956c;
}
.behavior-evidence-panel > article {
  border: 1px solid #edddc3;
  border-radius: 10px;
  padding: 17px;
  margin-bottom: 12px;
  background: #fffdf8;
}
.behavior-evidence-panel article > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.behavior-evidence-panel h3 {
  font-size: 12px;
  color: #a78354;
  font-weight: 500;
}
.behavior-evidence-panel article > header > b {
  font-size: 11px;
  color: #c49a5f;
  font-weight: 500;
}
.behavior-evidence-panel dl {
  display: flex;
  gap: 30px;
  margin: 15px 0 0;
}
.behavior-evidence-panel dl > div {
  display: flex;
  gap: 10px;
  font-size: 10px;
}
.behavior-evidence-panel dt {
  color: #bda784;
}
.behavior-evidence-panel dd {
  margin: 0;
  color: #ad8c60;
  overflow-wrap: anywhere;
}
.behavior-evidence-panel > footer {
  margin-top: 23px;
  padding-top: 18px;
  border-top: 1px solid #eddfc6;
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 9px;
  color: #bba27b;
}
.behavior-evidence-panel footer > button {
  display: flex;
  gap: 5px;
  align-items: center;
  font-size: 10px;
  color: #bd935c;
}
@media (max-width: 650px) {
  .behavior-admin-tabs {
    gap: 1px;
  }
  .behavior-admin-tabs > button {
    flex: 1;
    font-size: 10px;
    padding: 10px 7px;
    gap: 6px;
  }
  .behavior-alerts > form {
    padding: 17px 15px;
    gap: 12px;
  }
  .behavior-alerts label {
    flex-basis: calc(50% - 12px);
    min-width: 0;
  }
  .behavior-alerts input,
  .behavior-alerts select {
    font-size: 9px;
    padding: 9px 5px;
  }
  .behavior-alerts form > .outline {
    margin-left: auto;
  }
  .behavior-alert-table th,
  .behavior-alert-table td {
    padding: 15px;
  }
  .behavior-user-search {
    padding: 14px 12px;
    gap: 8px;
  }
  .behavior-user-search > .outline {
    font-size: 9px;
    padding: 10px;
  }
  .behavior-user-search > input {
    font-size: 10px;
  }
  .behavior-evidence-overlay {
    padding: 16px;
  }
  .behavior-evidence-panel {
    padding: 21px 18px;
  }
}
</style>
