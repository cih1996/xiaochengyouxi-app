import SwiftUI

/// 注册 / 找回密码 / 修改密码（Android 同一套界面：手机号 + 短信验证码 + 新密码）。
/// 修改密码成功后服务端会撤销全部会话，这里用新密码自动重新登录（已开启面容 ID 的同时更新钥匙串）。
struct AuthFlowView: View {
    enum Mode { case register, reset, change
        var title: String { self == .register ? "注册新账号" : self == .reset ? "找回密码" : "修改密码" }
        var purpose: String { self == .register ? "register" : "reset" }
        var action: String { self == .register ? "注册并登录" : "确认修改" }
    }
    let mode: Mode
    var presetPhone: String? = nil

    @EnvironmentObject private var session: Session
    @Environment(\.dismiss) private var dismiss
    @State private var phone = ""
    @State private var code = ""
    @State private var password = ""
    @State private var showPassword = false
    @State private var agreed = false
    @State private var countdown = 0
    @State private var busy = false
    @State private var error: String?

    private var canSubmit: Bool { phone.count == 11 && code.count >= 4 && password.count >= 8 && !busy }

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text(mode.title).font(.system(size: 30, weight: .heavy)).foregroundColor(Theme.text).padding(.top, 10)
                    Text(mode == .register ? "注册后需绑定好友的邀请码" : "验证手机号后设置新密码")
                        .font(.subheadline).foregroundColor(Theme.secondary)
                    box {
                        TextField("请输入手机号", text: $phone)
                            .keyboardType(.numberPad).textContentType(.telephoneNumber)
                            .disabled(presetPhone != nil)
                    }
                    .padding(.top, 12)
                    box {
                        HStack {
                            TextField("请输入验证码", text: $code).keyboardType(.numberPad).textContentType(.oneTimeCode)
                            Button(action: sendCode) {
                                Text(countdown > 0 ? "\(countdown)s 后重发" : "获取验证码").font(.subheadline.bold())
                                    .foregroundColor(countdown > 0 || phone.count != 11 ? Theme.hint : Theme.orange).monospacedDigit()
                            }
                            .disabled(countdown > 0 || phone.count != 11)
                        }
                    }
                    box {
                        HStack {
                            Group {
                                if showPassword { TextField("设置密码（至少 8 位）", text: $password) }
                                else { SecureField("设置密码（至少 8 位）", text: $password) }
                            }
                            .textContentType(.newPassword)
                            Button { showPassword.toggle() } label: {
                                Image(systemName: showPassword ? "eye" : "eye.slash").foregroundColor(Theme.hint)
                            }
                        }
                    }
                    if mode == .register {
                        HStack(spacing: 8) {
                            Button { agreed.toggle() } label: {
                                Image(systemName: agreed ? "checkmark.square.fill" : "square").font(.system(size: 22)).foregroundColor(Theme.orange)
                            }
                            (Text("我已阅读并同意").foregroundColor(Theme.secondary) + Text("《用户协议》").foregroundColor(Theme.orange)
                             + Text("和").foregroundColor(Theme.secondary) + Text("《隐私政策》").foregroundColor(Theme.orange))
                                .font(.footnote).onTapGesture { agreed.toggle() }
                        }
                    }
                    if let error { Text(error).font(.footnote).foregroundColor(.red) }
                    Button(action: submit) {
                        ZStack {
                            if busy { ProgressView().tint(.white) } else { Text(mode.action).font(.system(size: 18, weight: .bold)) }
                        }
                        .foregroundColor(.white).frame(maxWidth: .infinity).frame(height: 56)
                        .background(Theme.button, in: RoundedRectangle(cornerRadius: 28)).opacity(canSubmit ? 1 : 0.45)
                    }
                    .disabled(!canSubmit)
                    .padding(.top, 6)
                }
                .padding(.horizontal, 24)
            }
            .background(Theme.background.ignoresSafeArea())
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("取消") { dismiss() }.foregroundColor(Theme.secondary) } }
        }
        .onAppear { if let presetPhone { phone = presetPhone } }
    }

    private func box<C: View>(@ViewBuilder _ c: () -> C) -> some View {
        c().padding(.horizontal, 20).frame(height: 60)
            .background(Color.white, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).stroke(Color(hex: 0xF0DED2), lineWidth: 1.2))
    }

    private func sendCode() {
        countdown = 60
        error = nil
        Task {
            do {
                struct R: Decodable { let code: String? }
                let r: R = try await APIClient.shared.post("/auth/sms", ["phone": phone, "purpose": mode.purpose])
                if let c = r.code { code = c }   // 仅本地测试服务器会返回验证码
                Toast.shared.show("验证码已发送")
                while countdown > 0 { try? await Task.sleep(nanoseconds: 1_000_000_000); countdown -= 1 }
            } catch {
                countdown = 0
                self.error = error.localizedDescription
            }
        }
    }

    private func submit() {
        if mode == .register && !agreed {
            Toast.shared.show("请先阅读并同意用户协议和隐私政策"); return
        }
        busy = true
        error = nil
        Task {
            defer { busy = false }
            do {
                switch mode {
                case .register:
                    try await session.register(phone: phone, code: code, password: password)
                case .reset, .change:
                    let _: OK = try await APIClient.shared.post("/auth/reset-password",
                        ["phone": phone, "code": code, "password": password])
                    // 修改后旧会话全部失效：直接用新密码登录
                    try await session.login(phone: phone, password: password)
                    Toast.shared.show("密码已更新")
                }
                dismiss()
            } catch {
                self.error = error.localizedDescription
            }
        }
    }
}

/// 未绑定邀请关系的账号只能绑定或退出（服务端同样拦截）。
struct BindInvitationView: View {
    @EnvironmentObject private var session: Session
    @State private var code = ""
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Image("packet").resizable().scaledToFit().frame(width: 90).padding(.top, 60)
            Text("填写邀请码").font(.system(size: 30, weight: .heavy)).foregroundColor(Theme.text)
            Text("小橙有喜需要好友邀请才能使用，填写好友给你的 6 位邀请码。").font(.subheadline).foregroundColor(Theme.secondary)
            TextField("6 位邀请码", text: $code)
                .font(.system(size: 26, weight: .bold, design: .rounded))
                .textInputAutocapitalization(.characters).autocorrectionDisabled()
                .onChange(of: code) { v in
                    let f = String(v.uppercased().filter { $0.isLetter || $0.isNumber }.prefix(6))
                    if f != v { code = f }
                }
                .padding(.horizontal, 20).frame(height: 64)
                .background(Color.white, in: RoundedRectangle(cornerRadius: 18))
                .padding(.top, 8)
            if let error { Text(error).font(.footnote).foregroundColor(.red) }
            Button(action: bind) {
                ZStack { if busy { ProgressView().tint(.white) } else { Text("确认绑定").font(.system(size: 18, weight: .bold)) } }
                    .foregroundColor(.white).frame(maxWidth: .infinity).frame(height: 56)
                    .background(Theme.button, in: RoundedRectangle(cornerRadius: 28)).opacity(code.count == 6 ? 1 : 0.45)
            }
            .disabled(code.count != 6 || busy)
            Button("退出登录") { Task { await session.logout() } }
                .foregroundColor(Theme.secondary).frame(maxWidth: .infinity).padding(.top, 6)
            Spacer()
        }
        .padding(.horizontal, 24)
        .background(Theme.background.ignoresSafeArea())
        .onAppear {
            // 剪贴板里有邀请码时自动带入（便捷绑定）
            if let s = UIPasteboard.general.hasStrings ? UIPasteboard.general.string : nil,
               let m = s.range(of: "[A-Z0-9]{6}", options: .regularExpression) {
                code = String(s[m])
            }
        }
    }

    private func bind() {
        busy = true
        error = nil
        Task {
            defer { busy = false }
            do {
                let _: EmptyJSON = try await APIClient.shared.post("/invite/bind", ["invite_code": code])
                await session.refreshMe()
            } catch { self.error = error.localizedDescription }
        }
    }
}
