// Draws the Airkeys app icon (1024 px): the keyhole-in-a-ring mark on the bunker background.
// swift scripts/make-icon.swift Airkeys/Resources/Assets.xcassets/AppIcon.appiconset/icon-1024.png
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let size = 1024
let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
                    space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
ctx.setFillColor(CGColor(srgbRed: 0.043, green: 0.051, blue: 0.067, alpha: 1))
ctx.fill(CGRect(x: 0, y: 0, width: size, height: size))
// the mark on the Android adaptive-icon grid (108 units), scaled so the ring fills ~62 %
let s = CGFloat(size) / 108 * 1.25
let o = CGFloat(size) / 2 - 54 * s
func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: o + x * s, y: CGFloat(size) - (o + y * s)) }
let lamp = CGColor(srgbRed: 0.949, green: 0.710, blue: 0.267, alpha: 1)
ctx.setStrokeColor(lamp)
ctx.setLineWidth(4.5 * s)
ctx.strokeEllipse(in: CGRect(x: o + 27 * s, y: CGFloat(size) - (o + 81 * s), width: 54 * s, height: 54 * s))
ctx.setFillColor(lamp)
ctx.fillEllipse(in: CGRect(x: o + 45.5 * s, y: CGFloat(size) - (o + 57.5 * s), width: 17 * s, height: 17 * s))
ctx.beginPath()
ctx.move(to: p(51.2, 54))
ctx.addLine(to: p(48.4, 69.5))
ctx.addLine(to: p(59.6, 69.5))
ctx.addLine(to: p(56.8, 54))
ctx.closePath()
ctx.fillPath()
let url = URL(fileURLWithPath: CommandLine.arguments[1])
let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(dest, ctx.makeImage()!, nil)
CGImageDestinationFinalize(dest)
