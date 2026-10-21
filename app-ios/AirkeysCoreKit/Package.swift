// swift-tools-version:5.9
// The Rust core (core/app) as a Swift package: the static library in an xcframework plus the
// uniffi-generated Swift. Both are built by core/scripts/build-ios.sh.
import PackageDescription

let package = Package(
    name: "AirkeysCoreKit",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "AirkeysCore", targets: ["AirkeysCore"])
    ],
    targets: [
        .binaryTarget(name: "AirkeysFFI", path: "Artifacts/AirkeysFFI.xcframework"),
        .target(name: "AirkeysCore", dependencies: ["AirkeysFFI"], path: "Sources/AirkeysCore"),
    ]
)
