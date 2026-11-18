import Foundation
import LocalAuthentication
import Security

/// 面容 ID / 触控 ID 快速登录。
/// 账号和密码存在钥匙串里，访问控制设为 .biometryCurrentSet：只有本机录入的面容能取出；
/// 重新录入面容后条目自动失效，需要用密码重新登录一次。
/// 服务端会话（30 天 token）保持原样，这里只负责 token 失效或退出后的"一眨眼重新登录"。
enum BiometricVault {
    struct Credentials: Codable {
        let phone: String
        let password: String
    }

    private static let service = "com.lingzidev.xiaochengyouxi.biometric"
    private static let account = "login"
    private static let declinedKey = "biometric.declined"

    /// 当前设备支持且已录入的生物识别类型；不可用时为 nil。
    static var kind: LABiometryType? {
        let c = LAContext()
        guard c.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: nil) else { return nil }
        return c.biometryType == .none ? nil : c.biometryType
    }

    static var name: String {
        switch kind {
        case .touchID: return "触控 ID"
        default: return "面容 ID"
        }
    }

    static var symbol: String { kind == .touchID ? "touchid" : "faceid" }

    /// 只检查条目是否存在，不会弹出面容识别。
    static var isEnabled: Bool {
        let c = LAContext()
        c.interactionNotAllowed = true
        let q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecUseAuthenticationContext as String: c,
        ]
        let s = SecItemCopyMatching(q as CFDictionary, nil)
        return s == errSecSuccess || s == errSecInteractionNotAllowed
    }

    /// 用户点过"以后再说"的账号不再主动提示，可以在"我的"里手动开启。
    static func declined(_ phone: String) -> Bool {
        (UserDefaults.standard.stringArray(forKey: declinedKey) ?? []).contains(phone)
    }

    static func markDeclined(_ phone: String) {
        var list = UserDefaults.standard.stringArray(forKey: declinedKey) ?? []
        if !list.contains(phone) { list.append(phone) }
        UserDefaults.standard.set(list, forKey: declinedKey)
    }

    /// 先做一次面容验证：首次会弹出系统授权框，同意并识别成功即视为开启。
    /// 识别用的 LAContext 直接交给钥匙串写入，不会再弹第二次。
    static func enable(_ creds: Credentials) async throws {
        let c = LAContext()
        c.localizedCancelTitle = "取消"
        try await c.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics,
                                   localizedReason: "开启后可用\(name)快速登录")
        guard let access = SecAccessControlCreateWithFlags(
            nil, kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly, .biometryCurrentSet, nil)
        else { throw VaultError.unavailable }
        disable()
        let q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrAccessControl as String: access,
            kSecUseAuthenticationContext as String: c,
            kSecValueData as String: try JSONEncoder().encode(creds),
        ]
        let s = SecItemAdd(q as CFDictionary, nil)
        guard s == errSecSuccess else { throw VaultError.keychain(s) }
    }

    /// 弹出面容识别并取出账号密码。用户取消时返回 nil。
    static func unlock() async throws -> Credentials? {
        let c = LAContext()
        c.localizedCancelTitle = "用密码登录"
        c.localizedFallbackTitle = ""
        do {
            try await c.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics,
                                       localizedReason: "使用\(name)登录小橙有喜")
        } catch let e as LAError where [.userCancel, .appCancel, .systemCancel, .userFallback].contains(e.code) {
            return nil
        }
        let q: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecUseAuthenticationContext as String: c,
        ]
        var out: AnyObject?
        let s = SecItemCopyMatching(q as CFDictionary, &out)
        if s == errSecItemNotFound { return nil }
        guard s == errSecSuccess, let data = out as? Data else { throw VaultError.keychain(s) }
        return try JSONDecoder().decode(Credentials.self, from: data)
    }

    static func disable() {
        SecItemDelete([
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ] as CFDictionary)
    }

    enum VaultError: LocalizedError {
        case unavailable
        case keychain(OSStatus)
        var errorDescription: String? {
            switch self {
            case .unavailable: return "本机暂不支持\(BiometricVault.name)"
            case .keychain(let s): return "钥匙串错误（\(s)）"
            }
        }
    }
}
