import SwiftUI

struct LedgerView: View {
    @EnvironmentObject private var session: Session
    @State private var items: [LedgerItem] = []
    @State private var filter = 0   // 0 全部 1 收入 2 支出

    private var shown: [LedgerItem] {
        switch filter {
        case 1: return items.filter { $0.coins > 0 }
        case 2: return items.filter { $0.coins < 0 }
        default: return items
        }
    }

    private var groups: [(day: String, income: Int, rows: [LedgerItem])] {
        var out: [(String, Int, [LedgerItem])] = []
        for item in shown {
            let d = Self.dayTitle(item.createdAt)
            if let i = out.firstIndex(where: { $0.0 == d }) {
                out[i].2.append(item)
                if item.coins > 0 { out[i].1 += item.coins }
            } else {
                out.append((d, max(0, item.coins), [item]))
            }
        }
        return out.map { (day: $0.0, income: $0.1, rows: $0.2) }
    }

    var body: some View {
        SubPage(title: "金币明细") {
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    summary
                    picker
                    ForEach(groups, id: \.day) { g in
                        HStack {
                            Text(g.day).font(.subheadline.bold()).foregroundColor(Theme.text)
                            Spacer()
                            if g.income > 0 { Text("收入 \(g.income)").font(.footnote).foregroundColor(Theme.secondary) }
                        }
                        .padding(.horizontal, 6).padding(.top, 4)
                        VStack(spacing: 0) {
                            ForEach(Array(g.rows.enumerated()), id: \.element.id) { i, row in
                                if i > 0 { Divider().padding(.leading, 76) }
                                LedgerRow(item: row)
                            }
                        }
                        .card(radius: 20)
                    }
                    if items.isEmpty {
                        Text("还没有金币记录").font(.footnote).foregroundColor(Theme.hint)
                            .frame(maxWidth: .infinity).padding(.top, 40)
                    }
                }
                .padding(.horizontal, 14).padding(.bottom, 20)
            }
            .refreshable { await load() }
        }
        .task { await load() }
    }

    private var summary: some View {
        let u = session.user
        return VStack(alignment: .leading, spacing: 8) {
            Text("当前金币").font(.subheadline).foregroundColor(.white.opacity(0.85))
            HStack {
                CoinIcon(size: 34)
                Text("\(u?.coins ?? 0)").font(.system(size: 40, weight: .bold)).foregroundColor(.white).monospacedDigit()
                Spacer()
                NavigationLink(destination: WithdrawView()) {
                    Text("去提现").font(.subheadline.bold()).foregroundColor(Theme.orange)
                        .padding(.horizontal, 20).frame(height: 42).background(Color.white, in: Capsule())
                }
            }
            Text(String(format: "约 ¥%.2f", u?.cashYuan ?? 0)).font(.footnote).foregroundColor(.white.opacity(0.85))
            Divider().background(Color.white.opacity(0.4)).padding(.vertical, 6)
            HStack {
                stat("\(u?.todayEarned ?? 0)", "今日收入")
                stat("\(u?.totalEarned ?? 0)", "累计收入")
            }
        }
        .padding(22)
        .background(Theme.hero, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
    }

    private func stat(_ v: String, _ t: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(v).font(.title3.bold()).foregroundColor(.white)
            Text(t).font(.footnote).foregroundColor(.white.opacity(0.85))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var picker: some View {
        HStack(spacing: 0) {
            ForEach(Array(["全部", "收入", "支出"].enumerated()), id: \.offset) { i, t in
                Button { withAnimation(.easeOut(duration: 0.2)) { filter = i } } label: {
                    Text(t).font(.subheadline.weight(filter == i ? .bold : .regular))
                        .foregroundColor(filter == i ? Theme.orange : Theme.secondary)
                        .frame(maxWidth: .infinity).frame(height: 44)
                        .background(filter == i ? Color.white : .clear, in: RoundedRectangle(cornerRadius: 14))
                }
            }
        }
        .padding(5)
        .background(Color(hex: 0xF1E6DC), in: RoundedRectangle(cornerRadius: 18))
    }

    private func load() async {
        if let list: [LedgerItem] = try? await APIClient.shared.get("/ledger") { items = list }
        await session.refreshMe()
    }

    private static let dayFormatter: DateFormatter = {
        let f = DateFormatter()
        f.locale = Locale(identifier: "zh_CN")
        f.timeZone = TimeZone(identifier: "Asia/Shanghai")
        f.dateFormat = "M月d日 EEEE"
        return f
    }()
    static let timeFormatter: DateFormatter = {
        let f = DateFormatter()
        f.timeZone = TimeZone(identifier: "Asia/Shanghai")
        f.dateFormat = "HH:mm"
        return f
    }()
    static func dayTitle(_ iso: String) -> String {
        ISO.date(iso).map { dayFormatter.string(from: $0) } ?? ""
    }
}

private struct LedgerRow: View {
    let item: LedgerItem
    var body: some View {
        let style = Self.style(item.kind)
        HStack(spacing: 14) {
            Image(systemName: style.icon).font(.system(size: 16)).foregroundColor(style.color)
                .frame(width: 42, height: 42).background(style.color.opacity(0.12), in: Circle())
            VStack(alignment: .leading, spacing: 3) {
                Text(item.title).foregroundColor(Theme.text)
                Text(ISO.date(item.createdAt).map { LedgerView.timeFormatter.string(from: $0) } ?? "")
                    .font(.caption).foregroundColor(Theme.hint)
            }
            Spacer()
            Text(item.coins > 0 ? "+\(item.coins)" : "\(item.coins)")
                .font(.title3.bold()).foregroundColor(item.coins > 0 ? Theme.orange : Theme.text)
        }
        .padding(.horizontal, 18).frame(height: 76)
    }

    static func style(_ kind: String) -> (icon: String, color: Color) {
        switch kind {
        case "wheel": return ("bell", Color(hex: 0x9C4DB8))
        case "checkin": return ("checkmark", Color(hex: 0xF08A3C))
        case "withdraw": return ("arrow.up.right", Color(hex: 0x3D8B5A))
        case "refund": return ("arrow.uturn.backward", Color(hex: 0x3D8B5A))
        case "invite": return ("person.2", Color(hex: 0x3B7DD8))
        case "rain": return ("cloud.rain", Color(hex: 0xE04848))
        case "video": return ("play.rectangle", Color(hex: 0xE0603A))
        default: return ("gift", Color(hex: 0xE0484E))
        }
    }
}
