import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one immediate location-services request.
typealias LocationServicesEnabledCallback =
  @convention(c) (UInt64, UnsafeMutablePointer<Bool>?) -> UInt32

/// One C callback for one immediate last-known-location request.
typealias LocationLastKnownCallback =
  @convention(c) (UInt64, UnsafeMutablePointer<DestackRustLocationSample>?) -> UInt32

/// One C callback for one location-watch open request.
typealias LocationWatchOpenCallback =
  @convention(c) (UInt64, DestackRustStringRef, DestackRustLocationWatchOptions) -> UInt32

/// One C callback for one location-watch close request.
typealias LocationWatchCloseCallback =
  @convention(c) (UInt64, DestackRustStringRef) -> UInt32

/// The low-level location ingress ABI for one iOS runtime bridge.
protocol LocationAbi: Sendable {
  /// Deliver one location sample into one runtime session.
  func notifyLocationSample(
    sessionHandle: HostSessionHandle,
    watchID: String,
    sample: RuntimeHostLocationSample
  ) -> RuntimeAbiStatus
}
