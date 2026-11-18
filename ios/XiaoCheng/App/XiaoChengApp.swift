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
    @AppStorage(PrivacyText.key) private var privacyAccepted = false
    @AppStorage("welcome_seen") private var welcomeSeen = false

    var body: some View {
        Group {
            if !privacyAccepted || (!welcomeSeen && session.token == nil) {
                WelcomeView { withAnimation { welcomeSeen = true } }
                    .sheet(isPresented: .constant(!privacyAccepted)) {
                        PrivacyConsentView { privacyAccepted = true }
                    }
            } else if session.token == nil {
                LoginView()
            } else if session.deletionPending {
                NavigationView { AccountDeletionView(gate: true) }.navigationViewStyle(.stack)
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
            if session.token != nil { await session.checkDeletion() }
        }
        .onChange(of: session.token) { t in if t != nil { Task { await session.checkDeletion() } } }
        .modifier(CelebrationOverlay())
    }
}
