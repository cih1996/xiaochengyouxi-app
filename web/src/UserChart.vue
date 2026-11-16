<script setup>
import { computed, ref } from "vue";
const props = defineProps({
  data: Array,
  metric: String,
  title: String,
  type: { default: "line" },
  color: { default: "orange" },
  unit: { default: "人" },
  empty: { default: "所选时段暂无数据" },
});
const hovered = ref(-1);
const width = 760,
  height = 260,
  left = 42,
  right = 18,
  top = 22,
  bottom = 40;
const plotWidth = width - left - right,
  plotHeight = height - top - bottom;
const maxValue = computed(() =>
  Math.max(0, ...props.data.map((day) => day[props.metric])),
);
const ceiling = computed(() => {
  if (!maxValue.value) return 1;
  const interval = maxValue.value / 4;
  const unit = Math.pow(10, Math.floor(Math.log10(interval)));
  return Math.max(1, Math.ceil(interval / unit) * unit) * 4;
});
const x = (index) => left + ((index + 0.5) * plotWidth) / props.data.length;
const y = (value) => top + plotHeight * (1 - value / ceiling.value);
const points = computed(() =>
  props.data
    .map((day, index) => `${x(index)},${y(day[props.metric])}`)
    .join(" "),
);
const area = computed(
  () =>
    `M ${left},${top + plotHeight} L ${props.data.map((day, index) => `${x(index)},${y(day[props.metric])}`).join(" L ")} L ${width - right},${top + plotHeight} Z`,
);
const dateTicks = computed(() =>
  [
    ...new Set([
      0,
      Math.floor((props.data.length - 1) / 4),
      Math.floor((props.data.length - 1) / 2),
      Math.floor(((props.data.length - 1) * 3) / 4),
      props.data.length - 1,
    ]),
  ].filter((index) => index >= 0),
);
const selected = computed(() =>
  hovered.value >= 0 ? props.data[hovered.value] : null,
);
const number = (value) => Number(value).toLocaleString("zh-CN");
</script>
<template>
  <section class="admin-card user-chart" :class="color">
    <header>
      <h2>{{ title }}</h2>
      <span v-if="selected"
        ><b>{{ selected.date }}</b
        >{{ number(selected[metric]) }} {{ unit }}</span
      ><span v-else>按日统计</span>
    </header>
    <div v-if="!maxValue" class="user-chart-empty">
      <span></span>
      <p>{{ empty }}</p>
    </div>
    <svg
      v-else
      :viewBox="`0 0 ${width} ${height}`"
      role="img"
      :aria-label="title"
      @mouseleave="hovered = -1"
    >
      <g v-for="tick in [0, 1, 2, 3, 4]" :key="tick">
        <line
          :x1="left"
          :x2="width - right"
          :y1="y((ceiling * tick) / 4)"
          :y2="y((ceiling * tick) / 4)"
          class="chart-grid"
        />
        <text
          :x="left - 12"
          :y="y((ceiling * tick) / 4) + 4"
          text-anchor="end"
          class="chart-tick"
        >
          {{ number(Math.round((ceiling * tick) / 4)) }}
        </text>
      </g>
      <template v-if="type === 'bar'">
        <rect
          v-for="(day, index) in data"
          :key="day.date"
          :x="x(index) - Math.max(1, (plotWidth / data.length) * 0.58) / 2"
          :y="y(day[metric])"
          :width="Math.max(1, (plotWidth / data.length) * 0.58)"
          :height="top + plotHeight - y(day[metric])"
          rx="2"
          class="chart-bar"
          :class="{ selected: hovered === index }"
        />
      </template>
      <template v-else>
        <path :d="area" class="chart-area" />
        <polyline :points="points" class="chart-line" />
      </template>
      <g v-for="index in dateTicks" :key="index">
        <text
          :x="x(index)"
          :y="height - 14"
          text-anchor="middle"
          class="chart-tick"
        >
          {{ data[index].date.slice(5).replace("-", "/") }}
        </text>
      </g>
      <g v-if="selected">
        <line
          :x1="x(hovered)"
          :x2="x(hovered)"
          :y1="top"
          :y2="top + plotHeight"
          class="chart-guide"
        />
        <circle
          :cx="x(hovered)"
          :cy="y(selected[metric])"
          r="4"
          class="chart-dot"
        />
      </g>
      <rect
        v-for="(day, index) in data"
        :key="'hover-' + day.date"
        :x="left + (index * plotWidth) / data.length"
        :y="top"
        :width="plotWidth / data.length"
        :height="plotHeight"
        fill="transparent"
        tabindex="0"
        :aria-label="`${day.date} ${title} ${number(day[metric])}${unit}`"
        @mouseenter="hovered = index"
        @focus="hovered = index"
      >
        <title>{{ day.date }} · {{ number(day[metric]) }} {{ unit }}</title>
      </rect>
    </svg>
  </section>
</template>
<style scoped>
.user-chart {
  padding: 23px 22px 12px;
  min-width: 0;
  --chart: #ee9b59;
  --area: #fff2e4;
}
.user-chart.green {
  --chart: #79a48c;
  --area: #edf5ee;
}
.user-chart header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 23px;
  margin-bottom: 16px;
}
.user-chart h2 {
  font-size: 14px;
  color: #79583b;
  font-weight: 600;
}
.user-chart header > span {
  font-size: 10px;
  color: #b59a81;
  display: flex;
  gap: 10px;
  align-items: center;
  white-space: nowrap;
}
.user-chart header b {
  font-size: 9px;
  font-weight: 400;
}
.user-chart svg {
  display: block;
  width: 100%;
  overflow: visible;
}
.chart-grid {
  stroke: #f0e6da;
  stroke-width: 1;
  stroke-dasharray: 3 5;
}
.chart-tick {
  fill: #baa28b;
  font-size: 10px;
  font-family: inherit;
}
.chart-bar {
  fill: var(--chart);
  opacity: 0.7;
}
.chart-bar.selected {
  opacity: 1;
}
.chart-area {
  fill: var(--area);
}
.chart-line {
  fill: none;
  stroke: var(--chart);
  stroke-width: 2.6;
  stroke-linejoin: round;
  stroke-linecap: round;
}
.chart-guide {
  stroke: #d8c3aa;
  stroke-dasharray: 4 4;
}
.chart-dot {
  fill: var(--chart);
  stroke: #fff;
  stroke-width: 2;
}
.user-chart-empty {
  height: 240px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: 16px;
  color: #b69c83;
  font-size: 11px;
}
.user-chart-empty > span {
  width: 46px;
  height: 27px;
  border-bottom: 2px solid #ead8c3;
  border-left: 2px solid #ead8c3;
  position: relative;
}
.user-chart-empty > span:after {
  content: "";
  position: absolute;
  inset: 9px 5px;
  border-top: 2px dashed #dbc2a5;
  transform: rotate(-13deg);
}
@media (max-width: 650px) {
  .user-chart {
    padding: 19px 14px 9px;
  }
  .user-chart header {
    flex-wrap: wrap;
    gap: 7px;
    margin-bottom: 12px;
  }
  .user-chart h2 {
    font-size: 13px;
  }
  .user-chart header > span {
    font-size: 9px;
  }
  .user-chart-empty {
    height: 180px;
  }
}
</style>
