import SwiftUI

/// 对照 Android 0.3.24 登录页：品牌行 → 账号登录 → 账号/密码 → 协议勾选 → 登录 → 验证码登录/忘记密码 → 注册。
/// iOS 额外：账号字段标为 username，可直接用钥匙串自动填充；开启后显示面容 ID 登录。
struct LoginView: View {
    @EnvironmentObject private var session: Session
    @Environment(\.openURL) private var openURL
    @State private var account = ""
    @State private var password = ""
    @State private var showPassword = false
    @State private var agreed = false
    @State private var busy = false
    @State private var error: String?
    @State private var triedAuto = false
    @State private var shakeAgreement = 0
    @FocusState private var focus: Field?
    enum Field { case account, password }

    private var canSubmit: Bool { !account.trimmingCharacters(in: .whitespaces).isEmpty && password.count >= 8 }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: 12) {
                    Image("mascot").resizable().scaledToFit().frame(width: 64, height: 64)
                        .background(Theme.peach, in: RoundedRectangle(cornerRadius: 18))
                    VStack(alignment: .leading, spacing: 4) {
                        Text("小橙有喜").font(.headline).foregroundColor(Theme.text)
                        Text("自在生活 · 天天有礼").font(.footnote).foregroundColor(Theme.secondary)
                    }
                }
                .padding(.top, 50)

                Text("账号登录").font(.system(size: 32, weight: .heavy)).foregroundColor(Theme.text).padding(.top, 34)
                Text("手机号或用户名均可登录").font(.subheadline).foregroundColor(Theme.secondary).padding(.top, 8)

                VStack(spacing: 16) {
                    InputBox(focused: focus == .account) {
                        TextField("手机号 / 用户名", text: $account)
                            .textContentType(.username)
                            .keyboardType(.asciiCapable)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .focused($focus, equals: .account)
                            .submitLabel(.next)
                            .onSubmit { focus = .password }
                    }
                    InputBox(focused: focus == .password) {
                        HStack {
                            Group {
                                if showPassword { TextField("请输入密码", text: $password) }
                                else { SecureField("请输入密码", text: $password) }
                            }
                            .textContentType(.password)
                            .focused($focus, equals: .password)
                            .submitLabel(.go)
                            .onSubmit(submit)
                            Button { showPassword.toggle() } label: {
                                Image(systemName: showPassword ? "eye" : "eye.slash").foregroundColor(Theme.hint)
                            }
                        }
                    }
                }
                .padding(.top, 28)

                agreement.padding(.top, 18)
                    .modifier(Shake(animatableData: CGFloat(shakeAgreement)))

                if let error {
                    Text(error).font(.footnote).foregroundColor(.red).padding(.top, 12)
                }

                Button(action: submit) {
                    ZStack {
                        if busy { ProgressView().tint(.white) } else { Text("登录").font(.system(size: 18, weight: .bold)) }
                    }
                    .foregroundColor(.white)
                    .frame(maxWidth: .infinity).frame(height: 56)
                    .background(Theme.button, in: RoundedRectangle(cornerRadius: 28))
                    .opacity(canSubmit ? 1 : 0.45)
                }
                .disabled(busy || !canSubmit)
                .padding(.top, 22)

                if session.biometricEnabled {
                    Button(action: biometricLogin) {
                        Label("\(BiometricVault.name)登录", systemImage: BiometricVault.symbol)
                            .font(.body.weight(.semibold)).foregroundColor(Theme.orange)
                            .frame(maxWidth: .infinity).frame(height: 50)
                            .overlay(RoundedRectangle(cornerRadius: 25).stroke(Theme.orange.opacity(0.5)))
                    }
                    .disabled(busy)
                    .padding(.top, 12)
                }

                HStack {
                    Button("验证码登录") { Toast.shared.show("iOS 版正在赶来") }
                        .font(.body.bold()).foregroundColor(Theme.orange)
                    Spacer()
                    Button("忘记密码？") { Toast.shared.show("iOS 版正在赶来") }
                        .foregroundColor(Theme.secondary)
                }
                .padding(.top, 20)

                HStack(spacing: 6) {
                    Text("还没有账号？").foregroundColor(Theme.secondary)
                    Button("立即注册") { Toast.shared.show("iOS 版正在赶来") }.font(.body.bold()).foregroundColor(Theme.orange)
                }
                .frame(maxWidth: .infinity).padding(.top, 36)
            }
            .padding(.horizontal, 24)
        }
        .scrollDismissesKeyboardCompat()
        .background(Theme.background.ignoresSafeArea())
        .task {
            // 冷启动或 token 过期时自动弹面容识别；主动退出后只显示按钮
            if session.biometricEnabled && !session.explicitLogout && !triedAuto {
                triedAuto = true
                biometricLogin()
            }
        }
    }

    private var agreement: some View {
        HStack(spacing: 8) {
            Button { agreed.toggle() } label: {
                Image(systemName: agreed ? "checkmark.square.fill" : "square")
                    .font(.system(size: 22)).foregroundColor(Theme.orange)
            }
            (Text("我已阅读并同意").foregroundColor(Theme.secondary)
             + Text("《用户协议》").foregroundColor(Theme.orange)
             + Text("和").foregroundColor(Theme.secondary)
             + Text("《隐私政策》").foregroundColor(Theme.orange))
                .font(.footnote)
                .onTapGesture { agreed.toggle() }
            Spacer()
        }
        .contextMenu {
            Button("查看用户协议") { open("/terms") }
            Button("查看隐私政策") { open("/privacy") }
        }
    }

    private func open(_ path: String) {
        if let u = URL(string: path, relativeTo: AppConfig.apiBaseURL) { openURL(u.absoluteURL) }
    }

    private func submit() {
        guard canSubmit, !busy else { return }
        guard agreed else {
            withAnimation(.default) { shakeAgreement += 1 }
            Toast.shared.show("请先阅读并同意用户协议和隐私政策")
            return
        }
        busy = true
        error = nil
        focus = nil
        Task {
            defer { busy = false }
            do { try await session.login(phone: account.trimmingCharacters(in: .whitespaces), password: password) }
            catch { self.error = error.localizedDescription }
        }
    }

    private func biometricLogin() {
        busy = true
        error = nil
        Task {
            defer { busy = false }
            do { _ = try await session.loginWithBiometrics() }
            catch { self.error = error.localizedDescription }
        }
    }
}

private struct InputBox<Content: View>: View {
    let focused: Bool
    @ViewBuilder var content: Content
    var body: some View {
        content
            .font(.body)
            .padding(.horizontal, 20).frame(height: 60)
            .background(Color.white, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous)
                .stroke(focused ? Theme.orange.opacity(0.6) : Color(hex: 0xF0DED2), lineWidth: 1.2))
    }
}

struct Shake: GeometryEffect {
    var animatableData: CGFloat
    func effectValue(size: CGSize) -> ProjectionTransform {
        ProjectionTransform(CGAffineTransform(translationX: 8 * sin(animatableData * .pi * 4), y: 0))
    }
}

extension View {
    @ViewBuilder func scrollDismissesKeyboardCompat() -> some View {
        if #available(iOS 16.0, *) { scrollDismissesKeyboard(.interactively) } else { self }
    }
}
