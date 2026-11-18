import SwiftUI

struct ProfileView: View {
    @EnvironmentObject private var session: Session

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 14) {
                    PageHeader(title: "我的") {
                        NavigationLink(destination: SettingsView()) {
                            HStack(spacing: 2) { Text("设置"); Image(systemName: "chevron.right").font(.caption) }
                                .font(.body).foregroundColor(Theme.secondary)
                        }
                    }
                    HStack(spacing: 14) {
                        Image("mascot").resizable().scaledToFit().frame(width: 64, height: 64)
                            .background(Theme.peach, in: RoundedRectangle(cornerRadius: 16))
                        VStack(alignment: .leading, spacing: 4) {
                            Text(session.user?.nickname ?? "小橙同学").font(.title3.bold()).foregroundColor(Theme.text)
                            Text(session.user?.maskedPhone ?? "").font(.subheadline).foregroundColor(Theme.secondary)
                        }
                        Spacer()
                    }
                    .padding(.horizontal, 16)
                    CoinsCard(user: session.user)
                    NavigationLink(destination: InviteView()) { InviteBanner() }.buttonStyle(.plain)
                    VStack(spacing: 0) {
                        MenuRow(icon: "clock", title: "提现记录", destination: WithdrawRecordsView())
                        Divider().padding(.leading, 56)
                        MenuRow(icon: "checkmark.shield", title: "收款账户", destination: PayoutAccountView())
                        Divider().padding(.leading, 56)
                        MenuRow(icon: "yensign.circle", title: "金币明细", destination: LedgerView())
                    }
                    .card(radius: 20).padding(.horizontal, 14)
                    VStack(spacing: 0) {
                        MenuRow(icon: "shield", title: "帮助与反馈", destination: ComingSoon(title: "帮助与反馈"))
                    }
                    .card(radius: 20).padding(.horizontal, 14)
                    Text("小小的收获，大大的开心").font(.caption).foregroundColor(Theme.hint).padding(.vertical, 8)
                }
            }
            .background(Theme.background.ignoresSafeArea())
            .navigationBarHidden(true)
            .refreshable { await session.refreshMe() }
            .background(debugLinks)
        }
        .navigationViewStyle(.stack)
    }
}

extension ProfileView {
    /// Debug 启动参数直达二级页（截图用）
    @ViewBuilder var debugLinks: some View {
        #if DEBUG
        NavigationLink(destination: SettingsView(), isActive: .constant(DebugLaunch.screen == "settings")) { EmptyView() }
        NavigationLink(destination: LedgerView(), isActive: .constant(DebugLaunch.screen == "ledger")) { EmptyView() }
        NavigationLink(destination: WithdrawView(), isActive: .constant(DebugLaunch.screen == "withdraw")) { EmptyView() }
        NavigationLink(destination: PayoutAccountView(), isActive: .constant(DebugLaunch.screen == "payout")) { EmptyView() }
        NavigationLink(destination: InviteView(), isActive: .constant(DebugLaunch.screen == "invite")) { EmptyView() }
        NavigationLink(destination: WithdrawRecordsView(), isActive: .constant(DebugLaunch.screen == "records")) { EmptyView() }
        NavigationLink(destination: AccountDeletionView(), isActive: .constant(DebugLaunch.screen == "deletion")) { EmptyView() }
        #endif
    }
}

private struct CoinsCard: View {
    let user: User?
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("我的金币").font(.subheadline).foregroundColor(.white.opacity(0.85))
                Spacer()
                NavigationLink(destination: LedgerView()) {
                    Text("收益明细 ›").font(.subheadline).foregroundColor(.white.opacity(0.9))
                }
            }
            Text(user.map { "\($0.coins)" } ?? "--").font(.system(size: 54, weight: .bold)).foregroundColor(.white)
                .monospacedDigit()
            Text(String(format: "约 ¥%.2f", user?.cashYuan ?? 0)).font(.subheadline).foregroundColor(.white.opacity(0.85))
            HStack {
                Text("今日 \(user?.todayEarned ?? 0) 金币").frame(maxWidth: .infinity, alignment: .leading)
                Text("累计 \(user?.totalEarned ?? 0) 金币").frame(maxWidth: .infinity, alignment: .leading)
            }
            .font(.subheadline).foregroundColor(.white.opacity(0.85)).padding(.top, 6)
            NavigationLink(destination: WithdrawView()) {
                Text("去提现").font(.system(size: 18, weight: .bold)).foregroundColor(Theme.orange)
                    .frame(maxWidth: .infinity).frame(height: 54)
                    .background(Theme.cream, in: RoundedRectangle(cornerRadius: 27))
            }
            .padding(.top, 6)
        }
        .padding(22)
        .background(Theme.hero, in: RoundedRectangle(cornerRadius: 26, style: .continuous))
        .padding(.horizontal, 14)
    }
}

private struct InviteBanner: View {
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text("邀请好友").font(.title3.bold()).foregroundColor(Theme.text)
                Text("分享小欢喜，一起领福利").font(.footnote).foregroundColor(Theme.secondary)
            }
            Spacer()
            Image("packet").resizable().scaledToFit().frame(width: 52, height: 52)
            Image(systemName: "chevron.right").foregroundColor(Theme.secondary)
        }
        .padding(.horizontal, 22).padding(.vertical, 14)
        .background(Theme.peach, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
        .padding(.horizontal, 14)
    }
}

struct MenuRow<D: View>: View {
    let icon: String
    let title: String
    let destination: D
    var body: some View {
        NavigationLink(destination: destination) {
            HStack(spacing: 14) {
                Image(systemName: icon).font(.system(size: 18)).foregroundColor(Color(hex: 0xB86B3C)).frame(width: 26)
                Text(title).font(.body).foregroundColor(Theme.text)
                Spacer()
                Image(systemName: "chevron.right").font(.footnote).foregroundColor(Theme.hint)
            }
            .padding(.horizontal, 18).frame(height: 62)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

/// 二级页面通用骨架：返回栏 + 米色底，隐藏系统导航栏（Android 风格）
struct SubPage<Content: View>: View {
    let title: String
    @ViewBuilder var content: Content
    var body: some View {
        VStack(spacing: 0) {
            BackHeader(title: title)
            content
        }
        .background(Theme.background.ignoresSafeArea())
        .navigationBarHidden(true)
        .hideTabBar()
    }
}

extension View {
    /// 二级页隐藏底部 tab（Android 二级页没有底部导航）
    @ViewBuilder func hideTabBar() -> some View {
        if #available(iOS 16.0, *) { toolbar(.hidden, for: .tabBar) } else { self }
    }
}

struct ComingSoon: View {
    let title: String
    var body: some View {
        SubPage(title: title) {
            VStack(spacing: 12) {
                Image("mascot").resizable().scaledToFit().frame(width: 120)
                Text("iOS 版正在赶来").foregroundColor(Theme.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

