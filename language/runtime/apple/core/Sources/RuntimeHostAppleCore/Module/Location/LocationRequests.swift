import Foundation

/// The location request surface attached to one Apple runtime host.
@MainActor
public protocol LocationRequests {
    /// Read whether Apple location services are enabled.
    func locationServicesEnabled() -> RuntimeHostLocationServicesResponse

    /// Read one last-known Apple location sample.
    func locationLastKnown() -> RuntimeHostLocationLastKnownResponse

    /// Open one Apple location watch.
    func locationWatchOpen(
        watchID: String,
        options: RuntimeHostLocationWatchOptions
    ) -> UInt32

    /// Close one Apple location watch.
    func locationWatchClose(
        watchID: String
    ) -> UInt32
}

/// The explicit unsupported location request surface for one Apple runtime host.
@MainActor
public final class UnsupportedLocationRequests: LocationRequests {
    /// Create one unsupported location request surface.
    public init() {}

    public func locationServicesEnabled() -> RuntimeHostLocationServicesResponse {
        RuntimeHostLocationServicesResponse(status: 1)
    }

    public func locationLastKnown() -> RuntimeHostLocationLastKnownResponse {
        RuntimeHostLocationLastKnownResponse(status: 1)
    }

    public func locationWatchOpen(
        watchID: String,
        options: RuntimeHostLocationWatchOptions
    ) -> UInt32 {
        let _ = watchID
        let _ = options

        return 1
    }

    public func locationWatchClose(
        watchID: String
    ) -> UInt32 {
        let _ = watchID

        return 1
    }
}
