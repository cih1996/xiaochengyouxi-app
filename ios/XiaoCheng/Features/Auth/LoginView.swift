import SwiftUI

struct LoginView: View {
    @EnvironmentObject private var session: Session
    @State private var phone = ""
    @State private var password = ""
    @State private var showPassword = false
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                Text("小橙有喜").font(.largeTitle.bold()).foregroundColor(Theme.orange)
                    .padding(.top, 60)
                VStack(alignment: .leading, spacing: 6) {
                    Text("手机号").font(.subheadline).foregroundColor(.secondary)
                    TextField("请输入手机号", text: $phone)
                        .keyboardType(.numberPad)
                        .textContentType(.username)
                        .padding(12).background(Color.white).cornerRadius(10)
                }
                VStack(alignment: .leading, spacing: 6) {
                    Text("密码").font(.subheadline).foregroundColor(.secondary)
                    HStack {
                        Group {
                            if showPassword { TextField("请输入密码", text: $password) }
                            else { SecureField("请输入密码", text: $password) }
                        }
                        .textContentType(.password)
                        Button { showPassword.toggle() } label: {
                            Image(systemName: showPassword ? "eye.slash" : "eye").foregroundColor(.secondary)
                        }
                    }
                    .padding(12).background(Color.white).cornerRadius(10)
                }
                if let error {
                    Text(error).font(.footnote).foregroundColor(.red)
                }
                Button(action: submit) {
                    HStack {
                        if busy { ProgressView().tint(.white) }
                        Text("登录").bold()
                    }
                    .frame(maxWidth: .infinity).padding(14)
                    .background(Theme.orange).foregroundColor(.white).cornerRadius(24)
                }
                .disabled(busy || phone.count != 11 || password.count < 8)
                if session.biometricEnabled {
                    Button(action: biometricLogin) {
                        Label("\(BiometricVault.name)登录", systemImage: BiometricVault.symbol)
                            .font(.body.weight(.medium))
                    }
                    .disabled(busy)
                    .padding(.top, 4)
                }
                Text("注册、找回密码稍后接入").font(.caption).foregroundColor(.secondary)
            }
            .padding(.horizontal, 24)
        }
        .background(Theme.background.ignoresSafeArea())
        .task {
            // 冷启动或 token 过期时自动弹面容识别；主动退出后只显示按钮
            if session.biometricEnabled && !session.explicitLogout && !triedAuto {
                triedAuto = true
                biometricLogin()
            }
        }
    }

    @State private var triedAuto = false

    private func biometricLogin() {
        busy = true
        error = nil
        Task {
            defer { busy = false }
            do { _ = try await session.loginWithBiometrics() }
            catch { self.error = error.localizedDescription }
        }
    }

    private func submit() {
        busy = true
        error = nil
        Task {
            defer { busy = false }
            do { try await session.login(phone: phone, password: password) }
            catch { self.error = error.localizedDescription }
        }
    }
}
