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

    func locationServicesEnabled() -> RuntimeHostLocationServicesResponse {
        servicesEnabledResponse
    }

    func locationLastKnown() -> RuntimeHostLocationLastKnownResponse {
        lastKnownResponse
    }

    func locationWatchOpen(
        watchID: String,
        options: RuntimeHostLocationWatchOptions
    ) -> UInt32 {
        watchOpenCalls.append((watchID, options))

        return watchOpenStatus
    }

    func locationWatchClose(
        watchID: String
    ) -> UInt32 {
        watchCloseCalls.append(watchID)

        return watchCloseStatus
    }
}

/// Record location samples for Apple core tests.
final class RecordingLocationEventSink: LocationEvents {
    var samples: [(String, RuntimeHostLocationSample)] = []

    func sendLocationSample(
        watchID: String,
        sample: RuntimeHostLocationSample
    ) {
        samples.append((watchID, sample))
    }
}
