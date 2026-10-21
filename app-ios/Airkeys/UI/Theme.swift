import AirkeysCore
import SwiftUI

/// The Airkeys palette: a dark bunker, one warm lamp. Same values on Android and the web.
enum Ink {
    static let bg = Color(hex: 0x0B0D11)
    static let surface = Color(hex: 0x13161C)
    static let raised = Color(hex: 0x1A1E26)
    static let stroke = Color(hex: 0x262B35)
    static let text = Color(hex: 0xECEEF2)
    static let dim = Color(hex: 0x9BA3B4)
    static let faint = Color(hex: 0x687082)
    static let lamp = Color(hex: 0xF2B544)
    static let onLamp = Color(hex: 0x1C1505)
    static let safe = Color(hex: 0x3FD08F)
    static let warn = Color(hex: 0xF5A524)
    static let danger = Color(hex: 0xF2555A)

    static func network(_ kind: NetworkKind) -> Color {
        switch kind {
        case .bitcoin: Color(hex: 0xF7931A)
        case .evm: Color(hex: 0x627EEA)
        }
    }
}

extension Color {
    init(hex: UInt32) {
        self.init(.sRGB, red: Double(hex >> 16 & 255) / 255, green: Double(hex >> 8 & 255) / 255, blue: Double(hex & 255) / 255)
    }
}

extension Font {
    static let akTitle = Font.system(size: 28, weight: .semibold)
    static let akHeading = Font.system(size: 20, weight: .semibold)
    static let akBody = Font.system(size: 16)
    static let akDim = Font.system(size: 15)
    static let akSmall = Font.system(size: 13)
    static let akLabel = Font.system(size: 12, weight: .semibold)
    static let akMono = Font.system(size: 15, design: .monospaced)
    static let akAmount = Font.system(size: 30, weight: .semibold)
}
