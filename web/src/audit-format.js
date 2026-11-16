export const number = (value) => Number(value).toLocaleString("zh-CN");
export function dateTime(value) {
  return value
    ? new Date(value).toLocaleString("zh-CN", {
        hour12: false,
        timeZone: "Asia/Shanghai",
      })
    : "未记录";
}
export function duration(milliseconds) {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  return [
    ...(seconds >= 3600 ? [Math.floor(seconds / 3600)] : []),
    Math.floor(seconds / 60) % 60,
    seconds % 60,
  ]
    .map((value) => String(value).padStart(2, "0"))
    .join(":");
}
export function label(value) {
  const names = {
    unknown: "未采集",
    collecting: "采集中",
    available: "有记录",
    disabled: "已关闭",
    low: "低",
    medium: "中",
    high: "高",
    sdk: "设备核验",
    device_trust: "设备核验",
    behavior: "行为审计",
    screen_enter: "进入页面",
    screen_leave: "离开页面",
    gesture: "手势",
    business: "业务事件",
    heartbeat: "前台停留",
    shop: "捡漏",
    mine: "我的",
    reward_video: "激励视频",
    product_detail: "商品详情",
    withdrawal: "提现",
    invite: "邀请",
    rain: "红包雨",
    settings: "设置",
    reward_result: "奖励结果",
    red_packet: "红包",
    tab_hall: "大厅",
    tab_shop: "捡漏",
    tab_mine: "我的",
    product: "商品",
    buy: "购买",
    withdraw: "提现",
    rain_packet: "雨红包",
    reward_continue: "继续领奖",
    close: "关闭",
    verified: "签名已核验",
    expired: "已结束",
  };
  if (Object.hasOwn(names, value)) return names[value];
  return (
    {
      observe: "观察",
      review: "待核查",
      hold: "暂缓",
      active: "正常",
      restricted: "受限",
      pending: "待评估",
      reported: "已评估",
      assessment_error: "评估失败",
      signature_only: "持钥签名",
      hardware_attested: "硬件证明",
      missing: "未采集",
      unavailable: "暂无记录",
      foreground: "进入前台",
      background: "进入后台",
      page_enter: "进入页面",
      page_leave: "离开页面",
      click: "点击",
      touch: "触点",
      dwell: "停留",
      reward: "红包奖励",
      reward_complete: "奖励到账",
      check_in: "签到",
      withdrawal: "提现",
      hall: "大厅",
      deals: "捡漏",
      me: "我的",
      packet: "红包",
      reward_result: "奖励结果",
      login: "登录",
      register: "注册",
    }[value] || value
  );
}
