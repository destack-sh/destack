// swift-tools-version: 6.0
import PackageDescription

let rustLinkerSettings: [LinkerSetting] = [
  .unsafeFlags([
    "-L",
    ".build/rust",
    "-ldestack_runtime_abi",
  ])
]

let appleFrameworkLinkerSettings: [LinkerSetting] = [
  .linkedFramework("AVFoundation"),
  .linkedFramework("AudioToolbox"),
  .linkedFramework("Contacts"),
  .linkedFramework("CoreAudio"),
  .linkedFramework("CoreBluetooth"),
  .linkedFramework("CoreFoundation"),
  .linkedFramework("CoreGraphics"),
  .linkedFramework("CoreLocation"),
  .linkedFramework("CoreMedia"),
  .linkedFramework("CoreMIDI"),
  .linkedFramework("CoreVideo"),
  .linkedFramework("EventKit"),
  .linkedFramework("MediaToolbox"),
  .linkedFramework("UniformTypeIdentifiers"),
  .linkedFramework("UserNotifications"),
]

let runtimeAppleLinkerSettings = rustLinkerSettings + appleFrameworkLinkerSettings

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
      publicHeadersPath: "include",
      linkerSettings: rustLinkerSettings
    ),
    .target(
      name: "RuntimeHostAppleCore",
      path: "core/Sources/RuntimeHostAppleCore",
      linkerSettings: appleFrameworkLinkerSettings
    ),
    .target(
      name: "RuntimeHostIOS",
      dependencies: ["RuntimeHostAppleBridgeC", "RuntimeHostAppleCore"],
      path: "ios/Sources/RuntimeHostIOS",
      linkerSettings: runtimeAppleLinkerSettings
    ),
    .target(
      name: "RuntimeHostMacOS",
      dependencies: ["RuntimeHostAppleCore"],
      path: "macos/Sources/RuntimeHostMacOS",
      linkerSettings: appleFrameworkLinkerSettings
    ),
    .testTarget(
      name: "RuntimeHostAppleCoreTests",
      dependencies: ["RuntimeHostAppleCore"],
      path: "core/Tests/RuntimeHostAppleCoreTests",
      linkerSettings: appleFrameworkLinkerSettings
    ),
    .testTarget(
      name: "RuntimeHostIOSTests",
      dependencies: [
        "RuntimeHostAppleBridgeC", "RuntimeHostAppleCore",
        "RuntimeHostIOS",
      ],
      path: "ios/Tests/RuntimeHostIOSTests",
      linkerSettings: runtimeAppleLinkerSettings
    ),
    .testTarget(
      name: "RuntimeHostMacOSTests",
      dependencies: ["RuntimeHostAppleCore", "RuntimeHostMacOS"],
      path: "macos/Tests/RuntimeHostMacOSTests",
      linkerSettings: appleFrameworkLinkerSettings
    ),
  ],
  swiftLanguageModes: [.v6]
)
