import AirkeysCore
import Foundation
import Observation

/// The language the core draws in (`view.language`), set by the shell with each view. The
/// shell's own words follow it, not the system, so the shell and the core never disagree on a
/// screen. Observable: a view that reads it is drawn again when it changes.
@Observable
final class ShellLanguage {
    static let shared = ShellLanguage()
    var current: Language = .en
}

/// `tr("English", "中文")`: one of the shell's own words, in the core's language. Main thread.
func tr(_ en: String, _ zh: String) -> String {
    ShellLanguage.shared.current == .zh ? zh : en
}

extension Language {
    /// The app's language: Chinese for any Chinese language (any region or script), else
    /// English. The language iOS chose for the app (its own setting in Settings › Airkeys, or the
    /// system's), or the person's first language.
    static var system: Language {
        let chosen = [Bundle.main.preferredLocalizations.first, Locale.preferredLanguages.first]
        let chinese = chosen.contains { $0.map { Locale(identifier: $0).language.languageCode == .chinese } ?? false }
        return chinese ? .zh : .en
    }
}
