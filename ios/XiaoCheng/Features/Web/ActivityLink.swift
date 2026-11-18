import SwiftUI

enum ActivityLink: String, Identifiable {
    case newbie, wheel
    var id: String { rawValue }
    var title: String { self == .newbie ? "新人礼包" : "幸运转盘" }
    var url: URL { AppConfig.webBaseURL.appendingPathComponent("activity/\(rawValue).html") }
}

/// 活动页全屏打开，页面自己有返回按钮（调用 OrangeApp.close()）；左上角保留一个兜底关闭按钮。
struct ActivityScreen: View {
    let link: ActivityLink
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        WebActivityView(url: link.url, onClose: { dismiss() })
            .ignoresSafeArea(edges: .bottom)
            .background(Theme.background.ignoresSafeArea())
    }
}
