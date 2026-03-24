// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "RuntimeHostApple",
    platforms: [
        .macOS(.v13),
        .iOS(.v13),
    ],
    products: [
        .library(
            name: "RuntimeHostAppleCore",
            targets: ["RuntimeHostAppleCore"]
        ),
        .library(
            name: "RuntimeHostIOS",
            targets: ["RuntimeHostIOS"]
        ),
        .library(
            name: "RuntimeHostMacOS",
            targets: ["RuntimeHostMacOS"]
        ),
    ],
    targets: [
        .target(
            name: "RuntimeHostAppleBridgeC",
            path: "bridge/BridgeC",
            publicHeadersPath: "include"
        ),
        .target(
            name: "RuntimeHostAppleBridgeTest",
            dependencies: ["RuntimeHostAppleBridgeC"],
            path: "bridge/BridgeCTest",
            publicHeadersPath: "include"
        ),
        .target(
            name: "RuntimeHostAppleCore",
            path: "core/Sources/RuntimeHostAppleCore"
        ),
        .target(
            name: "RuntimeHostIOS",
            dependencies: ["RuntimeHostAppleBridgeC", "RuntimeHostAppleCore"],
            path: "ios/Sources/RuntimeHostIOS"
        ),
        .target(
            name: "RuntimeHostMacOS",
            dependencies: ["RuntimeHostAppleCore"],
            path: "macos/Sources/RuntimeHostMacOS"
        ),
        .testTarget(
            name: "RuntimeHostAppleCoreTests",
            dependencies: ["RuntimeHostAppleCore"],
            path: "core/Tests/RuntimeHostAppleCoreTests"
        ),
        .testTarget(
            name: "RuntimeHostIOSTests",
            dependencies: ["RuntimeHostAppleBridgeC", "RuntimeHostAppleBridgeTest", "RuntimeHostAppleCore", "RuntimeHostIOS"],
            path: "ios/Tests/RuntimeHostIOSTests"
        ),
        .testTarget(
            name: "RuntimeHostMacOSTests",
            dependencies: ["RuntimeHostAppleCore", "RuntimeHostMacOS"],
            path: "macos/Tests/RuntimeHostMacOSTests"
        ),
    ],
    swiftLanguageModes: [.v6]
)
