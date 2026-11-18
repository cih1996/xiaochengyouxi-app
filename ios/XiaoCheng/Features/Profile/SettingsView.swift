import SwiftUI

struct SettingsView: View {
    @EnvironmentObject private var session: Session
    @Environment(\.openURL) private var openURL
    @State private var askPassword = false
    @State private var confirmLogout = false
    @State private var changing = false
    @State private var deleting = false

    var body: some View {
        SubPage(title: "设置") {
            ScrollView {
                VStack(alignment: .leading, spacing: 10) {
                    SectionTitle("账号与安全")
                    VStack(spacing: 0) {
                        if BiometricVault.kind != nil {
                            HStack {
                                Text("\(BiometricVault.name)登录").foregroundColor(Theme.text)
                                Spacer()
                                Toggle("", isOn: Binding(
                                    get: { session.biometricEnabled },
                                    set: { on in if on { askPassword = true } else { session.setBiometric(enabled: false) } }))
                                    .labelsHidden().tint(Theme.orange)
                            }
                            .padding(.horizontal, 22).frame(height: 62)
                            Divider().padding(.leading, 22)
                        }
                        SettingRow(title: "修改密码") { changing = true }
                        Divider().padding(.leading, 22)
                        SettingRow(title: "注销账号", detail: "需审核") { deleting = true }
                    }
                    .card(radius: 20)
                    SectionTitle("关于").padding(.top, 8)
                    VStack(spacing: 0) {
                        SettingRow(title: "检查更新", detail: "当前 \(AppConfig.version)") {
                            Toast.shared.show("已是最新版本")
                        }
                        Divider().padding(.leading, 22)
                        SettingRow(title: "隐私政策") { open("/privacy") }
                        Divider().padding(.leading, 22)
                        SettingRow(title: "第三方 SDK 清单") { open("/privacy#sdk") }
                        Divider().padding(.leading, 22)
                        SettingRow(title: "用户协议") { open("/terms") }
                    }
                    .card(radius: 20)
                    Button { confirmLogout = true } label: {
                        Text("退出登录").font(.body.bold()).foregroundColor(Theme.orange)
                            .frame(maxWidth: .infinity).frame(height: 62).card(radius: 20)
                    }
                    .padding(.top, 20)
                }
                .padding(.horizontal, 14)
            }
        }
        .sheet(isPresented: $askPassword) { EnableBiometricSheet() }
        .sheet(isPresented: $changing) { AuthFlowView(mode: .change, presetPhone: session.user?.phone) }
        .background(NavigationLink(destination: AccountDeletionView(), isActive: $deleting) { EmptyView() })
        .confirmationDialog("确定退出登录？", isPresented: $confirmLogout, titleVisibility: .visible) {
            Button("退出登录", role: .destructive) { Task { await session.logout() } }
        }
    }

    private func open(_ path: String) {
        if let u = URL(string: path, relativeTo: AppConfig.apiBaseURL) { openURL(u.absoluteURL) }
    }
}

private struct SectionTitle: View {
    let text: String
    init(_ text: String) { self.text = text }
    var body: some View {
        Text(text).font(.subheadline).foregroundColor(Theme.secondary).padding(.leading, 6)
    }
}

private struct SettingRow: View {
    let title: String
    var detail: String? = nil
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            HStack {
                Text(title).foregroundColor(Theme.text)
                Spacer()
                if let detail { Text(detail).font(.subheadline).foregroundColor(Theme.secondary) }
                Image(systemName: "chevron.right").font(.footnote).foregroundColor(Theme.hint)
            }
            .padding(.horizontal, 22).frame(height: 62).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}
