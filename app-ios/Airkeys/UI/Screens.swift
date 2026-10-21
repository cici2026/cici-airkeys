import AirkeysCore
import SwiftUI

/// Draws the core's view; every tap becomes an action for the core.
struct RootView: View {
    let shell: Shell
    private func act(_ a: Action) { shell.dispatch(a) }

    var body: some View {
        ZStack(alignment: .bottom) {
            Ink.bg.ignoresSafeArea()
            screen
            Toast(text: shell.view.toast) { act(.dismissToast) }
        }
    }

    @ViewBuilder private var screen: some View {
        switch shell.view.screen {
        case let .offline(radios, walletOpen):
            OfflineScreen(radios: radios, walletOpen: walletOpen)
        case .welcome:
            Welcome(version: shell.view.version, act: act)
        case let .import(phrase):
            ImportScreen(p: phrase, act: act, rewrite: shell.phraseInput)
        case let .passphrase(passphrase, problem):
            Page(title: "Passphrase", onBack: { act(.back) }) {
                Text("Passphrase").font(.akTitle).foregroundStyle(Ink.text)
                Text("Only if your wallet has one — most do not. Leave it empty otherwise.").font(.akDim).foregroundStyle(Ink.dim)
                Field(placeholder: "No passphrase", text: Binding(get: { passphrase }, set: { act(.passphraseInput(passphrase: $0)) }), secret: true, error: problem != nil)
                if let problem { Text(problem).font(.akSmall).foregroundStyle(Ink.danger) }
                Notice(text: "A passphrase opens a different wallet from the same words, and any passphrase opens one. Airkeys cannot tell a mistyped passphrase: check the addresses it shows.", color: Ink.dim, symbol: "lock")
            } actions: {
                AkButton(title: "Open wallet", enabled: problem == nil) { act(.open) }
            }
        case let .home(index, accounts):
            Home(index: index, accounts: accounts, act: act)
        case let .account(account):
            AccountScreen(a: account, act: act)
        case let .connect(_, choices, scopes):
            Page(title: "Connect", onBack: { act(.back) }) {
                Text("Show your accounts to the web app").font(.akTitle).foregroundStyle(Ink.text)
                Text("The web app reads balances and prepares transfers. It gets addresses only — never a public key.").font(.akDim).foregroundStyle(Ink.dim)
                Card(padding: 6) {
                    ForEach(scopes, id: \.all) { c in
                        Button { act(.connectScope(all: c.all)) } label: {
                            HStack(spacing: 12) {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(c.title).font(.system(size: 16, weight: .semibold)).foregroundStyle(Ink.text)
                                    Text(c.detail).font(.akSmall).foregroundStyle(Ink.dim).multilineTextAlignment(.leading)
                                }
                                Spacer()
                                Radio(on: c.selected)
                            }
                            .padding(10)
                            .contentShape(Rectangle())
                        }
                    }
                }
                Card(padding: 6) {
                    ForEach(choices, id: \.network) { c in
                        Button { act(.toggleNetwork(network: c.network)) } label: {
                            HStack(spacing: 14) {
                                NetworkBadge(kind: c.network, size: 36)
                                Text(c.name).font(.akBody).foregroundStyle(Ink.text)
                                Spacer()
                                Check(on: c.selected)
                            }
                            .padding(10)
                            .contentShape(Rectangle())
                        }
                    }
                }
            } actions: {
                AkButton(title: "Show code", symbol: "qrcode", enabled: choices.contains { $0.selected }) { act(.showAccounts) }
            }
        case let .code(title, caption, _, message):
            CodeScreen(title: title, caption: caption, message: message, next: shell.codeFrame, act: act)
        case let .scan(title, hint, progress, problem):
            Page(title: title, onBack: { act(.back) }, scroll: false) {
                ZStack {
                    RoundedRectangle(cornerRadius: 20).fill(Color(hex: 0x05060A))
                    if problem == nil && shell.cameraOn {
                        CameraPreview(session: shell.camera.session).clipShape(RoundedRectangle(cornerRadius: 20))
                    }
                    Viewfinder()
                }
                .aspectRatio(0.8, contentMode: .fit)
                if let problem {
                    Notice(text: problem, color: Ink.danger)
                } else if let p = progress, p.needed > 0 {
                    Text("Receiving…").font(.akHeading).foregroundStyle(Ink.text)
                    ProgressView(value: min(1, Double(p.received) / Double(p.needed))).tint(Ink.lamp)
                } else {
                    Text(hint).font(.akDim).foregroundStyle(Ink.dim)
                }
            } actions: {
                if problem != nil { AkButton(title: "Try again", symbol: "viewfinder") { act(.scan) } }
            }
        case let .review(review):
            ReviewScreen(r: review, act: act)
        case let .refused(title, message):
            Page(onBack: { act(.back) }) {
                VStack(spacing: 14) {
                    Image(systemName: "exclamationmark.triangle").font(.system(size: 46)).foregroundStyle(Ink.danger)
                    Text(title).font(.system(size: 24, weight: .semibold)).foregroundStyle(Ink.text).multilineTextAlignment(.center)
                }
                .frame(maxWidth: .infinity)
                .padding(.top, 24)
                Card { Text(message).font(.akBody).foregroundStyle(Ink.text) }
            } actions: {
                AkButton(title: "Back", kind: .secondary) { act(.done) }
            }
        }
    }
}

private struct OfflineScreen: View {
    let radios: [RadioRow]
    let walletOpen: Bool

    var body: some View {
        Page {
            Spacer().frame(height: 24)
            Image(systemName: "airplane").font(.system(size: 28, weight: .semibold)).foregroundStyle(Ink.warn)
                .frame(width: 64, height: 64).background(RoundedRectangle(cornerRadius: 20).fill(Ink.warn.opacity(0.14)))
            Text("Take this phone offline").font(.akTitle).foregroundStyle(Ink.text)
            Text("Airkeys works only on a phone with every radio off. Turn on Airplane mode, then make sure Wi-Fi and Bluetooth stay off in Settings.")
                .font(.akDim).foregroundStyle(Ink.dim)
            Card(padding: 6) {
                ForEach(radios, id: \.name) { r in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(r.name).font(.akBody).foregroundStyle(Ink.text)
                            Spacer()
                            switch r.state {
                            case .off: Pill(text: "Off", color: Ink.safe, symbol: "checkmark")
                            case .on: Pill(text: "On", color: Ink.warn, symbol: "exclamationmark.triangle")
                            case .active: Pill(text: "Still active", color: Ink.warn, symbol: "exclamationmark.triangle")
                            case .unknown: Pill(text: "Unknown", color: Ink.dim)
                            }
                        }
                        if let detail = r.detail { Text(detail).font(.akSmall).foregroundStyle(Ink.dim) }
                    }
                    .padding(12)
                }
            }
            if radios.contains(where: { $0.name == "Bluetooth" && $0.state == .unknown }) {
                Notice(text: "To check Bluetooth, Airkeys needs the Bluetooth permission (Settings › Airkeys). It never uses Bluetooth.", color: Ink.dim, symbol: "info.circle")
            }
            if walletOpen {
                Notice(text: "Your wallet is still open. Airkeys carries on as soon as the phone is offline again.", color: Ink.dim, symbol: "lock")
            }
        } actions: {
            EmptyView()
        }
    }
}

private struct Welcome: View {
    let version: String
    let act: (Action) -> Void

    var body: some View {
        Page {
            Spacer().frame(height: 24)
            Mark(size: 76)
            Text("Airkeys").font(.system(size: 38, weight: .semibold)).foregroundStyle(Ink.text)
            Text("A cold wallet for a phone that stays offline. It shows the world addresses — never a public key — so the funds behind them stay out of reach, even of a quantum computer.")
                .font(.system(size: 16)).foregroundStyle(Ink.dim).lineSpacing(3)
            Spacer().frame(height: 8)
            Feature(symbol: "lock", title: "Nothing is stored", text: "Bring your recovery phrase each time. It stays in memory until you close the wallet or the app.")
            Feature(symbol: "checkmark.shield", title: "Addresses only", text: "Connecting to the web app shares addresses, never the keys behind them.")
            Feature(symbol: "eye", title: "Clear signing", text: "Airkeys signs transfers only, after showing every recipient, amount and fee.")
            Text(version).font(.akSmall).foregroundStyle(Ink.faint)
        } actions: {
            AkButton(title: "Enter recovery phrase") { act(.begin) }
            AkButton(title: "Scan SeedQR", kind: .secondary, symbol: "viewfinder") { act(.scan) }
        }
    }
}

private struct Feature: View {
    let symbol: String
    let title: String
    let text: String

    var body: some View {
        HStack(alignment: .top, spacing: 14) {
            Image(systemName: symbol).font(.system(size: 17)).foregroundStyle(Ink.lamp)
                .frame(width: 38, height: 38).background(RoundedRectangle(cornerRadius: 11).fill(Ink.lamp.opacity(0.12)))
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.system(size: 16, weight: .semibold)).foregroundStyle(Ink.text)
                Text(text).font(.system(size: 14)).foregroundStyle(Ink.dim)
            }
        }
    }
}

private struct ImportScreen: View {
    let p: PhraseInput
    let act: (Action) -> Void
    let rewrite: (String) -> String
    /// A word tapped in the grid, selected in the field: typing replaces it.
    @State private var select: PhraseSelection?

    /// Where each word of the phrase is, in the field's text.
    private var spans: [NSRange] {
        let text = p.text as NSString
        let words = try! NSRegularExpression(pattern: "\\S+")
        return words.matches(in: p.text, range: NSRange(location: 0, length: text.length)).map(\.range)
    }

    var body: some View {
        Page(title: "Recovery phrase", onBack: { act(.back) }, trailing: {
            IconAction(symbol: "viewfinder", label: "Scan SeedQR", tint: Ink.dim) { act(.scan) }
        }, content: {
            Text("Enter your recovery phrase").font(.akTitle).foregroundStyle(Ink.text)
            Text("Type the first letters of each word and a space: Airkeys completes it.").font(.akDim).foregroundStyle(Ink.dim)
            PhraseEditor(text: p.text, placeholder: "abandon ability …", rewrite: rewrite, select: select)
                .overlay(alignment: .topLeading) {
                    if p.text.isEmpty { Text("abandon ability …").font(.system(size: 17, design: .monospaced)).foregroundStyle(Ink.faint).allowsHitTesting(false) }
                }
                .padding(16)
                .background(RoundedRectangle(cornerRadius: 14).fill(Ink.surface))
                .overlay(RoundedRectangle(cornerRadius: 14).stroke(p.problem != nil ? Ink.danger : Ink.stroke, lineWidth: 1))
            Suggestions(words: p.suggestions) { act(.suggestion(word: $0)) }
            if let problem = p.problem { Text(problem).font(.akSmall).foregroundStyle(Ink.danger) }
            if !p.words.isEmpty {
                HStack {
                    Label2(text: "\(p.words.count) words")
                    Spacer()
                    if p.valid { Pill(text: "Valid phrase", color: Ink.safe, symbol: "checkmark") }
                }
                // as on a recovery card: even columns, numbers aligned
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 3), alignment: .leading, spacing: 8) {
                    ForEach(Array(p.words.enumerated()), id: \.offset) { i, w in
                        HStack(spacing: 6) {
                            Text("\(i + 1)").font(.system(size: 12, design: .monospaced)).foregroundStyle(Ink.faint)
                                .frame(width: 18, alignment: .trailing)
                            Text(w.word).font(.system(size: 15, design: .monospaced)).lineLimit(1)
                                .foregroundStyle(w.state == .known ? Ink.text : (w.state == .typing ? Ink.dim : Ink.danger))
                            Spacer(minLength: 0)
                        }
                        .padding(.horizontal, 8).padding(.vertical, 8)
                        .background(RoundedRectangle(cornerRadius: 10).fill(w.state == .unknown ? Ink.danger.opacity(0.12) : (w.state == .typing ? Color.clear : Ink.raised)))
                        .overlay(RoundedRectangle(cornerRadius: 10).stroke(w.state == .typing ? Ink.stroke : Color.clear, lineWidth: 1))
                        .contentShape(Rectangle())
                        .onTapGesture { if i < spans.count { select = PhraseSelection(range: spans[i]) } }
                    }
                }
            }
        }, actions: {
            AkButton(title: "Continue", enabled: p.words.count >= 12 && p.problem == nil) { act(.phraseDone) }
        })
    }
}

private struct Home: View {
    let index: UInt32
    let accounts: [AccountRow]
    let act: (Action) -> Void

    var body: some View {
        Page(trailing: {
            Button { act(.close) } label: {
                Label("Close wallet", systemImage: "lock").font(.akSmall).foregroundStyle(Ink.dim).padding(.horizontal, 12).padding(.vertical, 8)
            }
        }, content: {
            AccountWheel(index: index) { act(.account(index: $0)) }
            ForEach(accounts, id: \.network) { a in
                Button { act(.openAccount(network: a.network)) } label: {
                    Card {
                        HStack(spacing: 14) {
                            NetworkBadge(kind: a.network)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(a.name).font(.system(size: 16, weight: .semibold)).foregroundStyle(Ink.text)
                                Text(a.assets).font(.akSmall).foregroundStyle(Ink.dim).lineLimit(1)
                            }
                            Spacer()
                        }
                        Text(short(a.address, keep: 8)).font(.system(size: 14, design: .monospaced)).foregroundStyle(Ink.dim)
                    }
                }
            }
        }, actions: {
            HStack(spacing: 10) {
                AkButton(title: "Connect", kind: .secondary, symbol: "qrcode") { act(.connect) }.frame(maxWidth: .infinity)
                AkButton(title: "Scan request", symbol: "viewfinder") { act(.scan) }.frame(maxWidth: .infinity)
            }
        })
    }
}

/// The system wheel — the Timer's: a haptic tick per number, the choice made when it rests.
/// Like the Timer's "15 min", the unit and the number meet at the center: "Account 15".
private struct AccountWheel: View {
    let index: UInt32
    let choose: (UInt32) -> Void

    var body: some View {
        GeometryReader { g in
            let half = g.size.width / 2
            ZStack(alignment: .leading) {
                Picker("Account", selection: Binding(get: { index }, set: choose)) {
                    ForEach(0..<accountCount(), id: \.self) { i in
                        Text("\(i)").font(.system(size: 24).monospacedDigit()).foregroundStyle(Ink.text)
                            .frame(maxWidth: .infinity, alignment: .leading).padding(.leading, half + 5).tag(i)
                    }
                }
                .pickerStyle(.wheel)
                .labelsHidden()
                .accessibilityIdentifier("account-wheel")
                Text("Account").font(.system(size: 17, weight: .medium)).foregroundStyle(Ink.text)
                    .frame(width: half - 5, alignment: .trailing).allowsHitTesting(false)
            }
        }
        .frame(height: 216) // the system wheel's own height: less crops its drum
    }
}

private struct AccountScreen: View {
    let a: AccountRow
    let act: (Action) -> Void

    var body: some View {
        Page(title: a.name, onBack: { act(.back) }) {
            HStack(spacing: 14) {
                NetworkBadge(kind: a.network, size: 52)
                VStack(alignment: .leading) {
                    Text("Account #\(a.index)").font(.akHeading).foregroundStyle(Ink.text)
                    Text(a.assets).font(.akSmall).foregroundStyle(Ink.dim)
                }
                Spacer()
            }
            Card {
                Label2(text: "Address")
                Address(address: a.address, size: 19)
            }
            Card {
                Label2(text: "Derivation path")
                Text(a.path).font(.akMono).foregroundStyle(Ink.text)
                Text("Every step is hardened.").font(.akSmall).foregroundStyle(Ink.dim)
            }
            Notice(text: "The public key behind this address stays on this phone until the account first sends. Receiving reveals nothing.", color: Ink.safe, symbol: "checkmark.shield")
        } actions: {
            AkButton(title: "Show to the web app", symbol: "qrcode") { act(.shareAccount(network: a.network)) }
        }
    }
}

/// The code fills the screen from the title to the button; its message is a tap away.
private struct CodeScreen: View {
    let title: String
    let caption: String
    let message: [JsonLine]
    let next: (UInt32, UInt32) -> CodeFrame?
    let act: (Action) -> Void
    @State private var reading = false

    var body: some View {
        Page(title: title, onBack: { act(.back) }, scroll: false) {
            Button(reading ? "Code" : "Message") { reading.toggle() }
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(Ink.lamp)
                .padding(.horizontal, 12)
        } content: {
            if reading {
                Text("What the code says").font(.akHeading).foregroundStyle(Ink.text)
                // a line at a time, drawn as it scrolls into view: all 100 accounts are about a
                // thousand lines, too many to lay out at once
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 3) {
                        ForEach(Array(message.enumerated()), id: \.offset) { _, line in
                            Text(highlighted(line.pieces))
                                .padding(.leading, CGFloat(line.depth) * 15) // two characters a level
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                    .font(.system(size: 12.5, design: .monospaced))
                    .textSelection(.enabled)
                    .padding(16)
                }
                .background(RoundedRectangle(cornerRadius: 16).fill(Ink.surface))
                .overlay(RoundedRectangle(cornerRadius: 16).stroke(Ink.stroke, lineWidth: 1))
            } else {
                CodeView(next: next)
                Text(caption).font(.akDim).foregroundStyle(Ink.dim).lineLimit(2)
            }
        } actions: {
            AkButton(title: "Done") { act(.done) }
        }
    }
}

/// A line of the message, coloured by kind.
private func highlighted(_ pieces: [JsonPiece]) -> AttributedString {
    pieces.reduce(into: AttributedString()) { text, p in
        var piece = AttributedString(p.text)
        piece.foregroundColor = switch p.kind {
        case .key: Ink.jsonKey
        case .text: Ink.jsonText
        case .number: Ink.jsonNumber
        case .literal: Ink.jsonLiteral
        case .punctuation: Ink.faint
        }
        text += piece
    }
}

private struct Viewfinder: View {
    var body: some View {
        Canvas { ctx, size in
            let inset: CGFloat = 28
            let r = CGRect(origin: .zero, size: size).insetBy(dx: inset, dy: inset)
            let l = min(r.width, r.height) * 0.12
            var p = Path()
            for (x, y, dx, dy) in [(r.minX, r.minY, 1.0, 1.0), (r.maxX, r.minY, -1.0, 1.0), (r.minX, r.maxY, 1.0, -1.0), (r.maxX, r.maxY, -1.0, -1.0)] {
                p.move(to: CGPoint(x: x + dx * l, y: y))
                p.addLine(to: CGPoint(x: x, y: y))
                p.addLine(to: CGPoint(x: x, y: y + dy * l))
            }
            ctx.stroke(p, with: .color(Ink.lamp), style: StrokeStyle(lineWidth: 3, lineCap: .round, lineJoin: .round))
        }
        .allowsHitTesting(false)
    }
}

private struct Toast: View {
    let text: String?
    let dismiss: () -> Void

    var body: some View {
        if let text {
            Text(text).font(.system(size: 15)).foregroundStyle(Ink.text)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(16)
                .background(RoundedRectangle(cornerRadius: 14).fill(Ink.raised))
                .overlay(RoundedRectangle(cornerRadius: 14).stroke(Ink.stroke, lineWidth: 1))
                .padding(.horizontal, 16)
                .padding(.bottom, 92)
                .onTapGesture(perform: dismiss)
                .task(id: text) {
                    try? await Task.sleep(for: .seconds(5))
                    dismiss()
                }
                .transition(.opacity)
        }
    }
}
