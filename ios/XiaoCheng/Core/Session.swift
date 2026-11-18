import Foundation

@MainActor
final class Session: ObservableObject {
    @Published private(set) var token: String?
    @Published var user: User?
    @Published var lastError: String?
    /// 刚用密码登录成功、可以提示开启面容 ID 时，暂存凭据（只在内存里）。
    @Published var biometricOffer: BiometricVault.Credentials?
    @Published private(set) var biometricEnabled = BiometricVault.isEnabled
    /// 主动退出后，登录页不自动弹面容识别，只显示按钮。
    @Published private(set) var explicitLogout = false

    init() {
        token = Keychain.get("token")
        APIClient.shared.token = token
    }

    func bootstrap() async {
        guard token != nil else { return }
        await refreshMe()
    }

    func login(phone: String, password: String) async throws {
        let r: AuthResponse = try await APIClient.shared.post(
            "/auth/login", ["phone": phone, "password": password])
        save(token: r.token)
        user = r.user
        explicitLogout = false
        let creds = BiometricVault.Credentials(phone: phone, password: password)
        if biometricEnabled {
            // 密码可能改过：已开启时用最新的密码更新钥匙串条目，保证下次面容登录可用
            try? await BiometricVault.enable(creds)
        } else if BiometricVault.kind != nil && !BiometricVault.declined(phone) {
            biometricOffer = creds
        }
    }

    /// 返回 false 表示用户取消，或者没有可用的面容凭据。
    func loginWithBiometrics() async throws -> Bool {
        guard let creds = try await BiometricVault.unlock() else { return false }
        do {
            let r: AuthResponse = try await APIClient.shared.post(
                "/auth/login", ["phone": creds.phone, "password": creds.password])
            save(token: r.token)
            user = r.user
            explicitLogout = false
            return true
        } catch let e as APIError where e.status == 401 {
            // 密码已在别处修改：作废面容凭据，回到密码登录
            setBiometric(enabled: false)
            throw APIError(status: 401, message: "密码已变更，请用新密码登录一次", code: nil)
        }
    }

    func acceptBiometricOffer() async {
        guard let creds = biometricOffer else { return }
        biometricOffer = nil
        do {
            try await BiometricVault.enable(creds)
            biometricEnabled = true
        } catch {
            // 用户拒绝系统授权或识别失败：视为不开启，不再追问
            BiometricVault.markDeclined(creds.phone)
        }
    }

    func declineBiometricOffer() {
        if let p = biometricOffer?.phone { BiometricVault.markDeclined(p) }
        biometricOffer = nil
    }

    func setBiometric(enabled: Bool) {
        if !enabled { BiometricVault.disable() }
        biometricEnabled = BiometricVault.isEnabled
    }

    func refreshMe() async {
        do {
            user = try await APIClient.shared.get("/me")
        } catch let e as APIError where e.status == 401 {
            save(token: nil)
        } catch {
            lastError = error.localizedDescription
        }
    }

    func logout() async {
        _ = try? await APIClient.shared.post("/auth/logout", [String: String](), as: OK.self)
        save(token: nil)
        user = nil
        biometricOffer = nil
        explicitLogout = true
    }

    private func save(token: String?) {
        self.token = token
        APIClient.shared.token = token
        Keychain.set("token", token)
    }
}
