import SwiftUI

@main
struct XiaoChengApp: App {
    @StateObject private var session = Session()
    @StateObject private var reminders = Reminders.shared
    @Environment(\.scenePhase) private var phase

    init() { Reminders.shared.activate() }

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(session)
                .environmentObject(reminders)
                .tint(Theme.orange)
                .modifier(ToastOverlay())
                .preferredColorScheme(.light)
                .onChange(of: phase) { p in
                    switch p {
                    case .active: reminders.enterForeground()
                    case .background: reminders.enterBackground()
                    default: break
                    }
                }
        }
    }
}

struct RootView: View {
    @EnvironmentObject private var session: Session

    var body: some View {
        Group {
            if session.token == nil {
                LoginView()
            } else if session.user?.invitationRequired == true {
                BindInvitationView()
            } else {
                MainTabView()
            }
        }
        .task {
            #if DEBUG
            await DebugLaunch.run(session)
            #endif
            await session.bootstrap()
        }
    }
}
