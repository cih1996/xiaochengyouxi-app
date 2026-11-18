import SwiftUI

/// 首次启动：欢迎页（对照 Android welcome()）+ 隐私同意（对照 confirmPrivacy()）。
/// 隐私说明须与实际集成的 SDK 一致：接入广告 SDK / DeviceTrust iOS 版时，更新 PrivacyText 并提升 version，
/// 已同意旧版本的用户会再确认一次（与 Android privacy_accepted_v4 的做法相同）。
enum PrivacyText {
    static let version = 1
    static let key = "privacy_accepted_ios_v\(version)"
    static let body = """
    广西奇易电子商务有限公司为你提供服务。请阅读《隐私政策》和《用户协议》，点击“同意并继续”即表示你同意以下处理：

    1. 账号服务：注册、登录需要手机号和短信验证码；登录凭证保存在本机钥匙串中。开启面容 ID 登录后，账号密码仅保存在本机、仅能由你的面容解锁，不会上传。

    2. 账户安全：为识别作弊、保护账户与资金安全，会收集设备型号、系统版本、网络状态，以及你在本应用内的签到、领取、提现等业务操作记录。

    3. 通知提醒：仅在本机安排红包雨、签到、转盘的本地提醒，不使用推送服务收集你的信息。

    4. 收款信息：绑定微信收款码时，图片只在手机上识别，仅上传收款码内容；支付宝账号与实名仅用于提现打款。

    你可以在“我的 → 设置”中查看隐私政策和第三方 SDK 清单，也可以随时申请注销账号。
    """
}

struct WelcomeView: View {
    let onContinue: () -> Void
    @Environment(\.openURL) private var openURL
    @State private var appear = false
    @State private var bob = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 6) {
                Image(systemName: "gift").foregroundColor(Theme.orange)
                Text("小橙有喜").font(.system(size: 22, weight: .bold)).foregroundColor(Theme.text)
            }
            .padding(.top, 24)
            Spacer(minLength: 10)
            ZStack {
                Circle().fill(Theme.peach).frame(width: 230, height: 230).scaleEffect(appear ? 1 : 0.6)
                Image("packet").resizable().scaledToFit().frame(width: 84)
                    .rotationEffect(.degrees(-14)).offset(x: -92, y: 46 + (bob ? -6 : 6))
                Image("mascot").resizable().scaledToFit().frame(width: 190)
                    .offset(y: bob ? -5 : 5)
                CoinIcon(size: 34).offset(x: 96, y: -64 + (bob ? 6 : -6))
                CoinIcon(size: 22).offset(x: 110, y: 30 + (bob ? -4 : 4))
            }
            .opacity(appear ? 1 : 0)
            .frame(height: 250)
            Spacer(minLength: 10)
            Text("把小欢喜\n装进每一天").font(.system(size: 31, weight: .heavy)).foregroundColor(Theme.text)
                .multilineTextAlignment(.center).lineSpacing(5)
                .opacity(appear ? 1 : 0).offset(y: appear ? 0 : 16)
            Text("一点小收获，一整天好心情").font(.footnote).foregroundColor(Theme.secondary).padding(.top, 8)
            HStack(spacing: 7) {
                benefit("gift", "领福利"); benefit("bag", "逛好物"); benefit("yensign.circle", "攒金币")
            }
            .padding(.top, 16)
            Button(action: onContinue) {
                Text("登录 / 注册").font(.system(size: 18, weight: .bold)).foregroundColor(.white)
                    .frame(maxWidth: .infinity).frame(height: 54)
                    .background(Theme.button, in: RoundedRectangle(cornerRadius: 27))
            }
            .padding(.top, 18)
            HStack(spacing: 10) {
                Button("隐私政策") { open("/privacy") }
                Text("·").foregroundColor(Color(hex: 0xCFB9AA))
                Button("用户协议") { open("/terms") }
            }
            .font(.caption).foregroundColor(Theme.secondary).padding(.top, 8).padding(.bottom, 12)
        }
        .padding(.horizontal, 27)
        .background(Theme.background.ignoresSafeArea())
        .onAppear {
            withAnimation(.spring(response: 0.7, dampingFraction: 0.75)) { appear = true }
            withAnimation(.easeInOut(duration: 1.8).repeatForever(autoreverses: true)) { bob = true }
        }
    }

    private func benefit(_ icon: String, _ text: String) -> some View {
        HStack(spacing: 4) {
            Image(systemName: icon).font(.system(size: 14)).foregroundColor(Color(hex: 0xD27D42))
            Text(text).font(.caption).foregroundColor(Color(hex: 0x93715A))
        }
        .frame(maxWidth: .infinity).frame(height: 36).background(Color.white, in: RoundedRectangle(cornerRadius: 16))
    }

    private func open(_ path: String) {
        if let u = URL(string: path, relativeTo: AppConfig.apiBaseURL) { openURL(u.absoluteURL) }
    }
}

/// 隐私同意：不可下滑关闭；"不同意"只停留在说明页（iOS 不允许 App 自行退出）。
struct PrivacyConsentView: View {
    let onAgree: () -> Void
    @Environment(\.openURL) private var openURL
    @State private var declined = false

    var body: some View {
        VStack(spacing: 0) {
            Text("欢迎来到小橙有喜").font(.title3.bold()).foregroundColor(Theme.text).padding(.top, 26)
            ScrollView {
                Text(declined ? "需要你同意隐私政策后才能使用小橙有喜的服务。你可以再看一看，也可以随时关闭 App。" : PrivacyText.body)
                    .font(.subheadline).foregroundColor(Theme.text.opacity(0.85)).lineSpacing(4)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 24).padding(.vertical, 14)
            }
            HStack(spacing: 18) {
                Button("隐私政策") { open("/privacy") }
                Button("用户协议") { open("/terms") }
            }
            .font(.footnote).foregroundColor(Theme.orange).padding(.bottom, 12)
            Button(action: onAgree) {
                Text("同意并继续").font(.system(size: 17, weight: .bold)).foregroundColor(.white)
                    .frame(maxWidth: .infinity).frame(height: 52)
                    .background(Theme.button, in: RoundedRectangle(cornerRadius: 26))
            }
            .padding(.horizontal, 24)
            Button(declined ? "再看看" : "不同意") { withAnimation { declined.toggle() } }
                .font(.subheadline).foregroundColor(Theme.secondary).padding(.vertical, 14)
        }
        .background(Theme.background.ignoresSafeArea())
        .interactiveDismissDisabled()
    }

    private func open(_ path: String) {
        if let u = URL(string: path, relativeTo: AppConfig.apiBaseURL) { openURL(u.absoluteURL) }
    }
}
