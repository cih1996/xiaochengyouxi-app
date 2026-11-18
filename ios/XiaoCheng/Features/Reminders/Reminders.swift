import Foundation
import UserNotifications
import UIKit

/// 本地通知提醒（对应 Android ReminderReceiver，见 docs/REMINDERS.md）。
/// - 回到前台：刷新配置与状态并取消已安排的提醒；离开前台：按缓存的数据重新安排。
/// - 红包雨只提醒最近一场（开场前 rain_lead_minutes 分钟，限 1–10）；签到 / 转盘在当天设定时间提醒。
/// - 免打扰时段（北京时间）内不安排。
/// - 授权：默认用 iOS 临时授权（.provisional）——不弹框，通知静默进入通知中心，用户可一键"保留"；
///   设置页的"通知提醒"开关会请求正式授权。
@MainActor
final class Reminders: NSObject, ObservableObject, UNUserNotificationCenterDelegate {
    static let shared = Reminders()

    struct Config: Decodable {
        let enabled: Bool
        let rainEnabled: Bool?
        let rainLeadMinutes: Int?
        let rainTitle: String?
        let rainBody: String?
        let checkinEnabled: Bool?
        let checkinTime: String?
        let checkinTitle: String?
        let checkinBody: String?
        let wheelEnabled: Bool?
        let wheelTime: String?
        let wheelTitle: String?
        let wheelBody: String?
        let quietStart: String?
        let quietEnd: String?
    }

    enum Target: String { case hall, checkin, wheel }

    /// 点击通知后要去的页面，MainTabView / HallView 观察它
    @Published var pendingTarget: Target?
    @Published private(set) var authorization: UNAuthorizationStatus = .notDetermined

    var userEnabled: Bool {
        get { UserDefaults.standard.object(forKey: "reminders.enabled") as? Bool ?? true }
        set {
            UserDefaults.standard.set(newValue, forKey: "reminders.enabled")
            objectWillChange.send()
            if !newValue { cancelAll() }
        }
    }

    private var config: Config?
    private var nextRainStart: Date?
    private var checkedInToday = true
    private var wheelAvailable = 0
    private let center = UNUserNotificationCenter.current()
    private let beijing = TimeZone(identifier: "Asia/Shanghai")!

    func activate() {
        center.delegate = self
        Task { await refreshAuthorization() }
    }

    // MARK: - 授权

    func refreshAuthorization() async {
        authorization = await center.notificationSettings().authorizationStatus
    }

    /// 首次登录后静默申请临时授权（不弹框）
    func requestProvisionalIfNeeded() async {
        await refreshAuthorization()
        guard authorization == .notDetermined else { return }
        _ = try? await center.requestAuthorization(options: [.alert, .sound, .badge, .provisional])
        await refreshAuthorization()
    }

    /// 用户在设置里主动打开：请求正式授权；已被拒绝时返回 false，由界面引导去系统设置
    func requestFull() async -> Bool {
        await refreshAuthorization()
        if authorization == .denied { return false }
        let ok = (try? await center.requestAuthorization(options: [.alert, .sound, .badge])) ?? false
        await refreshAuthorization()
        return ok || authorization == .provisional
    }

    // MARK: - 前后台

    /// 回到前台：取消已安排的提醒，刷新数据
    func enterForeground() {
        cancelAll()
        Task { await refresh() }
    }

    /// 离开前台：按最新缓存安排
    func enterBackground() {
        schedule()
    }

    func refresh() async {
        if let c: Config = try? await APIClient.shared.get("/app/reminders") { config = c }
        guard APIClient.shared.token != nil else { return }
        if let r: RainStatus = try? await APIClient.shared.get("/rain/status") {
            nextRainStart = r.phase == "waiting" ? ISO.date(r.nextStartAt) : nil
        }
        if let c: CheckinStatus = try? await APIClient.shared.get("/check-in/status") { checkedInToday = c.checkedToday }
        struct Badge: Decodable { let available: Int?; let enabled: Bool? }
        if let w: Badge = try? await APIClient.shared.get("/wheel/badge") {
            wheelAvailable = (w.enabled ?? true) ? (w.available ?? 0) : 0
        }
    }

    /// 签到成功后调用，避免当天再提醒
    func markCheckedIn() { checkedInToday = true }

    private func cancelAll() {
        center.removePendingNotificationRequests(withIdentifiers: Target.allIds)
    }

    private func schedule() {
        cancelAll()
        #if DEBUG
        NSLog("[Reminders] schedule auth=%d user=%d config=%d rain=%@ checked=%d wheel=%d",
              authorization.rawValue, userEnabled ? 1 : 0, config == nil ? 0 : 1,
              nextRainStart?.description ?? "nil", checkedInToday ? 1 : 0, wheelAvailable)
        #endif
        guard userEnabled, APIClient.shared.token != nil, let c = config, c.enabled,
              authorization == .authorized || authorization == .provisional || authorization == .ephemeral
        else { return }
        let now = Date()

        if c.rainEnabled == true, let start = nextRainStart {
            let lead = min(max(c.rainLeadMinutes ?? 3, 1), 10)
            let fire = start.addingTimeInterval(TimeInterval(-lead * 60))
            if fire > now.addingTimeInterval(5) {
                let body = (c.rainBody ?? "").replacingOccurrences(of: "{minutes}", with: "\(lead)")
                add(.hall, id: "rain", title: c.rainTitle ?? "红包雨马上开始", body: body, at: fire, quiet: c)
            }
        }
        if c.checkinEnabled == true, !checkedInToday, let t = todayAt(c.checkinTime), t > now {
            add(.checkin, id: "checkin", title: c.checkinTitle ?? "今天还没签到哦", body: c.checkinBody ?? "", at: t, quiet: c)
        }
        if c.wheelEnabled == true, wheelAvailable > 0, let t = todayAt(c.wheelTime), t > now {
            add(.wheel, id: "wheel", title: c.wheelTitle ?? "幸运转盘", body: c.wheelBody ?? "", at: t, quiet: c)
        }
    }

    private func add(_ target: Target, id: String, title: String, body: String, at date: Date, quiet c: Config) {
        guard !inQuietHours(date, c) else { return }
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = body
        content.sound = .default
        content.userInfo = ["target": target.rawValue]
        content.threadIdentifier = "orange.reminders"
        content.interruptionLevel = .active
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: max(1, date.timeIntervalSinceNow), repeats: false)
        center.add(UNNotificationRequest(identifier: "orange.reminder.\(id)", content: content, trigger: trigger))
        #if DEBUG
        NSLog("[Reminders] scheduled %@ at %@ (%@)", id, date.description, body)
        #endif
    }

    // MARK: - 北京时间

    private func minutes(_ hhmm: String?) -> Int? {
        guard let p = hhmm?.split(separator: ":"), p.count == 2, let h = Int(p[0]), let m = Int(p[1]) else { return nil }
        return h * 60 + m
    }

    private func todayAt(_ hhmm: String?) -> Date? {
        guard let m = minutes(hhmm) else { return nil }
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = beijing
        return cal.date(bySettingHour: m / 60, minute: m % 60, second: 0, of: Date())
    }

    private func inQuietHours(_ date: Date, _ c: Config) -> Bool {
        guard let s = minutes(c.quietStart), let e = minutes(c.quietEnd), s != e else { return false }
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = beijing
        let x = cal.component(.hour, from: date) * 60 + cal.component(.minute, from: date)
        return s < e ? (x >= s && x < e) : (x >= s || x < e)
    }

    // MARK: - UNUserNotificationCenterDelegate

    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            didReceive response: UNNotificationResponse) async {
        let raw = response.notification.request.content.userInfo["target"] as? String
        await MainActor.run { self.pendingTarget = raw.flatMap(Target.init(rawValue:)) ?? .hall }
    }

    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            willPresent notification: UNNotification) async -> UNNotificationPresentationOptions {
        [] // 前台已取消全部提醒；万一到达也不打扰
    }
}

private extension Reminders.Target {
    static var allIds: [String] { ["rain", "checkin", "wheel"].map { "orange.reminder.\($0)" } }
}
