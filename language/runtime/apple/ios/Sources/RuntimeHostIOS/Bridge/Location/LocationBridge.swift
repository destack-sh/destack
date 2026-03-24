import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

func locationServicesEnabledCallback(
  sessionHandle: UInt64,
  isEnabled: UnsafeMutablePointer<Bool>?
) -> UInt32 {
  handleLocationServicesEnabled(
    sessionHandle: sessionHandle,
    isEnabled: isEnabled
  )
}

func locationLastKnownCallback(
  sessionHandle: UInt64,
  sample: UnsafeMutablePointer<DestackRustLocationSample>?
) -> UInt32 {
  handleLocationLastKnown(
    sessionHandle: sessionHandle,
    sample: sample
  )
}

func locationWatchOpenCallback(
  sessionHandle: UInt64,
  watchID: DestackRustStringRef,
  options: DestackRustLocationWatchOptions
) -> UInt32 {
  handleLocationWatchOpen(
    sessionHandle: sessionHandle,
    watchID: watchID,
    options: options
  )
}

func locationWatchCloseCallback(
  sessionHandle: UInt64,
  watchID: DestackRustStringRef
) -> UInt32 {
  handleLocationWatchClose(
    sessionHandle: sessionHandle,
    watchID: watchID
  )
}

/// One location bridge lane for one attached iOS runtime host.
@MainActor
final class LocationBridge: LocationEvents {
  /// The runtime session routed through this location bridge lane.
  nonisolated let sessionHandle: HostSessionHandle
  nonisolated private let bindings: any LocationAbi

  /// Create one location bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any LocationAbi
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
  }

  /// Send one location sample into the runtime ingress path.
  nonisolated func sendLocationSample(
    watchID: String,
    sample: RuntimeHostLocationSample
  ) {
    let status = bindings.notifyLocationSample(
      sessionHandle: sessionHandle,
      watchID: watchID,
      sample: sample
    )

    precondition(
      status.code == hostStatusOk,
      "runtime bridge could not deliver location sample: code \(status.code), error \(status.errorID)"
    )
  }

  /// Read whether iOS location services are enabled through the attached host.
  func locationServicesEnabled(
    runtimeHost: RuntimeHost
  ) -> RuntimeHostLocationServicesResponse {
    runtimeHost.locationRequests.locationServicesEnabled()
  }

  /// Read one last-known iOS location sample through the attached host.
  func locationLastKnown(
    runtimeHost: RuntimeHost
  ) -> RuntimeHostLocationLastKnownResponse {
    runtimeHost.locationRequests.locationLastKnown()
  }

  /// Open one iOS location watch through the attached host.
  func locationWatchOpen(
    runtimeHost: RuntimeHost,
    watchID: String,
    options: RuntimeHostLocationWatchOptions
  ) -> UInt32 {
    runtimeHost.locationRequests.locationWatchOpen(
      watchID: watchID,
      options: options
    )
  }

  /// Close one iOS location watch through the attached host.
  func locationWatchClose(
    runtimeHost: RuntimeHost,
    watchID: String
  ) -> UInt32 {
    runtimeHost.locationRequests.locationWatchClose(watchID: watchID)
  }
}

/// Resolve one registered location bridge for one runtime session.
func guardLocationBridge(
  sessionHandle: UInt64
) -> LocationBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.locationBridge
}

/// Handle one runtime callback asking whether iOS location services are enabled.
private func handleLocationServicesEnabled(
  sessionHandle: UInt64,
  isEnabled: UnsafeMutablePointer<Bool>?
) -> UInt32 {
  guard let bridge = guardLocationBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let isEnabled else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.locationServicesEnabled(runtimeHost: runtimeHost)
  }
  isEnabled.pointee = response.isEnabled

  return response.status
}

/// Handle one runtime callback asking for one last-known iOS location sample.
private func handleLocationLastKnown(
  sessionHandle: UInt64,
  sample: UnsafeMutablePointer<DestackRustLocationSample>?
) -> UInt32 {
  guard let bridge = guardLocationBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let sample else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.locationLastKnown(runtimeHost: runtimeHost)
  }

  guard let locationSample = response.sample else {
    return response.status
  }

  sample.pointee = encodeLocationSample(locationSample)

  return response.status
}

/// Handle one runtime callback asking to open one iOS location watch.
private func handleLocationWatchOpen(
  sessionHandle: UInt64,
  watchID bridgeWatchID: DestackRustStringRef,
  options: DestackRustLocationWatchOptions
) -> UInt32 {
  guard let bridge = guardLocationBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let watchID: String
  do {
    watchID = try tryDecodeNativeString(bridgeWatchID)
  } catch {
    return hostStatusInvalidArgument
  }

  guard let options = decodeLocationWatchOptions(options) else {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.locationWatchOpen(
      runtimeHost: runtimeHost,
      watchID: watchID,
      options: options
    )
  }
}

/// Handle one runtime callback asking to close one iOS location watch.
private func handleLocationWatchClose(
  sessionHandle: UInt64,
  watchID bridgeWatchID: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardLocationBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let watchID: String
  do {
    watchID = try tryDecodeNativeString(bridgeWatchID)
  } catch {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.locationWatchClose(
      runtimeHost: runtimeHost,
      watchID: watchID
    )
  }
}

/// Decode one native watch-options payload into one Swift value.
private func decodeLocationWatchOptions(
  _ value: DestackRustLocationWatchOptions
) -> RuntimeHostLocationWatchOptions? {
  guard let accuracy = RuntimeHostLocationAccuracy(rawValue: Int32(value.accuracy.rawValue)) else {
    return nil
  }

  return RuntimeHostLocationWatchOptions(
    accuracy: accuracy,
    minimumIntervalNs: value.minimum_interval_ns,
    minimumDistanceMeters: value.minimum_distance_meters,
    includeHeading: value.include_heading
  )
}

/// Encode one Swift location sample into one bridge payload.
private func encodeLocationSample(
  _ sample: RuntimeHostLocationSample
) -> DestackRustLocationSample {
  DestackRustLocationSample(
    latitude_degrees: sample.latitudeDegrees,
    longitude_degrees: sample.longitudeDegrees,
    altitude_meters: sample.altitudeMeters,
    horizontal_accuracy_meters: sample.horizontalAccuracyMeters,
    vertical_accuracy_meters: sample.verticalAccuracyMeters,
    speed_meters_per_second: sample.speedMetersPerSecond,
    heading_degrees: sample.headingDegrees,
    timestamp_unix_ns: sample.timestampUnixNs
  )
}
