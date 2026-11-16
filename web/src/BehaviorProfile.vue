<script setup>
import { ref, computed, watch, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import RiskOverview from "./RiskOverview.vue";
import BehaviorReplay from "./BehaviorReplay.vue";
import UserChart from "./UserChart.vue";
import { number, dateTime, duration, label } from "./audit-format";
const props = defineProps({
  api: Function,
  userId: Number,
  initialSession: String,
});
const emit = defineEmits(["unauthorized", "detail", "relations"]);
const profile = ref(null),
  sessions = ref({ items: [], total: 0, page: 1, page_size: 20 }),
  days = ref(30),
  error = ref(""),
  loading = ref(false),
  replay = ref("");
let controller;
const dwellMax = computed(() =>
  Math.max(1, ...(profile.value?.pages || []).map((page) => page.dwell_ms)),
);
const clickMax = computed(() =>
  Math.max(1, ...(profile.value?.pages || []).map((page) => page.clicks)),
);
const intervalMax = computed(() =>
  Math.max(
    1,
    ...(profile.value?.click_intervals?.buckets || []).map(
      (item) => item.count,
    ),
  ),
);
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
async function refresh() {
  controller?.abort();
  controller = new AbortController();
  loading.value = true;
  profile.value = null;
  error.value = "";
  try {
    const values = await Promise.all([
      props.api(
        `/admin/users/${props.userId}/behavior/profile?days=${days.value}`,
        { signal: controller.signal },
      ),
      props.api(
        `/admin/users/${props.userId}/behavior/sessions?page=1&page_size=20`,
        { signal: controller.signal },
      ),
    ]);
    profile.value = values[0];
    sessions.value = values[1];
    if (props.initialSession) replay.value = props.initialSession;
  } catch (failure) {
    fail(failure);
  } finally {
    loading.value = false;
  }
}
async function loadSessions(page) {
  error.value = "";
  try {
    sessions.value = await props.api(
      `/admin/users/${props.userId}/behavior/sessions?page=${page}&page_size=20`,
      { signal: controller.signal },
    );
  } catch (failure) {
    fail(failure);
  }
}
watch(
  () => props.userId,
  () => {
    replay.value = "";
    refresh();
  },
  { immediate: true },
);
watch(
  () => props.initialSession,
  (value) => {
    if (value) replay.value = value;
  },
);
onUnmounted(() => controller?.abort());
defineExpose({ refresh });
</script>
<template>
  <div class="behavior-profile">
    <div class="behavior-profile-toolbar">
      <b>用户 #{{ userId }}</b>
      <div class="behavior-profile-days">
        <button
          v-for="value in [7, 30, 90]"
          :key="value"
          :class="{ active: days === value }"
          @click="
            days = value;
            refresh();
          "
        >
          近{{ value }}天
        </button>
      </div>
      <button @click="emit('relations', userId)">
        <Icon name="relation" :size="15" />关系图</button
      ><button @click="emit('detail', userId)">
        完整明细<Icon name="arrow" :size="13" />
      </button>
    </div>
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="loading" class="behavior-profile-loading">正在读取行为画像…</div>
    <template v-if="profile"
      ><section class="admin-card behavior-collection">
        <span :class="profile.collection.status">{{
          label(profile.collection.status)
        }}</span>
        <div>
          <b>采集开始 {{ dateTime(profile.collection.since) }}</b
          ><small
            >最近记录 {{ dateTime(profile.collection.last_seen_at) }}</small
          >
        </div>
        <span>{{ number(profile.collection.session_count) }} 个会话</span>
      </section>
      <div
        v-for="note in [
          ...profile.collection.missing,
          ...profile.collection.notes,
        ]"
        :key="note"
        class="behavior-profile-note"
      >
        <Icon name="bell" :size="13" />{{ note }}
      </div>
      <div class="behavior-profile-metrics">
        <article class="admin-card">
          <span>前台停留</span
          ><b>{{ duration(profile.summary.foreground_ms) }}</b>
        </article>
        <article class="admin-card">
          <span>点击 / 手势</span
          ><b
            >{{ number(profile.summary.click_count)
            }}<small> / {{ number(profile.summary.gesture_count) }}</small></b
          >
        </article>
        <article class="admin-card">
          <span>已核对到账金币</span
          ><b>{{ number(profile.summary.claimed_coins) }}</b>
        </article>
        <article class="admin-card">
          <span>业务未匹配</span
          ><b
            >{{ number(profile.summary.unmatched_business_count)
            }}<small>次</small></b
          >
        </article>
      </div>
      <div class="behavior-profile-main">
        <div class="behavior-profile-distributions">
          <section class="admin-card behavior-page-bars">
            <h3>页面停留</h3>
            <div
              v-for="page in profile.pages"
              :key="page.screen_code"
              class="behavior-bar-row"
            >
              <span>{{ label(page.screen) }}</span>
              <div>
                <i
                  :style="{ width: (page.dwell_ms / dwellMax) * 100 + '%' }"
                ></i>
              </div>
              <b>{{ duration(page.dwell_ms) }}</b>
            </div>
            <p v-if="!profile.pages.length">暂无页面停留记录</p>
          </section>
          <section class="admin-card behavior-page-bars green">
            <h3>页面点击分布</h3>
            <div
              v-for="page in profile.pages"
              :key="page.screen_code"
              class="behavior-bar-row"
            >
              <span>{{ label(page.screen) }}</span>
              <div>
                <i :style="{ width: (page.clicks / clickMax) * 100 + '%' }"></i>
              </div>
              <b>{{ number(page.clicks) }}</b>
            </div>
            <p v-if="!profile.pages.length">暂无页面点击记录</p>
          </section>
        </div>
        <RiskOverview :risk="profile.risk" />
      </div>
      <section class="admin-card behavior-intervals">
        <header>
          <h3>点击间隔</h3>
          <span v-if="profile.click_intervals"
            >{{ number(profile.click_intervals.sample_count) }} 个样本</span
          >
        </header>
        <template v-if="profile.click_intervals?.sample_count"
          ><div class="behavior-interval-metrics">
            <span
              >中位数
              <b>{{ number(profile.click_intervals.median_ms) }} ms</b></span
            ><span
              >95分位
              <b>{{ number(profile.click_intervals.p95_ms) }} ms</b></span
            >
          </div>
          <div class="behavior-interval-bars">
            <div
              v-for="bucket in profile.click_intervals.buckets"
              :key="bucket.label"
            >
              <span>{{ number(bucket.count) }}</span><i
                :style="{
                  height: Math.max(2, (bucket.count / intervalMax) * 90) + 'px',
                }"
              ></i
              ><b>{{ bucket.label }}</b>
            </div>
          </div></template
        >
        <p v-else>暂无点击间隔样本</p>
      </section>
      <div class="behavior-profile-trends">
        <UserChart
          :data="profile.daily"
          metric="click_count"
          title="点击趋势"
          type="bar"
          unit="次"
          empty="所选时段暂无点击记录"
        /><UserChart
          :data="profile.daily"
          metric="reward_coins"
          title="已核对金币趋势"
          color="green"
          unit="金币"
          empty="所选时段暂无已核对金币"
        />
      </div>
      <section class="admin-card behavior-sessions">
        <header>
          <h3>操作会话</h3>
          <span>{{ number(sessions.total) }} 条</span>
        </header>
        <div class="behavior-sessions-table">
          <table>
            <thead>
              <tr>
                <th>开始时间</th>
                <th>设备</th>
                <th>停留 / 点击</th>
                <th>事件 / 缺失</th>
                <th>行为分</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="session in sessions.items" :key="session.id">
                <td>
                  {{ dateTime(session.started_at)
                  }}<small>{{ label(session.status) }}</small>
                </td>
                <td>
                  <button :title="session.device_id">
                    {{
                      session.device_id?.slice(0, 16) || "未关联"
                    }}</button><small>{{ label(session.proof_level) }}</small>
                </td>
                <td>
                  {{ duration(session.foreground_ms) }} /
                  {{ number(session.click_count) }}
                </td>
                <td>
                  {{ number(session.event_count) }} /
                  {{ number(session.dropped_events) }}
                </td>
                <td>
                  {{
                    session.behavior_score == null
                      ? "待评估"
                      : session.behavior_score + "分"
                  }}
                </td>
                <td>
                  <button
                    class="behavior-replay-open"
                    @click="replay = session.id"
                  >
                    <Icon name="play" :size="13" />回放
                  </button>
                </td>
              </tr>
              <tr v-if="!sessions.items.length">
                <td colspan="6" class="behavior-sessions-empty">
                  暂无可回放会话
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <footer>
          <span>第 {{ sessions.page }} 页</span
          ><button
            :disabled="sessions.page <= 1"
            @click="loadSessions(sessions.page - 1)"
          >
            上一页</button
          ><button
            :disabled="sessions.page * 20 >= sessions.total"
            @click="loadSessions(sessions.page + 1)"
          >
            下一页
          </button>
        </footer>
      </section>
      <BehaviorReplay
        v-if="replay"
        :api="api"
        :user-id="userId"
        :session-id="replay"
        @close="replay = ''"
        @unauthorized="(failure) => emit('unauthorized', failure)"
      />
    </template>
  </div>
</template>
<style scoped>
.behavior-profile-toolbar {
  display: flex;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
  margin: 0 0 20px;
}
.behavior-profile-toolbar > b {
  font-size: 12px;
  color: #a17a53;
}
.behavior-profile-toolbar > button {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: #b08a62;
}
.behavior-profile-days {
  display: flex;
  background: #f7eddf;
  border-radius: 8px;
  padding: 3px;
  gap: 3px;
  margin-right: auto;
}
.behavior-profile-days > button {
  font-size: 10px;
  padding: 8px 11px;
  border-radius: 6px;
  color: #b39879;
}
.behavior-profile-days > button.active {
  background: #fffaf2;
  color: #c28c58;
}
.behavior-collection {
  display: flex;
  gap: 17px;
  align-items: center;
  padding: 17px 20px;
  margin-bottom: 18px;
}
.behavior-collection > span:first-child {
  padding: 6px 9px;
  background: #f5eddc;
  color: #b3976f;
  border-radius: 7px;
  font-size: 10px;
}
.behavior-collection > span.available {
  background: #edf4e9;
  color: #92a77b;
}
.behavior-collection > div {
  flex: 1;
}
.behavior-collection b {
  font-size: 10px;
  font-weight: 400;
  color: #a68865;
}
.behavior-collection small {
  font-size: 9px;
  color: #bba283;
  display: block;
  margin-top: 6px;
}
.behavior-collection > span:last-child {
  font-size: 10px;
  color: #b09776;
}
.behavior-profile-note {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  color: #b99c76;
  font-size: 10px;
  line-height: 1.8;
  margin: 10px 2px;
}
.behavior-profile-metrics {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin: 20px 0;
}
.behavior-profile-metrics article {
  padding: 21px;
}
.behavior-profile-metrics span {
  display: block;
  font-size: 10px;
  color: #b19675;
  margin-bottom: 16px;
}
.behavior-profile-metrics b {
  font-size: 25px;
  font-weight: 500;
  color: #9e784d;
  letter-spacing: -0.8px;
}
.behavior-profile-metrics small {
  font-size: 10px;
  font-weight: 400;
  color: #b59b79;
  letter-spacing: 0;
  margin-left: 4px;
}
.behavior-profile-main {
  display: grid;
  grid-template-columns: 1.5fr 1fr;
  gap: 20px;
  margin-bottom: 20px;
  align-items: start;
}
.behavior-profile-distributions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 18px;
}
.behavior-page-bars {
  padding: 22px 19px;
  --bar: #e1b585;
}
.behavior-page-bars.green {
  --bar: #9bb79f;
}
.behavior-profile h3 {
  font-size: 13px;
  color: #95714a;
  font-weight: 600;
}
.behavior-page-bars h3 {
  margin-bottom: 22px;
}
.behavior-bar-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 17px 0;
  font-size: 9px;
}
.behavior-bar-row > span {
  width: 53px;
  flex-shrink: 0;
  color: #b29775;
}
.behavior-bar-row > div {
  flex: 1;
  height: 6px;
  background: #f7efdf;
  border-radius: 10px;
  overflow: hidden;
}
.behavior-bar-row i {
  display: block;
  height: 100%;
  background: var(--bar);
  border-radius: 10px;
}
.behavior-bar-row > b {
  width: 41px;
  font-size: 9px;
  text-align: right;
  color: #a88b65;
  font-weight: 400;
}
.behavior-page-bars p,
.behavior-intervals > p {
  padding: 34px 0;
  text-align: center;
  font-size: 10px;
  color: #c0a786;
}
.behavior-intervals {
  padding: 22px;
  margin-bottom: 20px;
}
.behavior-intervals header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.behavior-intervals header > span {
  font-size: 9px;
  color: #bba17e;
}
.behavior-interval-metrics {
  display: flex;
  gap: 27px;
  font-size: 10px;
  color: #baa07b;
  margin-top: 20px;
}
.behavior-interval-metrics b {
  font-weight: 500;
  color: #a58153;
  margin-left: 6px;
}
.behavior-interval-bars {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  margin-top: 25px;
  min-height: 128px;
}
.behavior-interval-bars > div {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  gap: 9px;
  min-width: 0;
  font-size: 9px;
  color: #bba17e;
}
.behavior-interval-bars i {
  width: 60%;
  max-width: 60px;
  background: #dcb88a;
  border-radius: 4px 4px 0 0;
}
.behavior-interval-bars b {
  font-size: 8px;
  font-weight: 400;
  text-align: center;
}
.behavior-profile-trends {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  margin-bottom: 20px;
}
.behavior-sessions > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 21px 23px;
}
.behavior-sessions > header > span {
  font-size: 10px;
  color: #b89e79;
}
.behavior-sessions-table {
  overflow-x: auto;
}
.behavior-sessions table {
  border-collapse: collapse;
  width: 100%;
  font-size: 10px;
  white-space: nowrap;
}
.behavior-sessions th {
  padding: 13px 20px;
  background: #fbf5e9;
  font-size: 9px;
  font-weight: 500;
  text-align: left;
  color: #b59a74;
}
.behavior-sessions td {
  padding: 16px 20px;
  border-top: 1px solid #f0e3d0;
  color: #ab8a62;
}
.behavior-sessions td > small {
  display: block;
  font-size: 8px;
  color: #c0a786;
  margin-top: 6px;
}
.behavior-sessions td > button {
  font: 9px monospace;
}
.behavior-replay-open {
  display: flex;
  align-items: center;
  gap: 5px;
  font: 10px inherit !important;
  color: #c5965c;
}
.behavior-sessions-empty {
  height: 130px;
  text-align: center;
}
.behavior-sessions footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 17px;
  padding: 17px 21px;
  border-top: 1px solid #eee0cb;
  font-size: 10px;
  color: #b99d76;
}
.behavior-sessions footer > span {
  margin-right: auto;
}
.behavior-profile-loading {
  padding: 80px 15px;
  text-align: center;
  color: #bca17e;
  font-size: 12px;
}
@media (max-width: 1100px) {
  .behavior-profile-main {
    grid-template-columns: 1fr;
  }
  .behavior-profile-main > .risk-overview {
    max-width: none;
  }
  .behavior-profile-trends {
    grid-template-columns: 1fr;
  }
}
@media (max-width: 650px) {
  .behavior-profile-toolbar {
    gap: 13px;
  }
  .behavior-profile-toolbar > b {
    font-size: 11px;
  }
  .behavior-profile-days {
    order: 3;
    width: 100%;
  }
  .behavior-profile-days > button {
    flex: 1;
  }
  .behavior-collection {
    padding: 15px;
    gap: 10px;
    flex-wrap: wrap;
  }
  .behavior-collection > div {
    min-width: 170px;
  }
  .behavior-collection > span:last-child {
    margin-left: auto;
  }
  .behavior-profile-metrics {
    grid-template-columns: 1fr 1fr;
    gap: 11px;
  }
  .behavior-profile-metrics article {
    padding: 18px 15px;
  }
  .behavior-profile-metrics b {
    font-size: 23px;
  }
  .behavior-profile-metrics span {
    font-size: 9px;
  }
  .behavior-profile-distributions {
    grid-template-columns: 1fr;
  }
  .behavior-page-bars {
    padding: 21px;
  }
  .behavior-intervals {
    padding: 20px 16px;
  }
  .behavior-interval-bars {
    gap: 6px;
  }
  .behavior-interval-bars b {
    font-size: 7px;
  }
  .behavior-sessions > header {
    padding: 19px 16px;
  }
  .behavior-sessions th,
  .behavior-sessions td {
    padding: 13px 16px;
  }
}
</style>
