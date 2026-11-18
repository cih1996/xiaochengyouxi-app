import Foundation
import SwiftUI

/// 大厅数据：/api/hall/stream（SSE，每条 data 是一个 HallItem）+ 签到 + 红包雨状态。
/// 与 Android 一致，消息只保存在内存里。
@MainActor
final class HallModel: ObservableObject {
    @Published var items: [HallItem] = []
    @Published var checkin: CheckinStatus?
    @Published var rain: RainStatus?
    @Published var rainOffset: TimeInterval = 0   // 服务器时间 - 本机时间
    private var streamTask: Task<Void, Never>?

    func start() {
        guard streamTask == nil else { return }
        streamTask = Task { [weak self] in await self?.runStream() }
        Task { await refreshCheckin() }
        Task { await pollRain() }
    }

    func stop() {
        streamTask?.cancel()
        streamTask = nil
    }

    func refreshCheckin() async {
        checkin = try? await APIClient.shared.get("/check-in/status")
    }

    private func pollRain() async {
        while !Task.isCancelled {
            if let r: RainStatus = try? await APIClient.shared.get("/rain/status") { rain = r }
            try? await Task.sleep(nanoseconds: (rain?.phase == "active" ? 2 : 15) * 1_000_000_000)
        }
    }

    private func runStream() async {
        var backoff: UInt64 = 1
        while !Task.isCancelled {
            do {
                var req = APIClient.shared.request("/hall/stream")
                req.setValue("text/event-stream", forHTTPHeaderField: "Accept")
                req.timeoutInterval = 3600
                let (bytes, resp) = try await URLSession.shared.bytes(for: req)
                if (resp as? HTTPURLResponse)?.statusCode == 401 { return }
                backoff = 1
                var event = ""
                for try await line in bytes.lines {
                    if line.hasPrefix("event:") {
                        event = line.dropFirst(6).trimmingCharacters(in: .whitespaces)
                    } else if line.hasPrefix("data:") {
                        let data = Data(line.dropFirst(5).trimmingCharacters(in: .whitespaces).utf8)
                        if event == "error" { break }
                        if let item = try? Self.decoder.decode(HallItem.self, from: data) { append(item) }
                        event = ""
                    }
                }
            } catch {
                if Task.isCancelled { return }
            }
            try? await Task.sleep(nanoseconds: backoff * 1_000_000_000)
            backoff = min(backoff * 2, 30)
        }
    }

    private func append(_ item: HallItem) {
        // 与 Android 相同：连续重复的文字消息不重复显示
        if item.kind == "text", let last = items.last, last.kind == "text", last.text == item.text { return }
        guard !items.contains(where: { $0.id == item.id }) else { return }
        withAnimation(.spring(response: 0.4, dampingFraction: 0.85)) { items.append(item) }
    }

    private static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }()

    func rainText(now: Date) -> String {
        guard let rain else { return "正在迎接小欢喜" }
        let serverNow = now.addingTimeInterval(rainOffset)
        switch rain.phase {
        case "disabled": return "敬请期待"
        case "waiting":
            guard let start = ISO.date(rain.nextStartAt) else { return "敬请期待" }
            return "距下场 " + Self.clock(start.timeIntervalSince(serverNow))
        default:
            guard let end = ISO.date(rain.round?.endAt) else { return "红包雨进行中" }
            let left = end.timeIntervalSince(serverNow)
            return left <= 0 ? "本场已结束" : "本场剩余 " + Self.clock(left)
        }
    }

    static func clock(_ t: TimeInterval) -> String {
        let s = max(0, Int(t.rounded(.up)))
        return String(format: "%02d:%02d:%02d", s / 3600, s / 60 % 60, s % 60)
    }
}
