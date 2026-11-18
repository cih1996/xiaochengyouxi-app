import SwiftUI

struct MainTabView: View {
    @EnvironmentObject private var session: Session
    @EnvironmentObject private var reminders: Reminders
    @StateObject private var rain = RainController()
    @State private var tab = 0

    init() {
        let a = UITabBarAppearance()
        a.configureWithOpaqueBackground()
        a.backgroundColor = .white
        a.shadowColor = UIColor(Theme.divider)
        for item in [a.stackedLayoutAppearance, a.inlineLayoutAppearance, a.compactInlineLayoutAppearance] {
            item.normal.iconColor = UIColor(Theme.secondary)
            item.normal.titleTextAttributes = [.foregroundColor: UIColor(Theme.secondary)]
            item.selected.iconColor = UIColor(Theme.orange)
            item.selected.titleTextAttributes = [.foregroundColor: UIColor(Theme.orange)]
        }
        UITabBar.appearance().standardAppearance = a
        UITabBar.appearance().scrollEdgeAppearance = a
    }

    var body: some View {
        TabView(selection: $tab) {
            HallView().tag(0)
                .tabItem { Label("大厅", systemImage: tab == 0 ? "bubble.left.fill" : "bubble.left") }
            ShopView().tag(1)
                .tabItem { Label("捡漏", systemImage: tab == 1 ? "bag.fill" : "bag") }
            ProfileView().tag(2)
                .tabItem { Label("我的", systemImage: tab == 2 ? "person.fill" : "person") }
        }
        .overlay(alignment: .bottom) {
            if session.biometricOffer != nil {
                BiometricOfferBanner()
                    .padding(.horizontal, 12)
                    .padding(.bottom, 58)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(.spring(response: 0.35, dampingFraction: 0.85), value: session.biometricOffer != nil)
        .environmentObject(rain)
        .overlay { RainLayer(rain: rain, inHall: tab == 0) }
        .onAppear { rain.session = session; rain.start() }
        .onDisappear { rain.stop() }
        .task {
            #if DEBUG
            switch DebugLaunch.screen {
            case "shop": tab = 1
            case "me", "settings", "ledger", "withdraw", "payout", "invite", "records", "deletion": tab = 2
            case "rain": rain.startPreview()
            default: break
            }
            #endif
            await session.refreshMe()
            // 登录后静默申请临时通知授权（不弹框），供本地提醒使用
            await reminders.requestProvisionalIfNeeded()
        }
        .onChange(of: reminders.pendingTarget) { t in if t != nil { tab = 0 } }
    }
}

/// 密码登录成功后的轻提示：一行文案 + 开启按钮，点"开启"即弹系统面容授权，同意就完成。
struct BiometricOfferBanner: View {
    @EnvironmentObject private var session: Session

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: BiometricVault.symbol).font(.title2).foregroundColor(Theme.orange)
            VStack(alignment: .leading, spacing: 2) {
                Text("下次用\(BiometricVault.name)登录").font(.subheadline.bold())
                Text("看一眼就进来，不用输密码").font(.caption).foregroundColor(.secondary)
            }
            Spacer(minLength: 4)
            Button("开启") { Task { await session.acceptBiometricOffer() } }
                .font(.subheadline.bold())
                .padding(.horizontal, 14).padding(.vertical, 7)
                .background(Theme.orange).foregroundColor(.white).clipShape(Capsule())
            Button { session.declineBiometricOffer() } label: {
                Image(systemName: "xmark").font(.caption.bold()).foregroundColor(.secondary)
            }
            .accessibilityLabel("以后再说")
        }
        .padding(12)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
        .shadow(color: .black.opacity(0.12), radius: 10, y: 4)
        .task {
            // 不打断操作：8 秒没处理就自己收起，下次密码登录再提示
            try? await Task.sleep(nanoseconds: 8_000_000_000)
            if session.biometricOffer != nil { session.biometricOffer = nil }
        }
    }
}

/// 在"我的"里手动开启：当前没有保存密码，需要输入一次确认。
struct EnableBiometricSheet: View {
    @EnvironmentObject private var session: Session
    @Environment(\.dismiss) private var dismiss
    @State private var password = ""
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        NavigationView {
            Form {
                Section(footer: Text(error ?? "验证一次登录密码，之后可用\(BiometricVault.name)直接登录。")
                    .foregroundColor(error == nil ? .secondary : .red)) {
                    SecureField("登录密码", text: $password).textContentType(.password)
                }
            }
            .navigationTitle("开启\(BiometricVault.name)登录").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("取消") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("开启") { submit() }.disabled(busy || password.count < 8)
                }
            }
        }
    }

    private func submit() {
        guard let phone = session.user?.phone else { return }
        busy = true
        error = nil
        Task {
            defer { busy = false }
            do {
                // 先向服务端核对密码，再写入面容钥匙串
                try await session.login(phone: phone, password: password)
                session.biometricOffer = BiometricVault.Credentials(phone: phone, password: password)
                await session.acceptBiometricOffer()
                if session.biometricEnabled { dismiss() } else { error = "未完成\(BiometricVault.name)验证" }
            } catch {
                self.error = error.localizedDescription
            }
        }
    }
}

/// 红包雨各层：雨层（仅大厅）、开场倒计时、本场结算
private struct RainLayer: View {
    @ObservedObject var rain: RainController
    let inHall: Bool

    var body: some View {
        TimelineView(.periodic(from: .now, by: 0.5)) { ctx in
            ZStack {
                if inHall && rain.skyVisible(at: ctx.date) {
                    RainSkyView(rain: rain).transition(.opacity)
                }
                if inHall, let n = rain.countdown(at: ctx.date) {
                    Color.black.opacity(0.35).ignoresSafeArea().onTapGesture { rain.dismissCountdown() }
                    RainCountdownCard(seconds: n) { withAnimation { rain.dismissCountdown() } }
                        .transition(.scale(scale: 0.9).combined(with: .opacity))
                }
                if let s = rain.summary {
                    RainSummaryCard(packets: s.packets, coins: s.coins) { rain.summary = nil }
                }
            }
            .animation(.easeOut(duration: 0.3), value: rain.skyVisible(at: ctx.date))
        }
    }
}
