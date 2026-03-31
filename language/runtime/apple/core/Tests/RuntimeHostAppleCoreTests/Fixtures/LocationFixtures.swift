import RuntimeHostAppleCore

/// Record location requests for Apple core tests.
@MainActor
final class RecordingLocationRequestHandler: LocationRequests {
  var servicesEnabledResponse = RuntimeHostLocationServicesResponse(status: 0)
  var lastKnownResponse = RuntimeHostLocationLastKnownResponse(status: 0)
  var watchOpenCalls: [(String, RuntimeHostLocationWatchOptions)] = []
  var watchCloseCalls: [String] = []
  var watchOpenStatus: UInt32 = 0
  var watchCloseStatus: UInt32 = 0

  func servicesEnabled() -> RuntimeHostLocationServicesResponse {
    servicesEnabledResponse
  }

  func lastKnown() -> RuntimeHostLocationLastKnownResponse {
    lastKnownResponse
  }

  func watchOpen(
    _ watchID: String,
    options: RuntimeHostLocationWatchOptions
  ) -> UInt32 {
    watchOpenCalls.append((watchID, options))

    return watchOpenStatus
  }

  func watchClose(
    _ watchID: String
  ) -> UInt32 {
    watchCloseCalls.append(watchID)

    return watchCloseStatus
  }
}

/// Record location samples for Apple core tests.
final class RecordingLocationEventSink: LocationEvents {
  var samples: [(String, RuntimeHostLocationSample)] = []

  func notifyLocationSample(
    _ watchID: String,
    sample: RuntimeHostLocationSample
  ) {
    samples.append((watchID, sample))
  }
}
