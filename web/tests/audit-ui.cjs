const { chromium } = require("../../tools/node_modules/playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs"),
  path = require("node:path");
const dist = path.join(__dirname, "../dist"),
  origin = "https://audit-preview.test";
const state = {
  graph: [],
  summaries: 0,
  profiles: 0,
  sessionReads: 0,
  policyWrites: 0,
  unexpected: [],
};
const node = (id, depth = 0, center = false) => ({
  id,
  nickname: id === 1 ? "中心用户" : "关联用户" + id,
  phone: "1380013" + String(id).padStart(4, "0"),
  invite_code: "JZ8F6A",
  coins: id * 100,
  created_at: "2026-11-01T00:00:00Z",
  last_active_at: "2026-11-11T00:00:00Z",
  account_status: "active",
  depth,
  is_center: center,
  children_count: 2,
  hidden_children_count: 0,
  device_count: 1,
  risk: {
    score: id === 1 ? 28 : null,
    level: id === 1 ? "low" : "unknown",
    sdk_score: null,
    behavior_score: id === 1 ? 28 : null,
    evidence_count: 0,
    last_assessed_at: null,
    policy_revision: 1,
    factors: [],
  },
});
function graph(id) {
  state.graph.push(id);
  return {
    center_user_id: id,
    up_depth: 3,
    down_depth: 3,
    max_nodes: 20,
    total_users_in_scope: 3,
    hidden_users: 0,
    truncated: false,
    cycle_detected: false,
    server_now: new Date().toISOString(),
    nodes: [node(id, 0, true), node(id + 1, 1), node(id + 2, 1)],
    edges: [
      {
        source: id,
        target: id + 1,
        kind: "invitation",
        bound_at: "2026-11-02T00:00:00Z",
      },
      {
        source: id,
        target: id + 2,
        kind: "invitation",
        bound_at: "2026-11-03T00:00:00Z",
      },
    ],
    device_links: [
      {
        device_id: "key-1",
        user_ids: [id, id + 1],
        model: "机型",
        account_count: 2,
        risk_score: null,
      },
    ],
  };
}
function profile() {
  return {
    collection: {
      status: "available",
      since: "2026-11-01T00:00:00Z",
      last_seen_at: "2026-11-11T00:00:00Z",
      session_count: 1,
      missing: [],
      notes: [],
    },
    summary: {
      foreground_ms: 12000,
      click_count: 2,
      gesture_count: 1,
      red_packet_opens: 1,
      ad_starts: 1,
      ad_completions: 1,
      claimed_coins: 40,
      claimed_count: 1,
      unmatched_business_count: 0,
      dropped_events: 0,
    },
    pages: [
      { screen: "hall", screen_code: 1, dwell_ms: 9000, entries: 1, clicks: 2 },
    ],
    daily: [
      {
        date: "2026-11-11",
        foreground_ms: 12000,
        click_count: 2,
        reward_coins: 40,
        session_count: 1,
      },
    ],
    click_intervals: {
      sample_count: 1,
      median_ms: 1200,
      p95_ms: 1200,
      buckets: [{ label: "1–2秒", count: 1 }],
    },
    risk: {
      score: 28,
      level: "low",
      sdk_score: null,
      behavior_score: 28,
      evidence_count: 0,
      last_assessed_at: null,
      policy_revision: 1,
      factors: [],
    },
  };
}
async function fixture(context) {
  await context.route("**/*", async (route) => {
    const request = route.request(),
      url = new URL(request.url()),
      endpoint = url.pathname.slice(4),
      json = (data) => route.fulfill({ json: data });
    if (url.pathname.startsWith("/api/")) {
      if (endpoint === "/config")
        return json({
          admin_auth_required: true,
          environment: "production",
          ad_mode: "csj",
          sms_mode: "mock",
          payment_mode: "mock",
        });
      if (endpoint === "/tasks" || endpoint === "/products") return json([]);
      if (endpoint.startsWith("/admin/")) {
        assert.equal(request.headers()["x-admin-token"], "mock-audit-admin");
      }
      const defaults = {
        "/admin/account": { username: "审计管理员", auth_required: true },
        "/admin/overview": { total_users: 3, ad_platforms: [] },
        "/admin/rewards": [],
        "/admin/withdrawals": [],
        "/admin/settings": {},
        "/admin/hall-strategy": {},
        "/admin/invitations": { records: [], platform_code: "TEST26" },
        "/admin/users/search": {
          items: [
            {
              id: 1,
              nickname: "中心用户",
              phone: "13800130001",
              coins: 100,
              created_at: "2026-11-01T00:00:00Z",
              account_status: "active",
              risk_count: 0,
            },
          ],
          total: 1,
          page: 1,
          page_size: 20,
        },
      };
      if (Object.hasOwn(defaults, endpoint)) return json(defaults[endpoint]);
      if (endpoint.startsWith("/admin/users/") && endpoint.endsWith("/graph"))
        return json(graph(Number(endpoint.split("/")[3])));
      if (
        endpoint.startsWith("/admin/users/") &&
        endpoint.endsWith("/graph-summary")
      ) {
        state.summaries++;
        return json({
          user: node(1, 0, true),
          devices: [],
          activity: {
            last_active_at: "2026-11-11T00:00:00Z",
            active_days_7d: 1,
            active_days_30d: 1,
            events_7d: 2,
          },
          recent_events: [],
          inviter: null,
          invitation: { children_count: 2, descendants_count: 2 },
          summary: {
            ledger_count: 1,
            total_earned_coins: 40,
            withdrawal_count: 0,
          },
        });
      }
      if (endpoint === "/admin/behavior/policy") {
        if (request.method() === "PUT") state.policyWrites++;
        return json({
          enabled: true,
          mode: "observe",
          revision: 2,
          session_ttl_seconds: 1800,
          batch_interval_seconds: 15,
          touch_sample_percent: 100,
          click_window_seconds: 10,
          click_count_threshold: 30,
          min_interval_samples: 12,
          interval_tolerance_ms: 40,
          repeated_interval_ratio: 0.9,
          short_dwell_ms: 800,
          short_dwell_samples: 12,
          business_mismatch_threshold: 5,
          session_anomaly_threshold: 3,
          review_score: 40,
          hold_score: 75,
          weights: {
            click_frequency: 25,
            regular_intervals: 30,
            short_dwell: 20,
            business_mismatch: 25,
            session_anomaly: 20,
          },
        });
      }
      if (endpoint.endsWith("/behavior/profile")) {
        state.profiles++;
        return json(profile());
      }
      if (
        endpoint.endsWith("/behavior/sessions") ||
        endpoint.includes("/behavior/sessions/s-1")
      ) {
        state.sessionReads++;
        if (endpoint.includes("/s-1"))
          return json({
            session: {
              id: "s-1",
              started_at: "2026-11-11T00:00:00Z",
              last_received_at: "2026-11-11T00:00:12Z",
              expires_at: "2026-11-11T00:30:00Z",
              status: "expired",
              protocol: "OBA1",
              device_id: "key-1",
              key_id: "key-1",
              proof_level: "signature_only",
              event_count: 3,
              batch_count: 1,
              foreground_ms: 12000,
              click_count: 2,
              dropped_events: 0,
              behavior_score: 28,
              last_assessed_at: null,
              policy_revision: 1,
              viewport_width: 390,
              viewport_height: 844,
            },
            events: [
              {
                id: "e1",
                time_ms: 0,
                type: "screen_enter",
                screen: "hall",
                control_id: "unknown",
                server_confirmed: false,
                credited_coins: null,
              },
              {
                id: "e2",
                time_ms: 1200,
                type: "click",
                screen: "hall",
                control_id: "red_packet",
                x: 5000,
                y: 4000,
                server_confirmed: false,
                credited_coins: null,
              },
              {
                id: "e3",
                time_ms: 4000,
                type: "business",
                screen: "reward_result",
                control_id: "unknown",
                operation_ref: "op-1",
                server_confirmed: true,
                server_state: "credited",
                credited_coins: 40,
              },
            ],
            events_total: 3,
            next_offset: null,
            batches: [],
            alerts: [],
            failures: [],
            availability: { status: "available", notes: [] },
          });
        return json({
          items: [
            {
              id: "s-1",
              started_at: "2026-11-11T00:00:00Z",
              last_received_at: "2026-11-11T00:00:12Z",
              status: "expired",
              protocol: "OBA1",
              device_id: "key-1",
              proof_level: "signature_only",
              event_count: 3,
              batch_count: 1,
              foreground_ms: 12000,
              click_count: 2,
              dropped_events: 0,
              behavior_score: 28,
            },
          ],
          total: 1,
          page: 1,
          page_size: 20,
        });
      }
      if (endpoint === "/admin/behavior/alerts")
        return json({ items: [], total: 0, page: 1, page_size: 20 });
      state.unexpected.push(endpoint);
      return route.fulfill({ status: 404, json: { error: "未配置测试接口" } });
    }
    const file = url.pathname.startsWith("/assets/")
      ? path.join(dist, decodeURIComponent(url.pathname.slice(1)))
      : path.join(dist, "index.html");
    return route.fulfill({
      contentType: {
        ".html": "text/html",
        ".js": "text/javascript",
        ".css": "text/css",
        ".png": "image/png",
      }[path.extname(file)],
      body: fs.readFileSync(file),
    });
  });
}
(async () => {
  const browser = await chromium.launch({
    executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe",
    headless: true,
  });
  const errors = [];
  try {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 1050 },
    });
    await context.addInitScript(() =>
      sessionStorage.setItem("orange-admin-token", "mock-audit-admin"),
    );
    await fixture(context);
    const page = await context.newPage();
    page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(origin + "/admin");
    const nav = page.getByRole("navigation", { name: "后台导航" });
    await nav.getByRole("button", { name: "邀请关系", exact: true }).click();
    await page.locator(".graph-node").first().waitFor();
    assert.equal(state.graph[0], 1);
    await page.locator(".graph-node").first().click();
    await page.getByText("综合风险", { exact: true }).waitFor();
    assert.equal(state.summaries, 1);
    await page.locator(".graph-node").nth(1).dblclick();
    await page.waitForFunction(
      () => document.querySelectorAll(".graph-node").length === 3,
    );
    assert.equal(state.graph.at(-1), 2);
    await nav.getByRole("button", { name: "行为审计", exact: true }).click();
    await page.getByRole("button", { name: "用户画像", exact: true }).click();
    await page.getByRole("button", { name: "查找用户", exact: true }).click();
    await page.getByText("中心用户", { exact: true }).last().click();
    await page.getByText("已核对到账金币", { exact: true }).waitFor();
    assert.equal(state.profiles, 1);
    await page.getByRole("button", { name: "回放", exact: true }).click();
    await page
      .getByRole("heading", { name: "行为回放", exact: true })
      .waitFor();
    await page.getByRole("button", { name: "播放回放", exact: true }).click();
    await page.getByText("+40", { exact: true }).waitFor({ timeout: 2000 });
    await page
      .getByRole("button", { name: "关闭行为回放", exact: true })
      .click();
    await page.getByRole("button", { name: "审核策略", exact: true }).click();
    await page.getByLabel("开启行为审计", { exact: true }).uncheck();
    await page.getByRole("button", { name: "保存策略", exact: true }).click();
    assert.equal(state.policyWrites, 1);
    assert.deepEqual(state.unexpected, []);
    assert.deepEqual(errors, []);
    console.log(
      JSON.stringify({
        passed: true,
        graph: true,
        profile: true,
        replay: true,
        policy: true,
        noFabricatedCoins: true,
        runtimeErrors: errors,
      }),
    );
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
