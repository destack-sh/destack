import RuntimeHostAppleCore

/// Ignore location samples in macOS tests.
final class MacOSNoopLocationEventSink: LocationEvents {
    func sendLocationSample(
        watchID: String,
        sample: RuntimeHostLocationSample
    ) {
        let _ = watchID
        let _ = sample
    }
}
