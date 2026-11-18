import SwiftUI

/// 红包雨（对照 Android HallRainController）。
/// - 状态：/api/rain/status 轮询（进行中 2s、等待中 5s），用 server_now 校准本机时钟。
/// - 开场后自动加入 /api/rain/join，拿到本人本场固定的红包组（id、kind、出现/截止时间）。
/// - 下落位置完全由服务器时间推算：出现窗口内从顶部线性落到底部；点击调 /api/rain/claim，只传红包 id。
/// - 小包金额由服务端决定；大包需要看激励视频（iOS 广告未接入前只提示）。
/// - DEBUG 下 startPreview() 用本地生成的红包演示界面与动画，不请求服务端、不发币。
@MainActor
final class RainController: ObservableObject {
    struct Round: Decodable { let id: String; let startAt: String?; let endAt: String }
    struct Config: Decodable { let maxVisible: Int?; let overlayOpacity: Double?; let fallSpeed: Double? }
    struct Status: Decodable {
        let phase: String
        let serverNow: String?
        let nextStartAt: String?
        let round: Round?
        let joined: Bool?
        let config: Config?
    }
    struct Packet: Decodable {
        let id: String
        let kind: String
        let revealAt: String
        let expiresAt: String
        var status: String
    }
    struct Assignment: Decodable {
        let serverNow: String?
        let round: Round
        let config: Config?
        var packets: [Packet]
    }
    struct Drop: Identifiable {
        let id: String
        let large: Bool
        let reveal: Date
        let expires: Date
        let hash: Int
    }
    struct Burst: Identifiable { let id = UUID(); let point: CGPoint; let at: Date; let large: Bool }
    struct Credit: Identifiable { let id = UUID(); let coins: Int; let point: CGPoint; let at: Date }

    @Published private(set) var status: Status?
    @Published private(set) var assignment: Assignment?
    @Published private(set) var drops: [Drop] = []
    @Published private(set) var hiddenRound = ""
    @Published private(set) var joinInFlight = false
    @Published private(set) var joinFailed = false
    @Published var countdownDismissed = ""
    @Published private(set) var roundCoins = 0
    @Published private(set) var roundPackets = 0
    @Published var summary: (packets: Int, coins: Int)?
    @Published var bursts: [Burst] = []
    @Published var credits: [Credit] = []
    @Published var hudPulse = 0
    @Published private(set) var preview = false

    /// 服务器时间 - 本机时间
    private var offset: TimeInterval = 0
    private var tapped = Set<String>()
    private var claiming = Set<String>()
    private var attemptedRound = ""
    private var summaryRound = ""
    private var poller: Task<Void, Never>?
    weak var session: Session?

    var now: Date { Date().addingTimeInterval(offset) }

    // MARK: 轮询

    func start() {
        guard poller == nil else { return }
        poller = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { return }
                if !self.preview { await self.poll() }
                let active = self.status?.phase == "active"
                try? await Task.sleep(nanoseconds: (active ? 2 : 5) * 1_000_000_000)
            }
        }
    }

    func stop() { poller?.cancel(); poller = nil }

    private func poll() async {
        guard let s: Status = try? await APIClient.shared.get("/rain/status") else { return }
        sync(s.serverNow)
        status = s
        if s.phase == "active", let r = s.round, now < ISO.date(r.endAt) ?? .distantPast,
           assignment?.round.id != r.id, !joinInFlight, attemptedRound != r.id {
            await join(r.id)
        }
    }

    private func sync(_ serverNow: String?) {
        if let d = ISO.date(serverNow) { offset = d.timeIntervalSinceNow }
    }

    func join(_ roundId: String) async {
        joinInFlight = true
        joinFailed = false
        attemptedRound = roundId
        defer { joinInFlight = false }
        do {
            let a: Assignment = try await APIClient.shared.post("/rain/join", ["round_id": roundId])
            sync(a.serverNow)
            if assignment?.round.id != a.round.id { resetRound() }
            assignment = a
        } catch {
            joinFailed = true
            Toast.shared.show(error.localizedDescription)
        }
    }

    private func resetRound() {
        roundCoins = 0; roundPackets = 0
        tapped.removeAll(); claiming.removeAll()
        drops = []; bursts = []; credits = []
    }

    // MARK: 顶部条文案与点击（对照 Android updateScene）

    struct BarState { let text: String; let action: (() -> Void)? }

    func bar(at date: Date) -> BarState {
        let t = date.addingTimeInterval(offset)
        if preview, let a = assignment, let end = ISO.date(a.round.endAt) {
            let left = end.timeIntervalSince(t)
            return BarState(text: left > 0 ? "预览剩余 " + Self.clock(left) : "预览已结束", action: nil)
        }
        guard let s = status else { return BarState(text: "正在迎接小欢喜", action: nil) }
        switch s.phase {
        case "disabled":
            return BarState(text: "敬请期待", action: nil)
        case "waiting":
            guard let start = ISO.date(s.nextStartAt) else { return BarState(text: "敬请期待", action: nil) }
            return BarState(text: "距下场 " + Self.clock(start.timeIntervalSince(t)), action: nil)
        default:
            guard let r = s.round, let end = ISO.date(r.endAt) else { return BarState(text: "红包雨进行中", action: nil) }
            if t >= end { return BarState(text: "本场已结束", action: nil) }
            if assignment?.round.id != r.id {
                if joinInFlight { return BarState(text: "正在迎接红包雨", action: nil) }
                return BarState(text: "点击加入本场红包雨", action: { [weak self] in Task { await self?.join(r.id) } })
            }
            if hiddenRound == r.id {
                return BarState(text: "展开本场红包雨", action: { [weak self] in self?.hiddenRound = "" })
            }
            return BarState(text: "本场剩余 " + Self.clock(end.timeIntervalSince(t)), action: nil)
        }
    }

    /// 开场前最后 10 秒的倒计时（可关闭）
    func countdown(at date: Date) -> Int? {
        guard !preview, let s = status, s.phase == "waiting", let start = ISO.date(s.nextStartAt),
              countdownDismissed != s.nextStartAt else { return nil }
        let left = start.timeIntervalSince(date.addingTimeInterval(offset))
        return left > 0 && left <= 10 ? Int(left.rounded(.up)) : nil
    }

    func dismissCountdown() { countdownDismissed = status?.nextStartAt ?? "" }

    // MARK: 雨层

    /// 雨层是否展开（已加入本场、未收起、未结束）
    func skyVisible(at date: Date) -> Bool {
        guard let a = assignment, hiddenRound != a.round.id, let end = ISO.date(a.round.endAt) else { return false }
        return date.addingTimeInterval(offset) < end
    }

    func timeLeft(at date: Date) -> Int {
        guard let end = ISO.date(assignment?.round.endAt) else { return 0 }
        return max(0, Int(end.timeIntervalSince(date.addingTimeInterval(offset)).rounded(.up)))
    }

    var overlayOpacity: Double { assignment?.config?.overlayOpacity ?? 0.64 }

    /// 当前应显示的红包（出现窗口内、未领、未点过，最多 max_visible 个）
    func visibleDrops(at date: Date) -> [Drop] {
        guard let a = assignment else { return [] }
        let t = date.addingTimeInterval(offset)
        let limit = a.config?.maxVisible ?? 8
        var out: [Drop] = []
        for p in a.packets where p.status != "claimed" && !tapped.contains(p.id) {
            guard let r = ISO.date(p.revealAt), let e = ISO.date(p.expiresAt), t >= r, t < e else { continue }
            out.append(Drop(id: p.id, large: p.kind == "large", reveal: r, expires: e, hash: Self.stableHash(p.id)))
            if out.count >= limit { break }
        }
        return out
    }

    /// 本场结束：收起雨层，接到过红包就弹结算
    func checkEnded(at date: Date) {
        guard let a = assignment, let end = ISO.date(a.round.endAt), date.addingTimeInterval(offset) >= end,
              summaryRound != a.round.id else { return }
        summaryRound = a.round.id
        if roundPackets > 0 { summary = (roundPackets, roundCoins) }
        if preview { preview = false; assignment = nil }
    }

    func collapse() { if let id = assignment?.round.id { hiddenRound = id } }

    // MARK: 领取

    func tap(_ drop: Drop, at point: CGPoint) {
        guard !claiming.contains(drop.id) else { return }
        tapped.insert(drop.id)
        claiming.insert(drop.id)
        bursts.append(Burst(point: point, at: Date(), large: drop.large))
        Sound.packet()
        if preview { previewClaim(drop, point); return }
        Task {
            defer { claiming.remove(drop.id) }
            struct Claim: Decodable { let kind: String; let reward: Int?; let credited: Bool?; let user: User? }
            do {
                let r: Claim = try await APIClient.shared.post("/rain/claim", ["packet_id": drop.id])
                if r.kind == "small" {
                    mark(drop.id, "claimed")
                    if let u = r.user { session?.user = u }
                    if r.credited == true, let c = r.reward { credit(c, at: point) }
                } else {
                    mark(drop.id, "opened")
                    // 大包：服务端只认激励视频回调入账；iOS 广告 SDK 接入后在此打开视频
                    Toast.shared.show("金喜大红包需要看视频领取，iOS 版即将开放")
                }
            } catch {
                tapped.remove(drop.id)   // 失败可再点
                Toast.shared.show(error.localizedDescription)
            }
        }
    }

    private func credit(_ coins: Int, at point: CGPoint) {
        roundCoins += coins
        roundPackets += 1
        hudPulse += 1
        credits.append(Credit(coins: coins, point: point, at: Date()))
        Sound.coins()
    }

    private func mark(_ id: String, _ state: String) {
        guard let i = assignment?.packets.firstIndex(where: { $0.id == id }) else { return }
        assignment?.packets[i].status = state
    }

    func prune(at date: Date) {
        bursts.removeAll { date.timeIntervalSince($0.at) > 0.7 }
        credits.removeAll { date.timeIntervalSince($0.at) > 1.0 }
    }

    // MARK: 预览（仅 DEBUG）

    #if DEBUG
    /// 本地 45 秒演示：24 个红包（含 1 个大包），不请求服务端
    func startPreview(seconds: Double = 45) {
        let f = ISO8601DateFormatter()
        let start = now
        let end = start.addingTimeInterval(seconds)
        let lifetime = 6.5
        var packets: [Packet] = []
        for i in 0..<24 {
            let reveal = start.addingTimeInterval(0.6 + Double(i) * (seconds - lifetime - 1) / 24)
            packets.append(Packet(id: "preview-\(i)-\(UUID().uuidString.prefix(6))", kind: i == 11 ? "large" : "small",
                                  revealAt: f.string(from: reveal), expiresAt: f.string(from: reveal.addingTimeInterval(lifetime)),
                                  status: "ready"))
        }
        resetRound()
        hiddenRound = ""
        preview = true
        assignment = Assignment(serverNow: nil,
                                round: Round(id: "preview-\(UUID().uuidString)", startAt: f.string(from: start), endAt: f.string(from: end)),
                                config: Config(maxVisible: 8, overlayOpacity: 0.64, fallSpeed: 1), packets: packets)
    }
    #endif

    private func previewClaim(_ drop: Drop, _ point: CGPoint) {
        claiming.remove(drop.id)
        if drop.large { Toast.shared.show("预览：大红包需看视频领取"); return }
        credit(Int.random(in: 1...5), at: point)
    }

    // MARK: 工具

    static func clock(_ t: TimeInterval) -> String {
        let s = max(0, Int(t.rounded(.up)))
        return String(format: "%02d:%02d:%02d", s / 3600, s / 60 % 60, s % 60)
    }

    /// 与平台无关的稳定哈希（决定横向位置、摆动幅度）
    static func stableHash(_ s: String) -> Int {
        var h: UInt32 = 2166136261
        for b in s.utf8 { h = (h ^ UInt32(b)) &* 16777619 }
        return Int(h & 0x7fffffff)
    }
}
