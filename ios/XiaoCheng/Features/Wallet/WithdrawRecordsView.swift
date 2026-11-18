import SwiftUI

struct WithdrawRecordsView: View {
    @State private var rows: [Withdrawal] = []
    @State private var loaded = false

    var body: some View {
        SubPage(title: "提现记录") {
            ScrollView {
                VStack(spacing: 12) {
                    ForEach(rows) { r in row(r) }
                    if loaded && rows.isEmpty {
                        VStack(spacing: 10) {
                            Image("mascot").resizable().scaledToFit().frame(width: 110)
                            Text("还没有提现记录").font(.footnote).foregroundColor(Theme.hint)
                        }
                        .padding(.top, 80)
                    }
                }
                .padding(14)
            }
            .refreshable { await load() }
        }
        .task { await load() }
    }

    private func row(_ r: Withdrawal) -> some View {
        let color: Color = r.state == "done" ? Color(hex: 0x07A65A) : r.state == "returned" ? Theme.secondary : Theme.orange
        return VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("提现到\(r.channelText ?? "")").font(.body.bold()).foregroundColor(Theme.text)
                Spacer()
                Text(String(format: "¥%.2f", r.amountYuan ?? 0)).font(.title3.bold()).foregroundColor(Theme.text)
            }
            HStack {
                Text([r.account, ISO.date(r.createdAt).map { Self.fmt.string(from: $0) }].compactMap { $0 }.joined(separator: " · "))
                    .font(.footnote).foregroundColor(Theme.secondary)
                Spacer()
                Text(r.status ?? "").font(.footnote.bold()).foregroundColor(color)
            }
            if let n = r.note { Text(n).font(.caption).foregroundColor(Theme.hint) }
        }
        .padding(18).card(radius: 18)
    }

    private func load() async {
        if let list: [Withdrawal] = try? await APIClient.shared.get("/withdrawals") { rows = list }
        loaded = true
    }

    private static let fmt: DateFormatter = {
        let f = DateFormatter()
        f.timeZone = TimeZone(identifier: "Asia/Shanghai")
        f.dateFormat = "M月d日 HH:mm"
        return f
    }()
}
