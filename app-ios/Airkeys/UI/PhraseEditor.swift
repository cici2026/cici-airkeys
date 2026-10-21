import SwiftUI
import UIKit

/// The recovery-phrase field. Each edit goes to the core at once, and the field takes the
/// core's text back in the same callback — a completed word replaces the typed letters before
/// the next keystroke can land on them. No autocorrect, no smart punctuation, no suggestions,
/// no keyboard learning.
/// A range of the phrase to select (a word tapped in the grid); a new value is a new request.
struct PhraseSelection: Equatable {
    let range: NSRange
    let stamp = UUID()
}

struct PhraseEditor: UIViewRepresentable {
    let text: String
    let placeholder: String
    /// Sends the edited text; returns the core's text.
    let rewrite: (String) -> String
    /// Selected when it changes, so typing replaces it.
    var select: PhraseSelection? = nil

    func makeCoordinator() -> Coordinator { Coordinator(rewrite: rewrite) }

    func makeUIView(context: Context) -> UITextView {
        let v = UITextView()
        v.delegate = context.coordinator
        v.backgroundColor = .clear
        v.textColor = UIColor(Ink.text)
        v.tintColor = UIColor(Ink.lamp)
        v.font = .monospacedSystemFont(ofSize: 17, weight: .regular)
        v.autocorrectionType = .no
        v.autocapitalizationType = .none
        v.spellCheckingType = .no
        v.smartQuotesType = .no
        v.smartDashesType = .no
        v.smartInsertDeleteType = .no
        v.inlinePredictionType = .no
        v.keyboardType = .asciiCapable
        v.textContentType = .oneTimeCode
        v.isScrollEnabled = false
        v.textContainerInset = .zero
        v.textContainer.lineFragmentPadding = 0
        v.accessibilityIdentifier = placeholder
        v.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        v.text = text
        DispatchQueue.main.async { v.becomeFirstResponder() }
        return v
    }

    func updateUIView(_ v: UITextView, context: Context) {
        context.coordinator.rewrite = rewrite
        if v.text != text { v.text = text }
        if let select, select != context.coordinator.selected {
            context.coordinator.selected = select
            if NSMaxRange(select.range) <= (v.text as NSString).length {
                v.becomeFirstResponder()
                v.selectedRange = select.range
            }
        }
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: UITextView, context: Context) -> CGSize? {
        let width = proposal.width ?? 320
        let h = uiView.sizeThatFits(CGSize(width: width, height: .greatestFiniteMagnitude)).height
        return CGSize(width: width, height: max(h, 3 * 22))
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var rewrite: (String) -> String
        var selected: PhraseSelection?
        init(rewrite: @escaping (String) -> String) { self.rewrite = rewrite }

        func textViewDidChange(_ v: UITextView) {
            let typed = v.text ?? ""
            let canonical = rewrite(typed)
            if canonical != typed {
                v.text = canonical
                v.selectedRange = NSRange(location: (canonical as NSString).length, length: 0)
            }
        }
    }
}
