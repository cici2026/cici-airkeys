import AirkeysCore
import SwiftUI

@main
struct AirkeysApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
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
            .onChange(of: phase) { _, now in shell.sceneActive(now == .active) }
            .onReceive(NotificationCenter.default.publisher(for: UIScreen.capturedDidChangeNotification)) { _ in
                captured = UIScreen.main.isCaptured
            }
            .onReceive(NotificationCenter.default.publisher(for: NSLocale.currentLocaleDidChangeNotification)) { _ in
                shell.followSystemLanguage()
            }
        }
    }
}

/// Third-party keyboards can log keystrokes and send them once the phone is back online: in
/// Airkeys a recovery phrase is typed with Apple's keyboard only.
final class AppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        shouldAllowExtensionPointIdentifier extensionPointIdentifier: UIApplication.ExtensionPointIdentifier
    ) -> Bool {
        extensionPointIdentifier != .keyboard
    }
}
