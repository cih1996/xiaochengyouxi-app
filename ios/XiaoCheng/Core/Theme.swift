import SwiftUI

/// 颜色取自 Android 0.3.24 截图（private/ios-ref/android-0.3.24）。
enum Theme {
    static let orange = Color(hex: 0xF0562E)          // 主色：选中 tab、强调文字
    static let background = Color(hex: 0xFFF6F0)      // 暖米色页面底
    static let card = Color.white
    static let text = Color(hex: 0x3A2620)
    static let secondary = Color(hex: 0x9C8478)
    static let hint = Color(hex: 0xB8A79E)
    static let peach = Color(hex: 0xFDE6D6)           // 邀请好友、签到卡底色
    static let cream = Color(hex: 0xFFF1D9)           // 去提现按钮底
    static let gold = Color(hex: 0xF7B733)
    static let divider = Color(hex: 0xF2E8E0)

    /// 金币卡、红包卡的橙红渐变
    static let hero = LinearGradient(colors: [Color(hex: 0xFF8A4C), Color(hex: 0xEE4A2C)],
                                     startPoint: .topLeading, endPoint: .bottomTrailing)
    /// 登录按钮渐变
    static let button = LinearGradient(colors: [Color(hex: 0xFF9A5C), Color(hex: 0xF0562E)],
                                       startPoint: .leading, endPoint: .trailing)
    /// 签到卡 / 捡漏横幅的浅桃渐变
    static let soft = LinearGradient(colors: [Color(hex: 0xFFE3CC), Color(hex: 0xFFF1E6)],
                                     startPoint: .leading, endPoint: .trailing)
    static let goldButton = LinearGradient(colors: [Color(hex: 0xFFD36B), Color(hex: 0xF9A93A)],
                                           startPoint: .top, endPoint: .bottom)
}

extension Color {
    init(hex: UInt32, alpha: Double = 1) {
        self.init(.sRGB,
                  red: Double((hex >> 16) & 0xFF) / 255,
                  green: Double((hex >> 8) & 0xFF) / 255,
                  blue: Double(hex & 0xFF) / 255,
                  opacity: alpha)
    }
}

extension View {
    func card(radius: CGFloat = 18) -> some View {
        background(Theme.card, in: RoundedRectangle(cornerRadius: radius, style: .continuous))
    }
}

/// 金币图标：金色圆 + ¥
struct CoinIcon: View {
    var size: CGFloat = 26
    var body: some View {
        ZStack {
            Circle().fill(LinearGradient(colors: [Color(hex: 0xFFD45A), Color(hex: 0xF5A623)],
                                         startPoint: .top, endPoint: .bottom))
            Circle().stroke(Color(hex: 0xFFE9A6), lineWidth: size * 0.08).padding(size * 0.12)
            Text("¥").font(.system(size: size * 0.5, weight: .bold)).foregroundColor(.white)
        }
        .frame(width: size, height: size)
    }
}

/// 轻量 toast，对应 Android Toast
final class Toast: ObservableObject {
    static let shared = Toast()
    @Published var message: String?
    private var token = 0

    func show(_ text: String) {
        token += 1
        let current = token
        DispatchQueue.main.async {
            withAnimation { self.message = text }
            DispatchQueue.main.asyncAfter(deadline: .now() + 2.2) {
                if current == self.token { withAnimation { self.message = nil } }
            }
        }
    }
}

struct ToastOverlay: ViewModifier {
    @ObservedObject var toast = Toast.shared
    func body(content: Content) -> some View {
        content.overlay(alignment: .center) {
            if let m = toast.message {
                Text(m).font(.subheadline).foregroundColor(.white)
                    .padding(.horizontal, 18).padding(.vertical, 10)
                    .background(Color.black.opacity(0.75), in: Capsule())
                    .transition(.opacity)
                    .allowsHitTesting(false)
            }
        }
    }
}

/// 页面统一的大标题栏（Android：左侧粗体标题，右侧附加内容）
struct PageHeader<Trailing: View>: View {
    let title: String
    @ViewBuilder var trailing: Trailing
    var body: some View {
        HStack {
            Text(title).font(.system(size: 30, weight: .heavy)).foregroundColor(Theme.text)
            Spacer()
            trailing
        }
        .padding(.horizontal, 16).padding(.top, 8).padding(.bottom, 4)
    }
}

/// 二级页面返回栏：‹ 标题
struct BackHeader: View {
    let title: String
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        HStack(spacing: 10) {
            Button { dismiss() } label: {
                Image(systemName: "chevron.left").font(.system(size: 24, weight: .medium))
                    .foregroundColor(Theme.text).frame(width: 32, height: 44)
            }
            Text(title).font(.system(size: 22, weight: .bold)).foregroundColor(Theme.text)
            Spacer()
        }
        .padding(.horizontal, 10)
    }
}

extension View {
    /// 半高弹窗（iOS 16+ 用 detents，iOS 15 全屏 sheet）
    @ViewBuilder func compactSheet() -> some View {
        if #available(iOS 16.0, *) { presentationDetents([.medium, .large]).presentationDragIndicator(.hidden) } else { self }
    }
}
