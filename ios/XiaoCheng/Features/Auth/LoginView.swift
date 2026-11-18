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
    @State private var codeMode = false
    @State private var code = ""
    @State private var countdown = 0
    @State private var flow: AuthFlowView.Mode?
    @State private var captchaQuestion: (id: String, question: String)?
    @State private var captchaAnswer = ""
    @FocusState private var focus: Field?
    enum Field { case account, password, code }

    private var trimmed: String { account.trimmingCharacters(in: .whitespaces) }
    private var canSubmit: Bool {
        codeMode ? (trimmed.count == 11 && code.count >= 4) : (!trimmed.isEmpty && password.count >= 6)
    }

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

                Text(codeMode ? "验证码登录" : "账号登录").font(.system(size: 32, weight: .heavy)).foregroundColor(Theme.text).padding(.top, 34)
                Text(codeMode ? "未注册的手机号请先注册" : "手机号或用户名均可登录").font(.subheadline).foregroundColor(Theme.secondary).padding(.top, 8)

                VStack(spacing: 16) {
                    InputBox(focused: focus == .account) {
                        TextField(codeMode ? "请输入手机号" : "手机号 / 用户名", text: $account)
                            .textContentType(codeMode ? .telephoneNumber : .username)
                            .keyboardType(codeMode ? .numberPad : .asciiCapable)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .focused($focus, equals: .account)
                            .submitLabel(.next)
                            .onSubmit { focus = codeMode ? .code : .password }
                    }
                    if codeMode {
                        InputBox(focused: focus == .code) {
                            HStack {
                                // iOS 收到短信后键盘上方直接出现验证码，一点即填
                                TextField("请输入验证码", text: $code)
                                    .textContentType(.oneTimeCode)
                                    .keyboardType(.numberPad)
                                    .focused($focus, equals: .code)
                                    .onChange(of: code) { v in if v.count == 6 { submit() } }
                                Button(action: sendCode) {
                                    Text(countdown > 0 ? "\(countdown)s 后重发" : "获取验证码")
                                        .font(.subheadline.bold())
                                        .foregroundColor(countdown > 0 || trimmed.count != 11 ? Theme.hint : Theme.orange)
                                        .monospacedDigit()
                                }
                                .disabled(countdown > 0 || trimmed.count != 11)
                            }
                        }
                    } else {
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

                if session.biometricEnabled && !codeMode {
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
                    Button(codeMode ? "密码登录" : "验证码登录") {
                        withAnimation(.easeOut(duration: 0.2)) { codeMode.toggle(); error = nil }
                    }
                        .font(.body.bold()).foregroundColor(Theme.orange)
                    Spacer()
                    Button("忘记密码？") { flow = .reset }
                        .foregroundColor(Theme.secondary)
                }
                .padding(.top, 20)

                HStack(spacing: 6) {
                    Text("还没有账号？").foregroundColor(Theme.secondary)
                    Button("立即注册") { flow = .register }.font(.body.bold()).foregroundColor(Theme.orange)
                }
                .frame(maxWidth: .infinity).padding(.top, 36)
            }
            .padding(.horizontal, 24)
        }
        .scrollDismissesKeyboardCompat()
        .alert("安全验证", isPresented: Binding(get: { captchaQuestion != nil }, set: { if !$0 { captchaQuestion = nil } })) {
            TextField("请输入答案", text: $captchaAnswer).keyboardType(.numberPad)
            Button("取消", role: .cancel) {}
            Button("继续") { submitCaptcha() }
        } message: {
            Text(captchaQuestion?.question ?? "")
        }
        .sheet(isPresented: Binding(get: { flow != nil }, set: { if !$0 { flow = nil } })) {
            if let flow { AuthFlowView(mode: flow, presetPhone: trimmed.count == 11 ? trimmed : nil) }
        }
        .background(Theme.background.ignoresSafeArea())
        .task {
            #if DEBUG
            if DebugLaunch.screen == "login-code" { codeMode = true }
            if DebugLaunch.screen == "register" { flow = .register }
            #endif
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
            do {
                if codeMode { try await session.login(phone: trimmed, code: code) }
                else { try await session.login(phone: trimmed, password: password) }
            } catch let e as APIError where e.code == "LOGIN_CAPTCHA_REQUIRED" {
                await requestCaptcha()
            } catch { self.error = error.localizedDescription }
        }
    }

    private func requestCaptcha() async {
        struct C: Decodable { let captchaId: String; let question: String }
        struct R: Decodable { let required: Bool?; let captcha: C? }
        do {
            let r: R = try await APIClient.shared.post("/auth/login/captcha", ["phone": trimmed])
            guard r.required == true else { error = codeMode ? "请重新获取验证码后登录" : "请重试"; return }
            guard let c = r.captcha else { error = "安全验证暂不可用"; return }
            captchaAnswer = ""
            captchaQuestion = (c.captchaId, c.question)
        } catch { self.error = error.localizedDescription }
    }

    private func submitCaptcha() {
        guard let q = captchaQuestion else { return }
        let a = captchaAnswer.trimmingCharacters(in: .whitespaces)
        guard !a.isEmpty else { Toast.shared.show("请输入验证答案"); return }
        session.captcha = (q.id, a)
        captchaQuestion = nil
        submit()
    }

    private func sendCode() {
        countdown = 60
        error = nil
        focus = .code
        Task {
            do {
                try await session.sendLoginCode(phone: trimmed)
                Toast.shared.show("验证码已发送")
                while countdown > 0 {
                    try? await Task.sleep(nanoseconds: 1_000_000_000)
                    countdown -= 1
                }
            } catch {
                countdown = 0
                self.error = error.localizedDescription
            }
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
