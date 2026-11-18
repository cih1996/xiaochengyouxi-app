import SwiftUI

@main
struct XiaoChengApp: App {
    @StateObject private var session = Session()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(session)
                .tint(Theme.orange)
                .modifier(ToastOverlay())
                .preferredColorScheme(.light)
        }
    }
}

struct RootView: View {
    @EnvironmentObject private var session: Session

    var body: some View {
        Group {
            if session.token == nil {
                LoginView()
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
