import SwiftUI

/// 邀请好友：App 内绘制与 Android 同款海报（邀请码 + 二维码），分享时渲染成图片。
struct InviteView: View {
    @State private var summary: InviteSummary?
    @State private var maxCoins: Int?
    @State private var sharing: [Any]?

    var body: some View {
        SubPage(title: "邀请好友") {
            ScrollView {
                VStack(spacing: 16) {
                    if let s = summary {
                        InvitePoster(code: s.inviteCode, url: s.inviteUrl, maxCoins: maxCoins)
                            .clipShape(RoundedRectangle(cornerRadius: 22, style: .continuous))
                            .shadow(color: Theme.orange.opacity(0.15), radius: 12, y: 6)
                        stats(s)
                        HStack(spacing: 12) {
                            Button {
                                UIPasteboard.general.string = s.inviteCode
                                Toast.shared.show("邀请码已复制")
                            } label: {
                                Text("复制邀请码").font(.body.bold()).foregroundColor(Theme.orange)
                                    .frame(maxWidth: .infinity).frame(height: 52)
                                    .background(Theme.cream, in: RoundedRectangle(cornerRadius: 26))
                            }
                            Button { share(s) } label: {
                                Text("分享海报").font(.body.bold()).foregroundColor(.white)
                                    .frame(maxWidth: .infinity).frame(height: 52)
                                    .background(Theme.button, in: RoundedRectangle(cornerRadius: 26))
                            }
                        }
                    } else {
                        ProgressView().padding(.top, 120)
                    }
                }
                .padding(14)
            }
        }
        .task { await load() }
        .sheet(isPresented: Binding(get: { sharing != nil }, set: { if !$0 { sharing = nil } })) {
            ShareSheet(items: sharing ?? [])
        }
    }

    private func stats(_ s: InviteSummary) -> some View {
        HStack {
            stat("\(s.invitedCount ?? 0)", "已邀请")
            Divider().frame(height: 34)
            stat("\(s.registeredCount ?? 0)", "已注册")
            Divider().frame(height: 34)
            stat("\(s.pendingCount ?? 0)", "待注册")
        }
        .padding(.vertical, 16).card(radius: 20)
    }

    private func stat(_ v: String, _ t: String) -> some View {
        VStack(spacing: 4) {
            Text(v).font(.title2.bold()).foregroundColor(Theme.text)
            Text(t).font(.footnote).foregroundColor(Theme.secondary)
        }
        .frame(maxWidth: .infinity)
    }

    private func load() async {
        summary = try? await APIClient.shared.get("/invite/summary")
        struct Newbie: Decodable { let invitedMax: Int? }
        maxCoins = (try? await APIClient.shared.get("/newbie", as: Newbie.self))?.invitedMax
    }

    private func share(_ s: InviteSummary) {
        let poster = InvitePoster(code: s.inviteCode, url: s.inviteUrl, maxCoins: maxCoins).frame(width: 360)
        var items: [Any] = []
        if let img = Snapshot.image(of: poster, width: 360) { items.append(img) }
        items.append("我在小橙有喜领到了新人大红包，邀请码 \(s.inviteCode)，点链接下载，你也能拆一个！\(s.inviteUrl)")
        sharing = items
    }
}

struct InvitePoster: View {
    let code: String
    let url: String
    let maxCoins: Int?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Text("橙").font(.system(size: 15, weight: .heavy)).foregroundColor(.white)
                    .frame(width: 28, height: 28).background(Theme.orange, in: RoundedRectangle(cornerRadius: 7))
                Text("小橙有喜").font(.system(size: 16, weight: .bold)).foregroundColor(Theme.text)
            }
            Text("好友送你一个").font(.system(size: 28, weight: .heavy)).foregroundColor(Theme.text).padding(.top, 18)
            Text("新人大红包").font(.system(size: 40, weight: .black)).foregroundColor(Theme.orange)
            if let m = maxCoins, m > 0 {
                Text("最高 \(m.formatted()) 金币 · 可提现").font(.system(size: 14, weight: .bold)).foregroundColor(.white)
                    .padding(.horizontal, 14).padding(.vertical, 7).background(Theme.hero, in: Capsule())
                    .padding(.top, 8)
            }
            HStack(alignment: .bottom) {
                Image("packet").resizable().scaledToFit().frame(width: 110)
                Spacer()
                Image("mascot").resizable().scaledToFit().frame(width: 140)
            }
            .padding(.top, 6)
            VStack(spacing: 6) {
                Text("邀请码 · 注册时填写").font(.caption).foregroundColor(Theme.secondary)
                Text(code.map(String.init).joined(separator: " ")).font(.system(size: 30, weight: .heavy, design: .rounded))
                    .foregroundColor(Theme.text)
            }
            .frame(maxWidth: .infinity).padding(.vertical, 14).background(Color.white, in: RoundedRectangle(cornerRadius: 16))
            .padding(.top, 10)
            VStack(spacing: 10) {
                if let qr = QRDecoder.make(url) {
                    Image(uiImage: qr).interpolation(.none).resizable().frame(width: 150, height: 150)
                }
                Text("长按识别二维码，领取新人红包").font(.caption).foregroundColor(Theme.secondary)
            }
            .frame(maxWidth: .infinity).padding(.vertical, 16).background(Color.white, in: RoundedRectangle(cornerRadius: 16))
            .padding(.top, 10)
        }
        .padding(22)
        .background(LinearGradient(colors: [Color(hex: 0xFFF3EA), Color(hex: 0xFFDCC3)], startPoint: .top, endPoint: .bottom))
    }
}

enum Snapshot {
    /// iOS 15 兼容：用 UIHostingController 把 SwiftUI 视图渲染成图片
    static func image<V: View>(of view: V, width: CGFloat) -> UIImage? {
        let host = UIHostingController(rootView: view.environment(\.colorScheme, .light))
        let size = host.sizeThatFits(in: CGSize(width: width, height: .greatestFiniteMagnitude))
        host.view.bounds = CGRect(origin: .zero, size: size)
        host.view.backgroundColor = .clear
        let format = UIGraphicsImageRendererFormat()
        format.scale = 3
        return UIGraphicsImageRenderer(size: size, format: format).image { _ in
            host.view.drawHierarchy(in: host.view.bounds, afterScreenUpdates: true)
        }
    }
}

struct ShareSheet: UIViewControllerRepresentable {
    let items: [Any]
    func makeUIViewController(context: Context) -> UIActivityViewController {
        UIActivityViewController(activityItems: items, applicationActivities: nil)
    }
    func updateUIViewController(_ vc: UIActivityViewController, context: Context) {}
}
