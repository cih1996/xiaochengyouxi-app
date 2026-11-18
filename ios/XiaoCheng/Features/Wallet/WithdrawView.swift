import SwiftUI

/// 对照 Android 0.3.24 提现页（微信钱包风格，灰底白卡、绿色主按钮）。
/// 只能选择后台"提现档位"里的固定金额；收款账户在"管理收款账户"里绑定，提现时不再填写。
struct WithdrawView: View {
    @EnvironmentObject private var session: Session
    @State private var options: WithdrawOptions?
    @State private var accounts: PayoutAccounts?
    @State private var channel: String?
    @State private var tierId: String?
    @State private var busy = false
    @State private var showAccounts = false
    @State private var done: Double?

    private let green = Color(hex: 0x07C160)
    private var tier: WithdrawOptions.Tier? { options?.tiers.first { $0.id == tierId } }
    private func bound(_ ch: String) -> Bool {
        let a = ch == "wechat" ? accounts?.wechat : accounts?.alipay
        return a?.account?.isEmpty == false
    }
    private var canSubmit: Bool { channel != nil && tier?.available == true && !busy }

    var body: some View {
        VStack(spacing: 0) {
            BackHeader(title: "提现")
            ScrollView {
                VStack(spacing: 16) {
                    VStack(alignment: .leading, spacing: 0) {
                        Text("提现到").font(.subheadline).foregroundColor(Theme.secondary).padding(.bottom, 10)
                        channelRow("wechat", "微信")
                        Divider().padding(.leading, 56)
                        channelRow("alipay", "支付宝")
                    }
                    .padding(20).background(Color.white, in: RoundedRectangle(cornerRadius: 16))

                    VStack(alignment: .leading, spacing: 10) {
                        Text("提现金额").font(.subheadline).foregroundColor(Theme.secondary)
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            Text("¥").font(.system(size: 26, weight: .bold))
                            Text(String(format: "%.2f", options?.cashYuan ?? session.user?.cashYuan ?? 0))
                                .font(.system(size: 44, weight: .bold)).monospacedDigit()
                        }
                        .foregroundColor(Theme.text)
                        Text("当前可提现余额").font(.footnote).foregroundColor(Theme.secondary)
                        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 10), count: 3), spacing: 10) {
                            ForEach(options?.tiers ?? []) { t in tierCell(t) }
                        }
                        .padding(.top, 6)
                    }
                    .padding(20).frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.white, in: RoundedRectangle(cornerRadius: 16))

                    Button(action: submit) {
                        ZStack {
                            if busy { ProgressView().tint(.white) }
                            else { Text(tier.map { "提现 \($0.label)" } ?? "提现").font(.system(size: 18, weight: .bold)) }
                        }
                        .foregroundColor(.white).frame(maxWidth: .infinity).frame(height: 54)
                        .background(green.opacity(canSubmit ? 1 : 0.45), in: RoundedRectangle(cornerRadius: 10))
                    }
                    .disabled(!canSubmit)
                    .padding(.top, 10)

                    Button("管理收款账户") { showAccounts = true }
                        .font(.subheadline).foregroundColor(Color(hex: 0x576B95))
                    Text("提现申请提交后需人工审核，预计 1–3 个工作日到账。")
                        .font(.caption).foregroundColor(Theme.hint).frame(maxWidth: .infinity, alignment: .leading)
                }
                .padding(14)
            }
        }
        .background(Color(hex: 0xEDEDED).ignoresSafeArea())
        .navigationBarHidden(true)
        .hideTabBar()
        .task { await load() }
        .sheet(isPresented: $showAccounts, onDismiss: { Task { await load() } }) {
            NavigationView { PayoutAccountView() }
        }
        .alert("提现申请已提交", isPresented: Binding(get: { done != nil }, set: { if !$0 { done = nil } })) {
            Button("好的") {}
        } message: {
            Text("人工审核后打款到你的收款账户，可在“提现记录”查看进度。")
        }
    }

    private func channelRow(_ ch: String, _ name: String) -> some View {
        Button {
            if bound(ch) { channel = ch } else { showAccounts = true }
        } label: {
            HStack(spacing: 14) {
                ChannelIcon(channel: ch)
                VStack(alignment: .leading, spacing: 3) {
                    Text(name).font(.body.bold()).foregroundColor(Theme.text)
                    if bound(ch) {
                        let a = ch == "wechat" ? accounts?.wechat : accounts?.alipay
                        Text([a?.name, a?.account].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " · "))
                            .font(.footnote).foregroundColor(Theme.secondary)
                    } else {
                        Text("未绑定，点击去绑定").font(.footnote).foregroundColor(Theme.orange)
                    }
                }
                Spacer()
                Image(systemName: channel == ch ? "checkmark.circle.fill" : "circle")
                    .font(.system(size: 24)).foregroundColor(channel == ch ? green : Color(hex: 0xC8C8C8))
            }
            .padding(.vertical, 10).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func tierCell(_ t: WithdrawOptions.Tier) -> some View {
        let selected = t.id == tierId
        return Button { if t.available { tierId = t.id } else { Toast.shared.show(t.reason ?? "暂不可提现") } } label: {
            VStack(spacing: 4) {
                Text(t.label).font(.system(size: 19, weight: .bold))
                    .foregroundColor(t.available ? Theme.text : Color(hex: 0xBBBBBB))
                Text(t.available ? (t.limit ?? "") : (t.reason ?? ""))
                    .font(.caption2).foregroundColor(t.available ? Theme.secondary : Theme.orange)
                    .lineLimit(1).minimumScaleFactor(0.8)
            }
            .frame(maxWidth: .infinity).frame(height: 76)
            .background(selected ? green.opacity(0.08) : Color(hex: 0xF5F5F5), in: RoundedRectangle(cornerRadius: 10))
            .overlay(RoundedRectangle(cornerRadius: 10).stroke(selected ? green : Color(hex: 0xE6E6E6), lineWidth: selected ? 1.5 : 1))
        }
        .buttonStyle(.plain)
    }

    private func load() async {
        async let o: WithdrawOptions? = try? APIClient.shared.get("/withdraw/options")
        async let a: PayoutAccounts? = try? APIClient.shared.get("/payout-account")
        options = await o
        accounts = await a
        if channel == nil { channel = bound("wechat") ? "wechat" : bound("alipay") ? "alipay" : nil }
        if tierId == nil { tierId = options?.tiers.first { $0.available }?.id }
    }

    private func submit() {
        guard let channel, let tier else { return }
        busy = true
        Task {
            defer { busy = false }
            do {
                struct Body: Encodable { let amount_yuan: Double; let channel: String; let account: String }
                let _: EmptyJSON = try await APIClient.shared.post("/withdrawals",
                    Body(amount_yuan: tier.amountYuan, channel: channel, account: ""))
                done = tier.amountYuan
                await session.refreshMe()
                await load()
            } catch {
                Toast.shared.show(error.localizedDescription)
            }
        }
    }
}

/// 只关心是否成功、不解析内容的响应
struct EmptyJSON: Decodable {}
