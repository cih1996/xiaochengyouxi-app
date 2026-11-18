import SwiftUI

/// 7 天签到（对照 Android showCheckinDialog）：顶部渐变头 + 七天格子 + 补签 + 签到按钮。
struct CheckinSheet: View {
    @ObservedObject var model: HallModel
    @EnvironmentObject private var session: Session
    @Environment(\.dismiss) private var dismiss
    @State private var busy = false

    private var status: CheckinStatus? { model.checkin }
    private var done: Bool { status?.checkedToday ?? (session.user?.checkedInToday ?? false) }
    private var total: Int { status?.days.reduce(0) { $0 + $1.reward } ?? 0 }

    var body: some View {
        VStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text(done ? "今日已签到" : "每日签到").font(.system(size: 26, weight: .heavy)).foregroundColor(.white)
                    Spacer()
                    Button { dismiss() } label: {
                        Image(systemName: "xmark").font(.system(size: 15, weight: .bold)).foregroundColor(.white.opacity(0.9))
                            .frame(width: 32, height: 32).background(Color.white.opacity(0.18), in: Circle())
                    }
                }
                (Text("已连续签到 ").foregroundColor(.white.opacity(0.9))
                 + Text("\(status?.streak ?? 0)").bold().foregroundColor(Color(hex: 0xFFE07A))
                 + Text(" 天  ·  7 天共 \(total) 金币").foregroundColor(.white.opacity(0.9)))
                    .font(.subheadline)
            }
            .padding(22)
            .background(Theme.hero)

            VStack(spacing: 14) {
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4), spacing: 8) {
                    ForEach(status?.days ?? [], id: \.day) { d in dayCell(d) }
                }
                if let m = status?.makeupAvailable, m > 0 {
                    Text("有 \(m) 天漏签，点击“补签”按当天档位补领").font(.caption).foregroundColor(Color(hex: 0xB39583))
                }
                Button(action: checkIn) {
                    ZStack {
                        if busy { ProgressView().tint(.white) }
                        else { Text(done ? "今日已领取，明天再来" : "签到领 \(status?.today?.reward ?? 0) 金币").font(.system(size: 17, weight: .bold)) }
                    }
                    .foregroundColor(done ? Color(hex: 0xB39583) : .white)
                    .frame(maxWidth: .infinity).frame(height: 52)
                    .background(done ? AnyShapeStyle(Color(hex: 0xF6EDE6)) : AnyShapeStyle(Theme.button), in: RoundedRectangle(cornerRadius: 26))
                }
                .disabled(done || busy)
            }
            .padding(18)
        }
        .background(Theme.background)
        .task { await model.refreshCheckin() }
    }

    private func dayCell(_ d: CheckinStatus.Day) -> some View {
        let isToday = d.day == status?.currentDay
        let checked = d.status == "checked"
        let missed = d.status == "missed"
        return VStack(spacing: 5) {
            Text(isToday ? "今天" : "第\(d.day)天").font(.caption2).foregroundColor(isToday ? Theme.orange : Theme.secondary)
            ZStack {
                if checked { Image(systemName: "checkmark.circle.fill").font(.system(size: 26)).foregroundColor(Theme.orange) }
                else { CoinIcon(size: 26).opacity(missed ? 0.45 : 1) }
            }
            .frame(height: 28)
            if missed {
                Button("补签") { makeup(d.day) }.font(.caption2.bold()).foregroundColor(.white)
                    .padding(.horizontal, 8).padding(.vertical, 2).background(Theme.orange, in: Capsule())
            } else {
                Text("+\(d.reward)").font(.caption.bold()).foregroundColor(checked ? Theme.hint : Theme.text)
            }
        }
        .frame(maxWidth: .infinity).frame(height: 84)
        .background(isToday ? Theme.peach : Color.white, in: RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(isToday ? Theme.orange.opacity(0.5) : .clear, lineWidth: 1.2))
    }

    private func checkIn() {
        busy = true
        Task {
            defer { busy = false }
            do {
                struct R: Decodable { let coins: Int?; let reward: Int? }
                let r: R = try await APIClient.shared.post("/check-in", [String: String]())
                await model.refreshCheckin()
                await session.refreshMe()
                Reminders.shared.markCheckedIn()
                Celebration.shared.show(r.reward ?? r.coins ?? status?.today?.reward ?? 0, title: "签到成功")
            } catch { Toast.shared.show(error.localizedDescription) }
        }
    }

    private func makeup(_ day: Int) {
        busy = true
        Task {
            defer { busy = false }
            do {
                struct R: Decodable { let reward: Int?; let coins: Int? }
                let r: R = try await APIClient.shared.post("/check-in/makeup", ["day": day])
                await model.refreshCheckin()
                await session.refreshMe()
                Celebration.shared.show(r.reward ?? r.coins ?? 0, title: "补签成功")
            } catch { Toast.shared.show(error.localizedDescription) }
        }
    }
}
