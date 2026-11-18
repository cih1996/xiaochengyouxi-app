<script setup>
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import Icon from "./Icon.vue";
import RiskOverview from "./RiskOverview.vue";
import { number, dateTime, label } from "./audit-format";
const props = defineProps({ api: Function, userId: Number });
const emit = defineEmits(["unauthorized", "detail", "behavior"]);
const graph = ref(null),
  selected = ref(null),
  summary = ref(null),
  loading = ref(false),
  summaryLoading = ref(false),
  error = ref("");
const search = ref(""),
  searchResults = ref([]),
  searching = ref(false),
  searched = ref(false);
const upDepth = ref(3),
  downDepth = ref(3),
  history = ref([]),
  showDevices = ref(true);
const positions = ref({}),
  zoom = ref(1),
  pan = ref({ x: 0, y: 0 }),
  svg = ref(null);
let controller,
  summaryController,
  searchController,
  singleClick,
  drag,
  skipClick = false;
const levels = { unknown: "待评估", low: "低", medium: "中", high: "高" };
const nodesById = computed(
  () => new Map((graph.value?.nodes || []).map((node) => [node.id, node])),
);
const devicePairs = computed(() =>
  (graph.value?.device_links || []).flatMap((device) =>
    device.user_ids
      .slice(1)
      .map((id) => ({ source: device.user_ids[0], target: id, device })),
  ),
);
function fail(failure) {
  if (failure.name === "AbortError") return;
  error.value = failure.message;
  if (failure.status === 401) emit("unauthorized", failure);
}
function resetView() {
  if (!graph.value) return;
  const levels = new Map();
  graph.value.nodes.forEach((node) => {
    if (!levels.has(node.depth)) levels.set(node.depth, []);
    levels.get(node.depth).push(node);
  });
  const minDepth = Math.min(...levels.keys()),
    maxDepth = Math.max(...levels.keys());
  const width = Math.max(
    900,
    ...[...levels.values()].map((nodes) => nodes.length * 188),
  );
  const height = (maxDepth - minDepth) * 118 + 170;
  const next = {};
  for (const [depth, nodes] of levels)
    nodes.forEach(
      (node, index) =>
        (next[node.id] = {
          x: ((index + 0.5) * width) / nodes.length,
          y: (depth - minDepth) * 118 + 85,
        }),
    );
  positions.value = next;
  zoom.value = Math.min(1, 1020 / width, 660 / height);
  pan.value = {
    x: (1100 - width * zoom.value) / 2,
    y: (760 - height * zoom.value) / 2,
  };
}
async function centerOn(id, remember = true) {
  clearTimeout(singleClick);
  controller?.abort();
  summaryController?.abort();
  controller = new AbortController();
  loading.value = true;
  error.value = "";
  selected.value = null;
  summary.value = null;
  try {
    const next = await props.api(
      `/admin/users/${id}/graph?` +
        new URLSearchParams({
          up_depth: String(upDepth.value),
          down_depth: String(downDepth.value),
          limit: "20",
        }),
      { signal: controller.signal },
    );
    if (
      remember &&
      graph.value &&
      graph.value.center_user_id !== next.center_user_id
    )
      history.value.push(graph.value.center_user_id);
    graph.value = next;
    searchResults.value = [];
    searched.value = false;
    resetView();
  } catch (failure) {
    fail(failure);
  } finally {
    loading.value = false;
  }
}
async function findUsers() {
  searchController?.abort();
  searchController = new AbortController();
  searching.value = true;
  error.value = "";
  try {
    const result = await props.api(
      "/admin/users/search?" +
        new URLSearchParams({
          q: search.value.trim(),
          page: "1",
          page_size: "10",
        }),
      { signal: searchController.signal },
    );
    searchResults.value = result.items;
    searched.value = true;
  } catch (failure) {
    fail(failure);
  } finally {
    searching.value = false;
  }
}
async function refresh() {
  if (graph.value) return centerOn(graph.value.center_user_id, false);
  if (props.userId) return centerOn(props.userId, false);
  await findUsers();
  if (searchResults.value.length)
    await centerOn(searchResults.value[0].id, false);
}
async function selectNode(node) {
  selected.value = node;
  summary.value = null;
  summaryLoading.value = true;
  error.value = "";
  summaryController?.abort();
  summaryController = new AbortController();
  try {
    summary.value = await props.api(`/admin/users/${node.id}/graph-summary`, {
      signal: summaryController.signal,
    });
  } catch (failure) {
    fail(failure);
  } finally {
    summaryLoading.value = false;
  }
}
function nodeClick(event, node) {
  if (skipClick) {
    skipClick = false;
    return;
  }
  if (event.detail > 1) return;
  clearTimeout(singleClick);
  singleClick = setTimeout(() => selectNode(node), 380);
}
function nodeDoubleClick(node) {
  clearTimeout(singleClick);
  centerOn(node.id);
}
function previous() {
  const id = history.value.pop();
  if (id) centerOn(id, false);
}
function point(event) {
  const value = svg.value.createSVGPoint();
  value.x = event.clientX;
  value.y = event.clientY;
  return value.matrixTransform(svg.value.getScreenCTM().inverse());
}
function pointerDown(event) {
  if (event.button !== 0) return;
  const node = event.target.closest("[data-node-id]");
  const capture = node || svg.value;
  drag = {
    id: node ? Number(node.dataset.nodeId) : null,
    point: point(event),
    moved: false,
    capture,
  };
  capture.setPointerCapture(event.pointerId);
}
function pointerMove(event) {
  if (!drag) return;
  const next = point(event),
    dx = next.x - drag.point.x,
    dy = next.y - drag.point.y;
  if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
  if (drag.id) {
    positions.value[drag.id].x += dx / zoom.value;
    positions.value[drag.id].y += dy / zoom.value;
  } else {
    pan.value = { x: pan.value.x + dx, y: pan.value.y + dy };
  }
  drag.point = next;
}
function pointerUp(event) {
  if (!drag) return;
  skipClick = drag.moved;
  const capture = drag.capture;
  drag = null;
  if (capture.hasPointerCapture(event.pointerId))
    capture.releasePointerCapture(event.pointerId);
}
function changeZoom(factor, at = { x: 550, y: 380 }) {
  const next = Math.min(2.2, Math.max(0.25, zoom.value * factor));
  pan.value = {
    x: at.x - ((at.x - pan.value.x) * next) / zoom.value,
    y: at.y - ((at.y - pan.value.y) * next) / zoom.value,
  };
  zoom.value = next;
}
function edgePath(source, target) {
  const a = positions.value[source],
    b = positions.value[target];
  if (!a || !b) return "";
  const mid = (a.y + b.y) / 2;
  return `M${a.x},${a.y + 37} C${a.x},${mid} ${b.x},${mid} ${b.x},${b.y - 37}`;
}
async function copy(value) {
  try {
    await navigator.clipboard.writeText(value);
  } catch (failure) {
    fail(failure);
  }
}
watch(
  () => props.userId,
  (id) => {
    if (id) centerOn(id, false);
  },
);
onMounted(refresh);
onUnmounted(() => {
  controller?.abort();
  summaryController?.abort();
  searchController?.abort();
  clearTimeout(singleClick);
});
defineExpose({ refresh });
</script>
<template>
  <div class="invitation-graph">
    <div v-if="error" class="form-error" role="alert">{{ error }}</div>
    <section class="admin-card graph-toolbar">
      <form @submit.prevent="findUsers">
        <Icon name="search" :size="16" /><input
          v-model="search"
          aria-label="查找关系用户"
          placeholder="手机号、昵称或用户编号"
        /><button class="outline" :disabled="searching">查找</button>
        <div v-if="searched" class="graph-search-results">
          <button
            v-for="user in searchResults"
            :key="user.id"
            type="button"
            @click="centerOn(user.id)"
          >
            <b>{{ user.nickname }}</b><span>{{ user.phone }}</span
            ><small>#{{ user.id }}</small>
          </button>
          <p v-if="!searchResults.length">没有找到用户</p>
        </div>
      </form>
      <div class="graph-depth">
        <label
          >向上<select
            v-model.number="upDepth"
            aria-label="上级层数"
            @change="refresh"
          >
            <option v-for="value in 5" :value="value" :key="value">
              {{ value }}级
            </option>
          </select></label
        ><label
          >向下<select
            v-model.number="downDepth"
            aria-label="下级层数"
            @change="refresh"
          >
            <option v-for="value in 5" :value="value" :key="value">
              {{ value }}级
            </option>
          </select></label
        >
      </div>
      <button class="graph-back" :disabled="!history.length" @click="previous">
        <Icon name="back" :size="15" />上个中心</button
      ><label class="graph-device-toggle"
        ><input v-model="showDevices" type="checkbox" />设备关联</label
      >
    </section>
    <section class="admin-card graph-canvas">
      <div v-if="loading" class="graph-loading">正在读取邀请关系…</div>
      <div v-if="!graph && !loading" class="graph-empty">
        <Icon name="user" :size="30" /><span>选择一位用户查看关系</span>
      </div>
      <svg
        v-if="graph"
        ref="svg"
        viewBox="0 0 1100 760"
        role="img"
        aria-label="邀请关系图"
        @pointerdown="pointerDown"
        @pointermove="pointerMove"
        @pointerup="pointerUp"
        @pointercancel="pointerUp"
        @wheel.prevent="
          changeZoom(Math.exp(-$event.deltaY * 0.001), point($event))
        "
      >
        <defs>
          <pattern
            id="relation-dots"
            width="22"
            height="22"
            patternUnits="userSpaceOnUse"
          >
            <circle cx="1" cy="1" r=".8" fill="#e7dbc9" />
          </pattern>
        </defs>
        <rect width="1100" height="760" fill="url(#relation-dots)" />
        <g :transform="`translate(${pan.x} ${pan.y}) scale(${zoom})`">
          <g class="graph-edges">
            <path
              v-for="edge in graph.edges"
              :key="`${edge.source}-${edge.target}`"
              :d="edgePath(edge.source, edge.target)"
            >
              <title>邀请关系 · {{ dateTime(edge.bound_at) }}</title>
            </path>
          </g>
          <g v-if="showDevices" class="graph-device-edges">
            <path
              v-for="(link, index) in devicePairs"
              :key="index"
              :d="edgePath(link.source, link.target)"
            >
              <title>
                {{ link.device.model || "设备关联" }} ·
                {{ link.device.account_count }}个账号
              </title>
            </path>
          </g>
          <g
            v-for="node in graph.nodes"
            :key="node.id"
            class="graph-node"
            :class="{
              center: node.is_center,
              selected: selected?.id === node.id,
            }"
            :data-node-id="node.id"
            :transform="`translate(${positions[node.id].x - 79} ${positions[node.id].y - 37})`"
            role="button"
            tabindex="0"
            :aria-label="`用户 ${node.nickname} ${node.phone}`"
            @click.stop="nodeClick($event, node)"
            @dblclick.stop="nodeDoubleClick(node)"
            @keydown.enter.prevent="selectNode(node)"
          >
            <rect width="158" height="74" rx="12" />
            <circle cx="24" cy="26" r="12" class="graph-node-avatar" />
            <text
              x="24"
              y="30"
              text-anchor="middle"
              class="graph-avatar-letter"
            >
              {{ node.nickname.slice(0, 1) }}
            </text>
            <text x="44" y="24" class="graph-node-name">
              {{
                node.nickname.length > 8
                  ? node.nickname.slice(0, 8) + "…"
                  : node.nickname
              }}
            </text>
            <text x="44" y="40" class="graph-node-phone">{{ node.phone }}</text>
            <text x="14" y="61" class="graph-node-coins">
              {{ number(node.coins) }} 金币
            </text>
            <text
              x="145"
              y="61"
              text-anchor="end"
              class="graph-node-risk"
              :class="node.risk.level"
            >
              {{ node.risk.score == null ? "待评估" : node.risk.score + "分" }}
            </text>
            <g v-if="node.hidden_children_count" transform="translate(127 -8)">
              <rect width="36" height="18" rx="9" class="graph-more-badge" />
              <text x="18" y="12" text-anchor="middle" class="graph-more-text">
                +{{ node.hidden_children_count }}
              </text>
            </g>
          </g>
        </g>
      </svg>
      <div class="graph-zoom">
        <button aria-label="放大关系图" @click="changeZoom(1.2)">+</button
        ><span>{{ Math.round(zoom * 100) }}%</span
        ><button aria-label="缩小关系图" @click="changeZoom(1 / 1.2)">−</button
        ><button aria-label="重置关系图" @click="resetView">
          <Icon name="refresh" :size="15" />
        </button>
      </div>
      <div v-if="graph" class="graph-legend">
        <span><i></i>邀请关系</span
        ><span v-if="showDevices"><i class="dashed"></i>设备关联</span
        ><span>{{ graph.nodes.length }} / {{ graph.max_nodes }} 位用户</span
        ><span v-if="graph.truncated"
          >另有 {{ number(graph.hidden_users) }} 位未展开</span
        ><span v-if="graph.cycle_detected" class="graph-warning"
          >存在循环关系</span
        >
      </div>
      <aside v-if="selected" class="graph-profile">
        <header>
          <div>
            <h2>{{ selected.nickname }}</h2>
            <button @click="copy(selected.phone)">
              {{ selected.phone }}<Icon name="list" :size="12" />
            </button>
          </div>
          <button
            aria-label="关闭关系用户"
            @click="
              selected = null;
              summaryController?.abort();
            "
          >
            <Icon name="close" :size="19" />
          </button>
        </header>
        <div v-if="summaryLoading" class="graph-profile-loading">
          正在读取资料…
        </div>
        <template v-if="summary"
          ><div class="graph-profile-metrics">
            <div>
              <span>金币余额</span><b>{{ number(summary.user.coins) }}</b>
            </div>
            <div>
              <span>最近活跃</span
              ><b class="graph-time">{{
                dateTime(summary.activity.last_active_at)
              }}</b>
            </div>
          </div>
          <RiskOverview :risk="summary.user.risk" />
          <section class="graph-profile-section">
            <h3>
              关联设备 <span>{{ summary.devices.length }}</span>
            </h3>
            <article v-for="device in summary.devices" :key="device.device_id">
              <button
                class="graph-device-id"
                :title="device.device_id"
                @click="copy(device.device_id)"
              >
                {{ device.device_id.slice(0, 18) }}…<Icon
                  name="list"
                  :size="12"
                />
              </button>
              <p>
                {{
                  [device.model, device.os_version]
                    .filter(Boolean)
                    .join(" · ") || "型号未上报"
                }}
              </p>
              <div>
                <span>{{ label(device.evidence_status) }}</span><span
                  >{{ device.account_count }}个账号</span
                ><span>{{
                  device.risk_score == null
                    ? "待评估"
                    : device.risk_score + "分"
                }}</span>
              </div>
              <div
                v-if="device.related_user_ids.length"
                class="graph-related-users"
              >
                <button
                  v-for="id in device.related_user_ids"
                  :key="id"
                  @click="centerOn(id)"
                >
                  用户 #{{ id }}<Icon name="arrow" :size="11" />
                </button>
              </div>
            </article>
            <p v-if="!summary.devices.length" class="graph-no-data">
              暂无设备记录
            </p>
          </section>
          <section class="graph-profile-section">
            <h3>关系与活跃</h3>
            <dl>
              <div>
                <dt>上级邀请人</dt>
                <dd>
                  <button
                    v-if="summary.inviter"
                    @click="centerOn(summary.inviter.id)"
                  >
                    {{ summary.inviter.nickname
                    }}<Icon name="arrow" :size="12" /></button
                  ><span v-else>无账号上级</span>
                </dd>
              </div>
              <div>
                <dt>直接邀请 / 全部下级</dt>
                <dd>
                  {{ summary.invitation.children_count }} /
                  {{ summary.invitation.descendants_count }}
                </dd>
              </div>
              <div>
                <dt>近7日 / 30日活跃</dt>
                <dd>
                  {{ summary.activity.active_days_7d }} /
                  {{ summary.activity.active_days_30d }} 天
                </dd>
              </div>
              <div>
                <dt>近7日行为</dt>
                <dd>{{ summary.activity.events_7d }} 次</dd>
              </div>
            </dl>
          </section>
          <div class="graph-profile-actions">
            <button class="outline" @click="emit('behavior', selected.id)">
              行为画像</button
            ><button class="primary" @click="emit('detail', selected.id)">
              完整明细<Icon name="arrow" :size="14" /></button
            ><button class="graph-recenter" @click="centerOn(selected.id)">
              <Icon name="send" :size="13" />以此用户展开
            </button>
          </div></template
        >
      </aside>
    </section>
  </div>
</template>
<style scoped>
.invitation-graph {
  max-width: 1500px;
}
.graph-toolbar {
  display: flex;
  align-items: center;
  gap: 18px;
  flex-wrap: wrap;
  padding: 17px 20px;
  margin-bottom: 18px;
}
.graph-toolbar form {
  display: flex;
  align-items: center;
  gap: 10px;
  position: relative;
  flex: 1;
  min-width: 230px;
}
.graph-toolbar form > svg {
  color: #bd9975;
}
.graph-toolbar input:not([type="checkbox"]) {
  padding: 10px 12px;
  font-size: 13px;
  border-radius: 8px;
  min-width: 0;
  flex: 1;
}
.graph-toolbar .outline {
  padding: 10px 13px;
  font-size: 12.5px;
  border-radius: 8px;
}
.graph-depth {
  display: flex;
  gap: 10px;
}
.graph-depth label {
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 0;
  font-size: 12.5px;
  font-weight: 400;
  color: #b09377;
}
.graph-depth select {
  width: 61px;
  padding: 9px 5px;
  font-size: 12.5px;
  border-radius: 8px;
  margin: 0;
}
.graph-back {
  display: flex;
  align-items: center;
  gap: 5px;
  color: #b38c66;
  font-size: 12.5px;
}
.graph-device-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  font-weight: 400;
  color: #aa8a6b;
  margin: 0;
  white-space: nowrap;
}
.graph-device-toggle input {
  width: 14px;
  height: 14px;
  margin: 0;
  accent-color: #da9e67;
}
.graph-search-results {
  position: absolute;
  top: 45px;
  left: 0;
  right: 0;
  background: #fffdfa;
  border: 1px solid #e9d7c3;
  border-radius: 11px;
  box-shadow: 0 12px 35px #6e492722;
  z-index: 20;
  max-height: 330px;
  overflow: auto;
}
.graph-search-results > button {
  display: flex;
  width: 100%;
  gap: 10px;
  align-items: center;
  padding: 13px 15px;
  border-bottom: 1px solid #f3e9dd;
  font-size: 13px;
  text-align: left;
}
.graph-search-results b {
  color: #855b3a;
  font-weight: 500;
}
.graph-search-results span {
  color: #b1967b;
}
.graph-search-results small {
  margin-left: auto;
  color: #c1aa90;
  font-size: 12px;
}
.graph-search-results > p {
  padding: 19px;
  text-align: center;
  font-size: 13px;
  color: #b69b7f;
}
.graph-canvas {
  position: relative;
  overflow: hidden;
  min-height: 620px;
  background: #fcfaf6;
}
.graph-canvas > svg {
  display: block;
  width: 100%;
  height: calc(100vh - 260px);
  min-height: 620px;
  max-height: 840px;
  touch-action: none;
  user-select: none;
  cursor: grab;
}
.graph-canvas > svg:active {
  cursor: grabbing;
}
.graph-edges path {
  stroke: #caa780;
  stroke-width: 1.7;
  fill: none;
}
.graph-device-edges path {
  stroke: #91ac9f;
  stroke-width: 1.5;
  stroke-dasharray: 6 6;
  fill: none;
  opacity: 0.8;
}
.graph-node {
  cursor: grab;
}
.graph-node > rect {
  fill: #fffdf9;
  stroke: #e6d7c4;
  stroke-width: 1.2;
  filter: drop-shadow(0 3px 4px #6d43170a);
}
.graph-node.center > rect {
  fill: #fff3e0;
  stroke: #dfa36b;
  stroke-width: 2;
}
.graph-node.selected > rect {
  stroke: #d58542;
  stroke-width: 2.5;
}
.graph-node-avatar {
  fill: #f8e7ce;
}
.graph-avatar-letter {
  fill: #bd8b5c;
  font-size: 12.5px;
}
.graph-node-name {
  fill: #846044;
  font-size: 13px;
  font-weight: 600;
}
.graph-node-phone {
  fill: #b69a7e;
  font-size: 11px;
}
.graph-node-coins {
  fill: #d39b60;
  font-size: 12px;
}
.graph-node-risk {
  fill: #a7957e;
  font-size: 11px;
}
.graph-node-risk.high {
  fill: #cd7860;
}
.graph-node-risk.medium {
  fill: #c69b50;
}
.graph-node-risk.low {
  fill: #83a083;
}
.graph-more-badge {
  fill: #e9ab72;
}
.graph-more-text {
  fill: #fffaf1;
  font-size: 11px;
}
.graph-loading {
  position: absolute;
  inset: 0;
  z-index: 12;
  background: #fffaf2cc;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13.5px;
  color: #b69a7a;
}
.graph-empty {
  min-height: 620px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  align-items: center;
  justify-content: center;
  color: #c0a386;
  font-size: 13.5px;
}
.graph-zoom {
  position: absolute;
  bottom: 20px;
  right: 20px;
  display: flex;
  align-items: center;
  gap: 8px;
  border: 0;
  border-radius: 9px;
  background: #fffdf6f5;
  padding: 5px;
  z-index: 13;
  box-shadow: 0 3px 12px #976f3b0b;
}
.graph-zoom button {
  width: 26px;
  height: 26px;
  font-size: 20px;
  display: grid;
  place-items: center;
  color: #ad875e;
}
.graph-zoom span {
  font-size: 12.5px;
  color: #b79b7b;
  min-width: 35px;
  text-align: center;
}
.graph-legend {
  position: absolute;
  bottom: 22px;
  left: 20px;
  display: flex;
  gap: 13px;
  flex-wrap: wrap;
  max-width: 65%;
  color: #b59b7e;
  font-size: 12px;
  pointer-events: none;
}
.graph-legend > span {
  display: flex;
  align-items: center;
  gap: 6px;
}
.graph-legend i {
  width: 19px;
  height: 0;
  border-top: 2px solid #caa780;
}
.graph-legend i.dashed {
  border-top: 2px dashed #91ac9f;
}
.graph-warning {
  color: #ca8561;
}
.graph-profile {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 320px;
  background: #fffdfa;
  z-index: 14;
  border-left: 1px solid #e8dbc9;
  box-shadow: -8px 0 24px #7250220c;
  overflow-y: auto;
  padding: 23px;
}
.graph-profile > header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 22px;
}
.graph-profile h2 {
  font-size: 20px;
  color: #775035;
}
.graph-profile > header > div > button {
  display: flex;
  gap: 7px;
  align-items: center;
  font-size: 13px;
  color: #b29475;
  margin-top: 9px;
}
.graph-profile > header > button {
  color: #b69b7d;
}
.graph-profile-metrics {
  display: grid;
  grid-template-columns: 1fr 1.2fr;
  gap: 12px;
  margin-bottom: 20px;
}
.graph-profile-metrics span {
  display: block;
  font-size: 12px;
  color: #b59b7f;
  margin-bottom: 8px;
}
.graph-profile-metrics b {
  font-size: 22px;
  color: #b08454;
  font-weight: 600;
}
.graph-profile-metrics b.graph-time {
  font-size: 12.5px;
  line-height: 1.8;
  font-weight: 400;
  color: #9a7857;
}
.graph-profile-section {
  margin-top: 23px;
}
.graph-profile-section h3 {
  font-size: 13.5px;
  color: #90704f;
  font-weight: 600;
  margin-bottom: 13px;
}
.graph-profile-section h3 > span {
  color: #bc9e7c;
  font-weight: 400;
  margin-left: 5px;
}
.graph-profile-section article {
  padding: 13px 0;
  border-bottom: 1px solid #f0e7db;
}
.graph-device-id {
  display: flex;
  gap: 7px;
  align-items: center;
  font: 9px monospace;
  color: #a9855e;
  max-width: 100%;
}
.graph-profile-section article > p {
  font-size: 12.5px;
  color: #b49a7b;
  margin: 8px 0;
}
.graph-profile-section article > div {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  font-size: 11px;
  color: #bea78b;
}
.graph-profile-section .graph-related-users {
  gap: 8px;
  margin-top: 10px;
}
.graph-related-users button {
  display: flex;
  gap: 3px;
  align-items: center;
  font-size: 12px;
  color: #b79067;
}
.graph-no-data {
  font-size: 12.5px;
  color: #bea88d;
}
.graph-profile dl {
  margin: 0;
}
.graph-profile dl > div {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  margin: 12px 0;
  font-size: 12px;
}
.graph-profile dt {
  color: #b79b7c;
}
.graph-profile dd {
  color: #9e7c54;
  margin: 0;
  text-align: right;
}
.graph-profile dd button {
  display: flex;
  gap: 4px;
  align-items: center;
  font-size: 12px;
}
.graph-profile-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 25px;
}
.graph-profile-actions > .primary,
.graph-profile-actions > .outline {
  flex: 1;
  font-size: 12.5px;
  padding: 10px 11px;
  border-radius: 8px;
}
.graph-recenter {
  display: flex;
  gap: 6px;
  align-items: center;
  justify-content: center;
  width: 100%;
  margin-top: 7px;
  font-size: 12.5px;
  color: #bb9166;
  padding: 7px;
}
.graph-profile-loading {
  font-size: 13px;
  color: #bba386;
  padding: 25px 0;
}
@media (max-width: 650px) {
  .graph-toolbar {
    padding: 15px;
    gap: 14px;
  }
  .graph-toolbar form {
    flex-basis: 100%;
  }
  .graph-depth {
    gap: 13px;
  }
  .graph-depth select {
    width: 55px;
  }
  .graph-back {
    margin-left: auto;
  }
  .graph-canvas,
  .graph-canvas > svg,
  .graph-empty {
    min-height: 560px;
  }
  .graph-canvas > svg {
    height: 600px;
  }
  .graph-legend {
    font-size: 11px;
    left: 14px;
    bottom: 66px;
    max-width: 88%;
    gap: 9px;
  }
  .graph-zoom {
    right: 14px;
    bottom: 15px;
  }
  .graph-profile {
    top: auto;
    left: 0;
    width: 100%;
    height: 72%;
    border-left: 0;
    border-top: 1px solid #e8dbc9;
    border-radius: 17px 17px 0 0;
    padding: 20px;
  }
  .graph-profile-metrics {
    grid-template-columns: 1fr 1fr;
  }
  .graph-profile-section article > div {
    font-size: 12px;
  }
  .graph-profile dl > div {
    font-size: 12.5px;
  }
}
</style>
