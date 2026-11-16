<script setup>
import { ref, computed, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import UserChart from "./UserChart.vue";
const props = defineProps({ api: Function, view: String });
const emit = defineEmits(["unauthorized", "users"]);
const stats = ref(null),
  loading = ref(false),
  error = ref(""),
  days = ref(30);
const today = new Date(Date.now() + 8 * 3600000).toISOString().slice(0, 10);
const startDate = ref(
    new Date(Date.now() + 8 * 3600000 - 29 * 86400000)
      .toISOString()
      .slice(0, 10),
  ),
  endDate = ref(today);
let controller;
const number = (value) => Number(value).toLocaleString("zh-CN");
const cards = computed(() =>
  props.view === "registrations"
    ? [
        {
          label: "今日注册",
          key: "today_registrations",
          icon: "user",
          warm: true,
        },
        { label: "昨日注册", key: "yesterday_registrations", icon: "calendar" },
        { label: "区间注册", key: "range_registrations", icon: "spark" },
        { label: "累计用户", key: "total_users", icon: "grid" },
      ]
    : props.view === "activity"
      ? [
          {
            label: "今日活跃",
            key: "today_active_users",
            icon: "spark",
            warm: true,
          },
          { label: "近7日活跃", key: "active_users_7d", icon: "calendar" },
          { label: "近30日活跃", key: "active_users_30d", icon: "user" },
          { label: "区间活跃", key: "range_active_users", icon: "grid" },
        ]
      : [
          { label: "累计用户", key: "total_users", icon: "user", warm: true },
          { label: "今日活跃", key: "today_active_users", icon: "spark" },
          { label: "近7日活跃", key: "active_users_7d", icon: "calendar" },
          { label: "近30日活跃", key: "active_users_30d", icon: "grid" },
          { label: "今日注册", key: "today_registrations", icon: "user" },
          { label: "区间注册", key: "range_registrations", icon: "calendar" },
        ],
);
const distributionMax = computed(() =>
  stats.value
    ? Math.max(1, ...stats.value.activity_days.map((item) => item.users))
    : 1,
);
const recent = computed(() =>
  stats.value ? stats.value.daily.slice(-7).reverse() : [],
);
async function refresh() {
  controller?.abort();
  const current = new AbortController();
  controller = current;
  loading.value = true;
  error.value = "";
  stats.value = null;
  try {
    const query = days.value
      ? new URLSearchParams({ days: String(days.value) })
      : new URLSearchParams({
          start_date: startDate.value,
          end_date: endDate.value,
        });
    const result = await props.api("/admin/user-center/stats?" + query, {
      signal: current.signal,
    });
    stats.value = result;
    startDate.value = result.selected_range.start_date;
    endDate.value = result.selected_range.end_date;
  } catch (failure) {
    if (failure.name === "AbortError") return;
    error.value = failure.message;
    if (failure.status === 401) emit("unauthorized", failure);
  } finally {
    if (controller === current) loading.value = false;
  }
}
function preset(value) {
  days.value = value;
  refresh();
}
onMounted(refresh);
onUnmounted(() => controller?.abort());
defineExpose({ refresh });
</script>
<template>
  <div class="user-center">
    <section class="admin-card user-center-filters">
      <div class="user-center-range">
        <Icon name="calendar" :size="19" />
        <div>
          <b>统计时段</b
          ><span
            >{{
              stats
                ? `${stats.selected_range.start_date} 至 ${stats.selected_range.end_date}`
                : "选择日期查看"
            }}<small>北京时间</small></span
          >
        </div>
      </div>
      <div class="user-center-presets">
        <button
          v-for="value in [7, 30, 90]"
          :key="value"
          :class="{ active: days === value }"
          @click="preset(value)"
        >
          近{{ value }}天
        </button>
      </div>
      <form @submit.prevent="refresh">
        <label
          >开始日期<input
            v-model="startDate"
            aria-label="统计开始日期"
            type="date"
            :max="today"
            required
            @input="days = null" /></label
        ><span>至</span
        ><label
          >结束日期<input
            v-model="endDate"
            aria-label="统计结束日期"
            type="date"
            :max="today"
            required
            @input="days = null" /></label
        ><button class="outline" :disabled="loading">
          <Icon name="search" :size="14" />查询
        </button>
      </form>
    </section>
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <div v-if="loading" class="user-center-loading">
      <Icon name="refresh" :size="19" />正在读取用户数据…
    </div>
    <template v-if="stats"
      ><div class="user-center-metrics" :class="{ six: cards.length === 6 }">
        <article
          v-for="card in cards"
          :key="card.key"
          class="admin-card"
          :class="{ warm: card.warm }"
        >
          <div>
            <span>{{ card.label }}</span
            ><Icon :name="card.icon" :size="19" />
          </div>
          <b>{{ number(stats.summary[card.key]) }}<small>人</small></b
          ><span
            v-if="card.key.startsWith('range_')"
            class="user-center-metric-caption"
            >所选时段</span
          ><span
            v-else-if="card.key === 'total_users'"
            class="user-center-metric-caption"
            >截至当前</span
          ><span v-else class="user-center-metric-caption">去重用户数</span>
        </article>
      </div>
      <div class="user-center-charts">
        <UserChart
          v-if="view !== 'activity'"
          :data="stats.daily"
          metric="registrations"
          title="每日注册"
          type="bar"
          empty="所选时段暂无新增注册"
        />
        <UserChart
          v-if="view === 'registrations'"
          :data="stats.daily"
          metric="total_users"
          title="累计用户趋势"
          empty="所选时段暂无用户"
        />
        <UserChart
          v-if="view !== 'registrations'"
          :data="stats.daily"
          metric="active_users"
          title="每日活跃"
          color="green"
          empty="所选时段暂无活跃记录"
        />
        <section
          v-if="view === 'activity'"
          class="admin-card user-center-distribution"
        >
          <header>
            <h2>活跃天数分布</h2>
            <span :title="stats.activity_definition" tabindex="0"
              ><Icon name="bell" :size="15"
            /></span>
          </header>
          <div class="distribution-summary">
            <b
              >{{ number(stats.summary.range_active_users)
              }}<small>位活跃用户</small></b
            ><span>所选时段</span>
          </div>
          <div
            v-for="item in stats.activity_days"
            :key="item.key"
            class="distribution-row"
          >
            <span>{{ item.label }}</span>
            <div>
              <i
                :style="{ width: `${(item.users / distributionMax) * 100}%` }"
              ></i>
            </div>
            <b>{{ number(item.users) }}</b>
          </div>
          <footer>
            区间未活跃
            <b>{{ number(stats.summary.range_inactive_users) }}</b> 人<span
              :title="stats.metadata.inactive_definition"
              tabindex="0"
              >ⓘ</span
            >
          </footer>
        </section>
      </div>
      <section class="admin-card user-center-daily">
        <header>
          <div>
            <h2>区间最近 {{ recent.length }} 天</h2>
            <span :title="stats.activity_definition" tabindex="0"
              >统计口径 ⓘ</span
            >
          </div>
          <button @click="$emit('users')">
            查看用户列表<Icon name="arrow" :size="14" />
          </button>
        </header>
        <div class="user-center-table-wrap">
          <table>
            <thead>
              <tr>
                <th>日期</th>
                <th>新增注册</th>
                <th>活跃用户</th>
                <th>
                  老用户活跃<span
                    :title="stats.metadata.returning_active_definition"
                    tabindex="0"
                  >
                    ⓘ</span
                  >
                </th>
                <th>累计用户</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="day in recent" :key="day.date">
                <td>{{ day.date }}</td>
                <td>{{ number(day.registrations) }}</td>
                <td>{{ number(day.active_users) }}</td>
                <td>{{ number(day.returning_active) }}</td>
                <td>{{ number(day.total_users) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
      <p class="user-center-footnote">
        {{ stats.metadata.history_note
        }}<span :title="stats.activity_definition" tabindex="0"
          >活跃口径 ⓘ</span
        >
      </p>
    </template>
  </div>
</template>
<style scoped>
.user-center {
  max-width: 1280px;
}
.user-center-filters {
  padding: 20px 23px;
  display: flex;
  align-items: center;
  gap: 23px;
  flex-wrap: wrap;
  margin-bottom: 23px;
}
.user-center-range {
  display: flex;
  align-items: center;
  gap: 11px;
  min-width: 225px;
}
.user-center-range > svg {
  color: #cba076;
}
.user-center-range b {
  font-size: 12px;
  color: #8f6c4b;
}
.user-center-range span {
  display: block;
  font-size: 10px;
  color: #b3987e;
  margin-top: 6px;
}
.user-center-range small {
  font-size: 8px;
  margin-left: 8px;
  color: #c2aa91;
}
.user-center-presets {
  display: flex;
  gap: 4px;
  background: #faf1e6;
  padding: 4px;
  border-radius: 9px;
}
.user-center-presets button {
  padding: 8px 11px;
  color: #b0967c;
  font-size: 10px;
  border-radius: 6px;
  white-space: nowrap;
}
.user-center-presets button.active {
  background: #fffdfa;
  color: #ce8e55;
  box-shadow: 0 2px 5px #a77a4220;
}
.user-center-filters form {
  display: flex;
  align-items: flex-end;
  gap: 9px;
  margin-left: auto;
}
.user-center-filters label {
  font-size: 9px;
  color: #b3987e;
  font-weight: 400;
  margin: 0;
}
.user-center-filters input {
  margin-top: 5px;
  font-size: 10px;
  width: 132px;
  border-radius: 8px;
  padding: 9px;
  color: #997557;
  background: #fffcf7;
}
.user-center-filters form > span {
  font-size: 9px;
  color: #bea78f;
  padding-bottom: 11px;
}
.user-center-filters .outline {
  padding: 9px 11px;
  font-size: 10px;
  border-radius: 8px;
  height: 35px;
  white-space: nowrap;
}
.user-center-loading {
  padding: 75px 20px;
  display: flex;
  justify-content: center;
  gap: 10px;
  align-items: center;
  font-size: 12px;
  color: #b49b81;
}
.user-center-metrics {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 17px;
  margin-bottom: 23px;
}
.user-center-metrics.six {
  grid-template-columns: repeat(3, 1fr);
}
.user-center-metrics article {
  padding: 21px 23px;
}
.user-center-metrics article > div {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  color: #b19476;
  font-size: 11px;
}
.user-center-metrics article > div > svg {
  color: #c6a27d;
}
.user-center-metrics article > b {
  font-size: 32px;
  font-weight: 600;
  letter-spacing: -1px;
  display: block;
  margin: 15px 0 11px;
  color: #765136;
}
.user-center-metrics article > b small {
  font-size: 9px;
  font-weight: 400;
  margin-left: 6px;
  letter-spacing: 0;
  color: #b69574;
}
.user-center-metric-caption {
  font-size: 9px;
  color: #c2ab92;
}
.user-center-metrics .warm {
  background: linear-gradient(125deg, #ffad70, #fa8455);
  border-color: transparent;
  box-shadow: 0 7px 18px #ed995c18;
}
.user-center-metrics .warm > div,
.user-center-metrics .warm > div > svg,
.user-center-metrics .warm > b,
.user-center-metrics .warm > b small,
.user-center-metrics .warm > .user-center-metric-caption {
  color: #fff9ef;
}
.user-center-charts {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 23px;
  margin-bottom: 23px;
}
.user-center-distribution {
  padding: 24px;
}
.user-center-distribution header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.user-center h2 {
  font-size: 14px;
  color: #79583b;
}
.user-center-distribution header > span {
  color: #c3a484;
  cursor: help;
}
.distribution-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 20px 0 23px;
}
.distribution-summary > b {
  font-size: 25px;
  color: #7b593c;
}
.distribution-summary small {
  font-size: 9px;
  font-weight: 400;
  color: #b39a81;
  margin-left: 8px;
}
.distribution-summary > span {
  font-size: 9px;
  color: #c1a68c;
}
.distribution-row {
  display: flex;
  align-items: center;
  gap: 13px;
  margin: 16px 0;
  font-size: 10px;
  color: #ad9277;
}
.distribution-row > span {
  width: 63px;
  flex-shrink: 0;
}
.distribution-row > div {
  flex: 1;
  height: 8px;
  background: #f8f1e7;
  border-radius: 20px;
  overflow: hidden;
}
.distribution-row i {
  display: block;
  height: 100%;
  border-radius: 20px;
  background: linear-gradient(90deg, #b6cfbc, #7fa68a);
}
.distribution-row > b {
  width: 40px;
  text-align: right;
  font-size: 11px;
  font-weight: 500;
  color: #8b6c51;
}
.user-center-distribution footer {
  border-top: 1px solid #efe4d7;
  padding-top: 16px;
  margin-top: 22px;
  color: #b59b81;
  font-size: 10px;
  display: flex;
  gap: 5px;
  align-items: center;
}
.user-center-distribution footer > b {
  color: #927254;
  font-weight: 500;
}
.user-center-distribution footer > span {
  margin-left: auto;
  cursor: help;
}
.user-center-daily header {
  padding: 22px 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 15px;
}
.user-center-daily header > div {
  display: flex;
  align-items: center;
  gap: 14px;
}
.user-center-daily header span {
  font-size: 9px;
  color: #bca389;
  cursor: help;
}
.user-center-daily header > button {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: #c18b5d;
  white-space: nowrap;
}
.user-center-table-wrap {
  overflow-x: auto;
}
.user-center-daily table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
  text-align: left;
  white-space: nowrap;
}
.user-center-daily th {
  padding: 12px 24px;
  background: #faf5ed;
  color: #b79d82;
  font-size: 10px;
  font-weight: 500;
}
.user-center-daily td {
  padding: 15px 24px;
  border-top: 1px solid #f2e9de;
  color: #967455;
}
.user-center-daily td:first-child {
  color: #b1977f;
  font-size: 10px;
}
.user-center-footnote {
  font-size: 9px;
  line-height: 1.9;
  color: #bda78f;
  padding: 17px 2px;
  display: flex;
  gap: 15px;
  justify-content: space-between;
}
.user-center-footnote > span {
  white-space: nowrap;
  cursor: help;
}
@media (max-width: 1100px) {
  .user-center-filters form {
    margin-left: 0;
  }
  .user-center-charts {
    grid-template-columns: 1fr;
  }
  .user-center-metrics {
    gap: 12px;
  }
  .user-center-metrics article {
    padding: 18px;
  }
  .user-center-metrics article > b {
    font-size: 28px;
  }
}
@media (max-width: 650px) {
  .user-center-filters {
    padding: 18px 15px;
    gap: 16px;
  }
  .user-center-range {
    min-width: 0;
    width: 100%;
  }
  .user-center-presets {
    width: 100%;
  }
  .user-center-presets button {
    flex: 1;
  }
  .user-center-filters form {
    width: 100%;
    gap: 6px;
  }
  .user-center-filters label {
    flex: 1;
    min-width: 0;
  }
  .user-center-filters input {
    width: 100%;
    padding: 9px 5px;
    font-size: 9px;
  }
  .user-center-filters .outline {
    padding: 9px;
    font-size: 0;
    gap: 0;
    width: 34px;
    flex-shrink: 0;
  }
  .user-center-metrics,
  .user-center-metrics.six {
    grid-template-columns: 1fr 1fr;
    gap: 11px;
  }
  .user-center-metrics article {
    padding: 17px 15px;
  }
  .user-center-metrics article > div {
    font-size: 10px;
  }
  .user-center-metrics article > div > svg {
    width: 16px;
  }
  .user-center-metrics article > b {
    font-size: 28px;
    margin: 12px 0 9px;
  }
  .user-center-charts {
    gap: 18px;
  }
  .user-center-distribution {
    padding: 20px;
  }
  .user-center-daily header {
    padding: 19px 15px;
  }
  .user-center-daily header > div {
    display: block;
  }
  .user-center-daily header span {
    display: block;
    margin-top: 7px;
  }
  .user-center-daily td,
  .user-center-daily th {
    padding: 13px 15px;
  }
  .user-center-footnote {
    flex-direction: column;
    gap: 3px;
  }
}
</style>
