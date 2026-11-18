import SwiftUI

struct MainTabView: View {
    @EnvironmentObject private var session: Session

    var body: some View {
        tabs
            .overlay(alignment: .bottom) {
                if session.biometricOffer != nil {
                    BiometricOfferBanner()
                        .padding(.horizontal, 12)
                        .padding(.bottom, 58)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }
            }
            .animation(.spring(response: 0.35, dampingFraction: 0.85), value: session.biometricOffer != nil)
    }

    private var tabs: some View {
        TabView {
            HallView().tabItem { Label("大厅", systemImage: "bubble.left.and.bubble.right") }
            NavigationView {
                WebActivityView(url: AppConfig.webBaseURL.appendingPathComponent("activity/"))
                    .navigationTitle("活动").navigationBarTitleDisplayMode(.inline)
            }
            .tabItem { Label("活动", systemImage: "gift") }
            ProfileView().tabItem { Label("我的", systemImage: "person") }
        }
    }
}

struct HallView: View {
    var body: some View {
        NavigationView {
            VStack(spacing: 12) {
                Image(systemName: "hammer").font(.largeTitle).foregroundColor(Theme.orange)
                Text("大厅开发中").foregroundColor(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Theme.background.ignoresSafeArea())
            .navigationTitle("大厅")
        }
    }
}

struct ProfileView: View {
    @EnvironmentObject private var session: Session
    @State private var askPassword = false

    var body: some View {
        NavigationView {
            List {
                if let u = session.user {
                    Section {
                        LabeledRow("昵称", u.nickname ?? "-")
                        LabeledRow("手机号", u.phone)
                        LabeledRow("金币", "\(u.coins)")
                        LabeledRow("今日收益", "\(u.todayEarned ?? 0)")
                        LabeledRow("邀请码", u.inviteCode ?? "-")
                    }
                } else {
                    ProgressView()
                }
                if BiometricVault.kind != nil {
                    Section(footer: Text("开启后，退出或登录过期时可用\(BiometricVault.name)直接登录。")) {
                        if session.biometricEnabled {
                            Toggle(isOn: Binding(get: { true }, set: { if !$0 { session.setBiometric(enabled: false) } })) {
                                Label("\(BiometricVault.name)登录", systemImage: BiometricVault.symbol)
                            }
                        } else {
                            Button { askPassword = true } label: {
                                Label("开启\(BiometricVault.name)登录", systemImage: BiometricVault.symbol)
                            }
                        }
                    }
                }
                Section {
                    LabeledRow("版本", "\(AppConfig.version) (\(AppConfig.build))")
                    Button("退出登录", role: .destructive) { Task { await session.logout() } }
                }
            }
            .navigationTitle("我的")
            .refreshable { await session.refreshMe() }
            .sheet(isPresented: $askPassword) { EnableBiometricSheet() }
        }
    }
}

private struct LabeledRow: View {
    let title: String
    let value: String
    init(_ title: String, _ value: String) { self.title = title; self.value = value }
    var body: some View {
        HStack { Text(title); Spacer(); Text(value).foregroundColor(.secondary) }
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
