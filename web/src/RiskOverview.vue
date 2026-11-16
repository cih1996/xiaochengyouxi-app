<script setup>
import { label, dateTime } from "./audit-format";
defineProps({ risk: Object });
</script>
<template>
  <section class="risk-overview">
    <header>
      <h3>综合风险</h3>
      <span :title="'综合核查分数，结合设备和行为证据。'" tabindex="0">ⓘ</span>
    </header>
    <div class="risk-overview-score">
      <b
        >{{ risk.score == null ? "待评估" : risk.score
        }}<small v-if="risk.score != null"> / 100</small></b
      ><span>{{ label(risk.level) }}</span>
    </div>
    <div class="risk-overview-parts">
      <div>
        <span>设备核验</span
        ><b>{{ risk.sdk_score == null ? "待评估" : risk.sdk_score + "分" }}</b>
      </div>
      <div>
        <span>行为审计</span
        ><b>{{
          risk.behavior_score == null ? "待评估" : risk.behavior_score + "分"
        }}</b>
      </div>
      <div>
        <span>证据</span><b>{{ risk.evidence_count }} 条</b>
      </div>
    </div>
    <div v-if="risk.factors.length" class="risk-overview-factors">
      <article v-for="(factor, index) in risk.factors" :key="index">
        <b>{{ factor.label }}</b><span
          >{{ label(factor.source)
          }}<small v-if="factor.version != null">
            · V{{ factor.version }}</small
          ></span
        ><strong v-if="factor.score != null">{{ factor.score }} 分</strong>
        <strong v-else-if="factor.points != null">+{{ factor.points }}</strong>
        <div v-if="factor.observed != null" class="risk-factor-observed">
          <span>命中值 {{ factor.observed }}</span
          ><span>阈值 {{ factor.threshold }}</span>
        </div>
        <details v-if="factor.hits?.length" class="risk-factor-hits">
          <summary>命中规则 {{ factor.hits.length }} 条</summary>
          <pre>{{ JSON.stringify(factor.hits, null, 2) }}</pre>
        </details>
      </article>
    </div>
    <footer>
      策略 {{ risk.policy_revision ?? "未记录"
      }}<span>{{ dateTime(risk.last_assessed_at) }}</span>
    </footer>
  </section>
</template>
<style scoped>
.risk-factor-observed {
  grid-column: 1 / -1;
  display: flex;
  gap: 15px;
  font-size: 9px;
  color: #b59c76;
  margin-top: 4px;
}
.risk-factor-hits {
  grid-column: 1 / -1;
  margin-top: 6px;
}
.risk-factor-hits summary {
  cursor: pointer;
  font-size: 9px;
  color: #b79a70;
}
.risk-factor-hits pre {
  max-height: 190px;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  padding: 10px;
  background: #f7edda;
  border-radius: 7px;
  font: 9px/1.8 monospace;
  color: #a98d64;
}
.risk-overview {
  border: 1px solid #ecd7bc;
  background: #fff7e8;
  border-radius: 12px;
  padding: 20px;
}
.risk-overview header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: #b39772;
}
.risk-overview h3 {
  font-size: 13px;
  font-weight: 600;
  color: #9c7649;
}
.risk-overview header > span {
  font-size: 11px;
  cursor: help;
}
.risk-overview-score {
  display: flex;
  align-items: center;
  gap: 16px;
  margin: 16px 0 20px;
}
.risk-overview-score > b {
  font-size: 32px;
  color: #b17e48;
  font-weight: 600;
}
.risk-overview-score small {
  font-size: 10px;
  font-weight: 400;
  color: #bd9b72;
}
.risk-overview-score > span {
  font-size: 10px;
  color: #b29572;
  padding: 5px 8px;
  background: #f7ecd7;
  border-radius: 6px;
}
.risk-overview-parts {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}
.risk-overview-parts span {
  display: block;
  font-size: 9px;
  color: #ba9d77;
  margin-bottom: 7px;
}
.risk-overview-parts b {
  font-size: 11px;
  color: #ac885c;
  font-weight: 500;
}
.risk-overview-factors {
  border-top: 1px solid #efdfc7;
  margin-top: 18px;
  padding-top: 7px;
}
.risk-overview-factors article {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 5px 9px;
  margin: 12px 0;
}
.risk-overview-factors b {
  font-size: 10px;
  font-weight: 500;
  color: #ab8658;
}
.risk-overview-factors span {
  font-size: 8px;
  color: #bca17c;
  grid-column: 1;
}
.risk-overview-factors small {
  font-size: 8px;
}
.risk-overview-factors strong {
  font-size: 10px;
  font-weight: 500;
  color: #bc915c;
  grid-column: 2;
  grid-row: span 2;
}
.risk-overview footer {
  border-top: 1px solid #eedfc8;
  padding-top: 12px;
  margin-top: 17px;
  display: flex;
  justify-content: space-between;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 8px;
  color: #bfa584;
}
</style>
