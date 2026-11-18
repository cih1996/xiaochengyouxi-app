import Foundation

/// 构建期配置（来自 Config/*.xcconfig → Info.plist），对应 Android 的 BuildConfig。
enum AppConfig {
    private static func string(_ key: String) -> String {
        (Bundle.main.object(forInfoDictionaryKey: key) as? String) ?? ""
    }

    static let apiBaseURL = URL(string: string("OrangeAPIBaseURL"))!
    static let webBaseURL = URL(string: string("OrangeWebBaseURL"))!
    static let privacyURL = URL(string: string("OrangePrivacyURL"))
    static let activityHosts: Set<String> = Set(
        string("OrangeActivityHosts").split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
    )
    static var version: String { string("CFBundleShortVersionString") }
    static var build: String { string("CFBundleVersion") }
}
