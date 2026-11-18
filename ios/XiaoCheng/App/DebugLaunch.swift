#if DEBUG
import Foundation

/// 仅 Debug：模拟器截图自动化用的启动参数（Release 不编译）。
/// xcrun simctl launch <sim> <bundle> -XCLogin 手机号:验证码 -XCScreen hall|shop|me|settings|ledger|login-code
///   -XCPassword 账号:密码  用密码登录（会触发面容 ID 提示）
///   -XCBiometric accept    自动点提示条上的"开启"；reset 清掉已开启/已拒绝状态
///   -XCLogout 1            启动后退出登录（验证面容 ID 登录按钮）
enum DebugLaunch {
    static func value(_ key: String) -> String? { UserDefaults.standard.string(forKey: key) }
    static var screen: String? { value("XCScreen") }

    @MainActor
    static func run(_ session: Session) async {
        if value("XCBiometric") == "reset" {
            session.setBiometric(enabled: false)
            UserDefaults.standard.removeObject(forKey: "biometric.declined")
        }
        if value("XCLogout") == "1", session.token != nil { await session.logout() }
        if let login = value("XCLogin"), session.token == nil {
            let p = login.split(separator: ":", maxSplits: 1).map(String.init)
            if p.count == 2 { try? await session.login(phone: p[0], code: p[1]) }
        }
        if let login = value("XCPassword"), session.token == nil {
            let p = login.split(separator: ":", maxSplits: 1).map(String.init)
            if p.count == 2 { try? await session.login(phone: p[0], password: p[1]) }
            if value("XCBiometric") == "accept", session.biometricOffer != nil {
                try? await Task.sleep(nanoseconds: 2_500_000_000)   // 先让提示条露出来，方便截图
                await session.acceptBiometricOffer()
            }
        }
    }
}
#endif
