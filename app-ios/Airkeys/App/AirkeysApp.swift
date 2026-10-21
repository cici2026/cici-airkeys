import AirkeysCore
import SwiftUI

@main
struct AirkeysApp: App {
    @State private var shell = Shell()
    @Environment(\.scenePhase) private var phase
    @State private var captured = UIScreen.main.isCaptured

    var body: some Scene {
        WindowGroup {
            ZStack {
                RootView(shell: shell)
                // the app switcher's snapshot, and screen recording while a phrase shows,
                // see the mark instead of the wallet (which stays open in the background)
                if phase != .active || (shell.privateScreen && captured) {
                    Ink.bg.ignoresSafeArea()
                    Mark(size: 84)
                }
            }
            .preferredColorScheme(.dark)
            .onAppear { shell.start() }
            .onReceive(NotificationCenter.default.publisher(for: UIScreen.capturedDidChangeNotification)) { _ in
                captured = UIScreen.main.isCaptured
            }
        }
    }
}
