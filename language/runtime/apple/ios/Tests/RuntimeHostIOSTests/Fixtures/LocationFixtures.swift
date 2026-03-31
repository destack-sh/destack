import RuntimeHostAppleCore

/// Record location requests for iOS tests.
@MainActor
final class IOSRecordingLocationRequestHandler: LocationRequests {
  var servicesEnabledResponse = RuntimeHostLocationServicesResponse(status: hostStatusOk)
  var lastKnownResponse = RuntimeHostLocationLastKnownResponse(status: hostStatusOk)
  var watchOpenCalls: [(String, RuntimeHostLocationWatchOptions)] = []
  var watchCloseCalls: [String] = []
  var watchOpenStatus: UInt32 = hostStatusOk
  var watchCloseStatus: UInt32 = hostStatusOk

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

/// Ignore location samples in iOS tests.
final class IOSNoopLocationEventSink: LocationEvents {
  var samples: [(String, RuntimeHostLocationSample)] = []

  func notifyLocationSample(
    _ watchID: String,
    sample: RuntimeHostLocationSample
  ) {
    samples.append((watchID, sample))
  }
}

/// Build one representative iOS location sample.
func makeIOSLocationSample() -> RuntimeHostLocationSample {
  RuntimeHostLocationSample(
    latitudeDegrees: 47.3769,
    longitudeDegrees: 8.5417,
    altitudeMeters: 408.0,
    horizontalAccuracyMeters: 5.0,
    verticalAccuracyMeters: 8.0,
    speedMetersPerSecond: 1.25,
    headingDegrees: 180.0,
    timestampUnixNs: 1_700_000_000_000_000_000
  )
}
