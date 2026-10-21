import AirkeysCore
import SwiftUI

/// A screen: a top bar, scrolling content, and actions pinned at the bottom.
struct Page<Content: View, Actions: View, Trailing: View>: View {
    var title: String = ""
    var onBack: (() -> Void)?
    var scroll = true
    @ViewBuilder var trailing: () -> Trailing
    @ViewBuilder var content: () -> Content
    @ViewBuilder var actions: () -> Actions

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                if let onBack {
                    IconAction(symbol: "chevron.left", label: "Back", action: onBack)
                } else {
                    Spacer().frame(width: 12)
                }
                Text(title).font(.system(size: 17, weight: .semibold)).foregroundStyle(Ink.text).lineLimit(1)
                Spacer()
                trailing()
            }
            .padding(.horizontal, 8)
            .frame(height: 52)
            Group {
                if scroll {
                    ScrollView { body(content) }.scrollDismissesKeyboard(.interactively)
                } else {
                    body(content)
                    Spacer(minLength: 0)
                }
            }
            VStack(spacing: 10) { actions() }
                .padding(.horizontal, 20)
                .padding(.vertical, 12)
        }
        .background(Ink.bg.ignoresSafeArea())
    }

    private func body(_ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 16) { content() }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 20)
            .padding(.top, 4)
            .padding(.bottom, 12)
    }
}

extension Page where Trailing == EmptyView {
    init(title: String = "", onBack: (() -> Void)? = nil, scroll: Bool = true, @ViewBuilder content: @escaping () -> Content, @ViewBuilder actions: @escaping () -> Actions) {
        self.init(title: title, onBack: onBack, scroll: scroll, trailing: { EmptyView() }, content: content, actions: actions)
    }
}

struct IconAction: View {
    let symbol: String
    let label: String
    var tint: Color = Ink.text
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol).font(.system(size: 18, weight: .medium)).foregroundStyle(tint).frame(width: 44, height: 44)
        }
        .accessibilityLabel(label)
    }
}

enum ButtonKind { case primary, secondary }

struct AkButton: View {
    let title: String
    var kind: ButtonKind = .primary
    var symbol: String?
    var enabled = true
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                if let symbol { Image(systemName: symbol).font(.system(size: 17, weight: .semibold)) }
                Text(title).font(.system(size: 16, weight: .semibold)).lineLimit(1).minimumScaleFactor(0.8)
            }
            .padding(.horizontal, 12)
            .frame(maxWidth: .infinity, minHeight: 54)
            // disabled: a neutral surface, not a faded colour
            .foregroundStyle(enabled ? fg : Ink.faint)
            .background(RoundedRectangle(cornerRadius: 14).fill(enabled ? bg : Ink.raised))
            .overlay(RoundedRectangle(cornerRadius: 14).stroke(enabled ? border : Ink.stroke, lineWidth: 1))
        }
        .disabled(!enabled)
    }

    private var bg: Color {
        switch kind {
        case .primary: Ink.lamp
        case .secondary: Ink.raised
        }
    }

    private var fg: Color {
        switch kind {
        case .primary: Ink.onLamp
        case .secondary: Ink.text
        }
    }

    private var border: Color {
        switch kind {
        case .primary: .clear
        case .secondary: Ink.stroke
        }
    }
}

/// Press and hold to confirm: an accidental tap cannot sign. Timed by the clock, so it holds
/// with Reduce Motion on too; the bar only shows the progress.
struct HoldButton: View {
    let title: String
    let warn: Bool
    let onConfirm: () -> Void
    @State private var holding = false
    @State private var task: Task<Void, Never>?
    private let seconds = 0.9

    var body: some View {
        let color = warn ? Ink.warn : Ink.lamp
        ZStack(alignment: .leading) {
            // a primary button; while held, a light band fills it
            RoundedRectangle(cornerRadius: 14).fill(color)
            GeometryReader { g in
                Rectangle().fill(Color.white.opacity(0.32)).frame(width: holding ? g.size.width : 0)
                    .animation(holding ? .linear(duration: seconds) : .easeOut(duration: 0.18), value: holding)
            }
            .clipShape(RoundedRectangle(cornerRadius: 14))
            Text(title).font(.system(size: 16, weight: .semibold)).foregroundStyle(Ink.onLamp)
                .frame(maxWidth: .infinity)
        }
        .frame(height: 58)
        .contentShape(Rectangle())
        .gesture(
            DragGesture(minimumDistance: 0)
                .onChanged { _ in
                    guard task == nil else { return }
                    holding = true
                    task = Task { @MainActor in
                        try? await Task.sleep(for: .seconds(seconds))
                        if !Task.isCancelled { onConfirm() }
                    }
                }
                .onEnded { _ in
                    task?.cancel()
                    task = nil
                    holding = false
                }
        )
        // a screen that changes mid-hold takes the hold with it
        .onDisappear {
            task?.cancel()
            task = nil
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(title)
        .accessibilityIdentifier("hold-to-confirm")
        .accessibilityAddTraits(.isButton)
        .accessibilityAction { onConfirm() }
    }
}

struct Card<Content: View>: View {
    var border: Color = Ink.stroke
    var padding: CGFloat = 16
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) { content() }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(padding)
            .background(RoundedRectangle(cornerRadius: 16).fill(Ink.surface))
            .overlay(RoundedRectangle(cornerRadius: 16).stroke(border, lineWidth: 1))
    }
}

/// An address in groups of four, the first and last groups bright (EVM: "0x" set apart).
struct Address: View {
    let address: String
    var size: CGFloat = 15

    var body: some View {
        let prefix = address.hasPrefix("0x") ? "0x" : ""
        let groups = Array(address.dropFirst(prefix.count)).chunked(4).map { String($0) }
        var text = Text(prefix.isEmpty ? "" : "0x ").foregroundColor(Ink.faint)
        for (i, g) in groups.enumerated() {
            let edge = i == 0 || i == groups.count - 1
            text = text + Text(g).foregroundColor(edge ? Ink.text : (i % 2 == 0 ? Ink.dim : Ink.text.opacity(0.8))).fontWeight(edge ? .semibold : .regular)
            if i < groups.count - 1 { text = text + Text(" ") }
        }
        return text.font(.system(size: size, design: .monospaced)).lineSpacing(5).textSelection(.enabled)
    }
}

extension Array {
    func chunked(_ n: Int) -> [[Element]] {
        stride(from: 0, to: count, by: n).map { Array(self[$0 ..< Swift.min($0 + n, count)]) }
    }
}

/// First and last characters with an ellipsis, for lists.
func short(_ address: String, keep: Int = 6) -> String {
    let k = keep + (address.hasPrefix("0x") ? 2 : 0)
    return address.count <= k + keep + 1 ? address : "\(address.prefix(k))…\(address.suffix(keep))"
}

/// The network's own logo (Resources: cryptocurrency-icons 0.18.1, CC0-1.0).
struct NetworkBadge: View {
    let kind: NetworkKind
    var size: CGFloat = 40

    var body: some View {
        Image(kind == .bitcoin ? "logo_btc" : "logo_eth").resizable().frame(width: size, height: size).accessibilityHidden(true)
    }
}

struct Pill: View {
    let text: String
    let color: Color
    var symbol: String?

    var body: some View {
        HStack(spacing: 5) {
            if let symbol { Image(systemName: symbol).font(.system(size: 11, weight: .semibold)) }
            Text(text).font(.system(size: 12, weight: .semibold))
        }
        .foregroundStyle(color)
        .padding(.horizontal, 10)
        .padding(.vertical, 4)
        .background(Capsule().fill(color.opacity(0.13)))
    }
}

/// A message in a tinted card: a warning, a fact, a problem.
struct Notice: View {
    let text: String
    let color: Color
    var symbol = "exclamationmark.triangle"

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: symbol).font(.system(size: 17)).foregroundStyle(color)
            Text(text).font(.system(size: 14)).foregroundStyle(Ink.text.opacity(0.92)).fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 14).fill(color.opacity(0.10)))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(color.opacity(0.32), lineWidth: 1))
    }
}

/// The Airkeys mark: a keyhole inside a ring.
struct Mark: View {
    var size: CGFloat = 72
    var color: Color = Ink.lamp

    var body: some View {
        Canvas { ctx, sz in
            let s = min(sz.width, sz.height) / 54
            ctx.stroke(Path(ellipseIn: CGRect(x: 3 * s, y: 3 * s, width: 48 * s, height: 48 * s)), with: .color(color), lineWidth: 4 * s)
            ctx.fill(Path(ellipseIn: CGRect(x: 19 * s, y: 13.5 * s, width: 16 * s, height: 16 * s)), with: .color(color))
            var p = Path()
            p.move(to: CGPoint(x: 24.4 * s, y: 26 * s))
            p.addLine(to: CGPoint(x: 21.2 * s, y: 40 * s))
            p.addLine(to: CGPoint(x: 32.8 * s, y: 40 * s))
            p.addLine(to: CGPoint(x: 29.6 * s, y: 26 * s))
            p.closeSubpath()
            ctx.fill(p, with: .color(color))
        }
        .frame(width: size, height: size)
    }
}

/// A field for secrets: no autocorrect, no keyboard suggestions or learning.
struct Field: View {
    let placeholder: String
    @Binding var text: String
    var secret = false
    var error = false

    var body: some View {
        Group {
            if secret {
                SecureField("", text: $text, prompt: Text(placeholder).foregroundColor(Ink.faint))
            } else {
                TextField("", text: $text, prompt: Text(placeholder).foregroundColor(Ink.faint))
            }
        }
        .font(.system(size: 17, design: .monospaced))
        .foregroundStyle(Ink.text)
        .tint(Ink.lamp)
        .textInputAutocapitalization(.never)
        .autocorrectionDisabled()
        .keyboardType(.asciiCapable)
        .textContentType(.oneTimeCode)
        .privacySensitive()
        .accessibilityIdentifier(placeholder)
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 14).fill(Ink.surface))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(error ? Ink.danger : Ink.stroke, lineWidth: 1))
    }
}

struct Suggestions: View {
    let words: [String]
    let pick: (String) -> Void

    var body: some View {
        if !words.isEmpty {
            HStack(spacing: 8) {
                ForEach(words, id: \.self) { w in
                    Button { pick(w) } label: {
                        Text(w).font(.system(size: 15, design: .monospaced)).foregroundStyle(Ink.text)
                            .padding(.horizontal, 14).padding(.vertical, 9)
                            .background(RoundedRectangle(cornerRadius: 10).fill(Ink.raised))
                            .overlay(RoundedRectangle(cornerRadius: 10).stroke(Ink.stroke, lineWidth: 1))
                    }
                }
            }
        }
    }
}

struct Label2: View {
    let text: String
    var body: some View {
        Text(text.uppercased()).font(.akLabel).tracking(0.8).foregroundStyle(Ink.faint)
    }
}

struct Radio: View {
    let on: Bool
    var body: some View {
        ZStack {
            Circle().stroke(on ? Ink.lamp : Ink.stroke, lineWidth: 2)
            if on { Circle().fill(Ink.lamp).frame(width: 12, height: 12) }
        }
        .frame(width: 24, height: 24)
    }
}

struct Check: View {
    let on: Bool
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8).fill(on ? Ink.lamp : Ink.raised)
            RoundedRectangle(cornerRadius: 8).stroke(on ? Ink.lamp : Ink.stroke, lineWidth: 1)
            if on { Image(systemName: "checkmark").font(.system(size: 13, weight: .bold)).foregroundStyle(Ink.onLamp) }
        }
        .frame(width: 26, height: 26)
    }
}
