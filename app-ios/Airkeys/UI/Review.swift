import AirkeysCore
import SwiftUI

/// What the phone is asked to sign, in full, before anything is signed.
struct ReviewScreen: View {
    let r: ReviewView
    let act: (Action) -> Void
    @State private var details = false

    var body: some View {
        Page(title: "Review", onBack: { act(.reject) }) {
            HStack(spacing: 14) {
                NetworkBadge(kind: r.network, size: 44)
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 10) {
                        Text(r.chain).font(.akHeading).foregroundStyle(Ink.text)
                        // test coins have no value: never mistake one network for the other
                        if r.testnet { Pill(text: "Testnet", color: Ink.warn) }
                    }
                    Text("From account #\(r.fromIndex) · \(short(r.fromAddress))").font(.akSmall).foregroundStyle(Ink.dim)
                }
            }
            ForEach(r.warnings, id: \.self) { Notice(text: $0, color: Ink.warn) }
            ForEach(Array(r.transfers.enumerated()), id: \.offset) { _, t in
                Card(border: t.verified ? Ink.stroke : Ink.warn.opacity(0.5)) {
                    if let label = t.label { Text(label).font(.akSmall).foregroundStyle(Ink.faint) }
                    Text("\(t.amount) \(t.asset)").font(.akAmount).foregroundStyle(Ink.text).minimumScaleFactor(0.5).lineLimit(2)
                    if let token = t.token {
                        if t.verified {
                            HStack(spacing: 6) {
                                Image(systemName: "checkmark").font(.system(size: 12, weight: .semibold)).foregroundStyle(Ink.safe)
                                Text("Listed token · \(short(token))").font(.akSmall).foregroundStyle(Ink.dim)
                            }
                        } else {
                            HStack(spacing: 10) {
                                Pill(text: "Unlisted token", color: Ink.warn, symbol: "exclamationmark.triangle")
                                Text(short(token)).font(.system(size: 13, design: .monospaced)).foregroundStyle(Ink.dim)
                            }
                        }
                    }
                    // the phone derived its own account's address: named, the address only to glance at
                    if let own = t.ownIndex {
                        VStack(alignment: .leading, spacing: 2) {
                            (Text("to ").foregroundStyle(Ink.dim) + Text("Your account #\(own)").fontWeight(.semibold).foregroundStyle(Ink.text))
                                .font(.system(size: 16))
                            Text(short(t.to, keep: 4)).font(.system(size: 13, design: .monospaced)).foregroundStyle(Ink.dim)
                        }
                        .padding(.top, 2)
                    } else {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("to").font(.akSmall).foregroundStyle(Ink.dim)
                            Address(address: t.to, size: 16)
                        }
                        .padding(.top, 2)
                    }
                    if let note = t.note { Text(note).font(.akSmall).foregroundStyle(Ink.dim) }
                }
            }
            HStack {
                Text("Network fee, at most").font(.akDim).foregroundStyle(Ink.dim)
                Spacer()
                Text(r.fee).font(.system(size: 16, weight: .semibold)).foregroundStyle(Ink.text)
            }
            .padding(.horizontal, 4)
            ForEach(r.notices, id: \.self) { Notice(text: $0, color: Ink.dim, symbol: "shield") }
            Button { details.toggle() } label: {
                Card {
                    HStack {
                        Text("Details").font(.system(size: 16, weight: .semibold)).foregroundStyle(Ink.text)
                        Spacer()
                        Text(details ? "Hide" : "Show").font(.akSmall).foregroundStyle(Ink.lamp)
                    }
                    if details {
                        ForEach(r.details, id: \.label) { d in
                            HStack(alignment: .top) {
                                Text(d.label).font(.akSmall).foregroundStyle(Ink.dim)
                                Spacer()
                                Text(d.value).font(.akSmall).foregroundStyle(Ink.text).multilineTextAlignment(.trailing)
                            }
                        }
                        Text("From").font(.akSmall).foregroundStyle(Ink.dim)
                        Address(address: r.fromAddress, size: 13)
                    }
                }
            }
        } actions: {
            HoldButton(title: r.holdLabel, warn: !r.warnings.isEmpty) { act(.sign) }
            AkButton(title: "Reject", kind: .secondary) { act(.reject) }
        }
    }
}

/// The animated code, as large as the phone allows: the whole width of the screen, edge to
/// edge, and the height it is given. The core shapes the code to this area (a tall code on a
/// phone). Cells are a whole number of device pixels, never smoothed. Big cells are what a
/// camera reads fastest, and from farther away; filling the screen carries more in each frame.
struct CodeView: View {
    let next: (UInt32, UInt32) -> CodeFrame?
    @State private var image: CGImage?
    @State private var area: CGSize = .zero
    @Environment(\.displayScale) private var displayScale

    var body: some View {
        GeometryReader { g in
            ZStack {
                Color(red: 10 / 255, green: 13 / 255, blue: 20 / 255) // the code's own quiet zone (Lumen's ink)
                if let image {
                    // whole device pixels per cell
                    let fit = min(g.size.width / CGFloat(image.width), g.size.height / CGFloat(image.height)) * displayScale
                    let scale = max(floor(fit), 1) / displayScale
                    Image(decorative: image, scale: 1)
                        .interpolation(.none)
                        .resizable()
                        .frame(width: CGFloat(image.width) * scale, height: CGFloat(image.height) * scale)
                }
            }
            .onAppear { area = g.size }
            .onChange(of: g.size) { _, size in area = size }
        }
        .padding(.horizontal, -20) // out of the page's side margins
        .task {
            let interval = 1.0 / Double(codeFps())
            while !Task.isCancelled {
                let (w, h) = (UInt32(area.width * displayScale), UInt32(area.height * displayScale))
                if w > 0, h > 0, let f = next(w, h) { image = f.cgImage }
                try? await Task.sleep(for: .seconds(interval))
            }
        }
    }
}

private extension CodeFrame {
    var cgImage: CGImage? {
        guard let provider = CGDataProvider(data: rgba as CFData) else { return nil }
        return CGImage(width: Int(width), height: Int(height), bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: Int(width) * 4,
                       space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue),
                       provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
    }
}
