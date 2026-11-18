#if DEBUG
import Foundation

/// 仅 Debug：模拟器截图自动化用的启动参数（Release 不编译）。
/// xcrun simctl launch <sim> <bundle> -XCLogin 手机号:验证码 -XCScreen hall|shop|me|settings|ledger|login-code
enum DebugLaunch {
    static func value(_ key: String) -> String? { UserDefaults.standard.string(forKey: key) }
    static var screen: String? { value("XCScreen") }

    @MainActor
    static func run(_ session: Session) async {
        guard let login = value("XCLogin"), session.token == nil else { return }
        let parts = login.split(separator: ":", maxSplits: 1).map(String.init)
        guard parts.count == 2 else { return }
        try? await session.login(phone: parts[0], code: parts[1])
    }
}
#endif
