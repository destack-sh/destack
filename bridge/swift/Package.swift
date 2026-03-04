// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Destack",
    platforms: [
        .macOS(.v13),
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "Destack",
            targets: ["Destack"]
        )
    ],
    targets: [
        .target(
            name: "Destack"
        )
    ],
    swiftLanguageModes: [.v6]
)
