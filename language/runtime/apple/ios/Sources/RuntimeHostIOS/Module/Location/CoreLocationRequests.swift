import CoreLocation
import Foundation
import RuntimeHostAppleCore

private let nanosecondsPerSecond: UInt64 = 1_000_000_000

/// One live iOS location watch registration.
private final class LocationWatchRegistration {
    /// The stable watch identifier.
    let watchID: String
    /// The requested watch options.
    let options: RuntimeHostLocationWatchOptions
    /// The event surface that receives normalized samples.
    let events: any LocationEvents
    /// The active location manager for this watch.
    let manager: CLLocationManager
    /// The delegate adapter retained by the manager.
    let delegate: LocationWatchDelegate

    /// Create one iOS location watch registration.
    init(
        watchID: String,
        options: RuntimeHostLocationWatchOptions,
        events: any LocationEvents,
        manager: CLLocationManager,
        delegate: LocationWatchDelegate
    ) {
        self.watchID = watchID
        self.options = options
        self.events = events
        self.manager = manager
        self.delegate = delegate
    }
}

/// One delegate adapter for one live iOS location watch.
private final class LocationWatchDelegate: NSObject, CLLocationManagerDelegate {
    /// The watch registration that owns this delegate.
    weak var registration: LocationWatchRegistration?
    /// The last sample timestamp delivered for this watch.
    private var lastTimestampUnixNs: UInt64?

    /// Create one iOS location watch delegate.
    init(
        registration: LocationWatchRegistration? = nil
    ) {
        self.registration = registration
    }

    nonisolated func locationManager(
        _ manager: CLLocationManager,
        didUpdateLocations locations: [CLLocation]
    ) {
        guard
            let registration = registration,
            let location = locations.last
        else {
            return
        }

        // honor the requested minimum interval before delivering one new sample
        let sample = runtimeHostLocationSample(
            location,
            includeHeading: registration.options.includeHeading
        )
        if let lastTimestampUnixNs {
            let elapsedUnixNs = sample.timestampUnixNs - lastTimestampUnixNs
            if elapsedUnixNs < registration.options.minimumIntervalNs {
                return
            }
        }

        lastTimestampUnixNs = sample.timestampUnixNs

        let watchID = registration.watchID
        let events = registration.events

        events.sendLocationSample(
            watchID: watchID,
            sample: sample
        )
    }
}

/// The iOS location request surface backed by Core Location.
@MainActor
public final class CoreLocationRequests: LocationRequests {
    private let events: any LocationEvents
    private let watchRegistrations = NSMutableDictionary()

    /// Create one live iOS location request surface.
    public init(
        events: any LocationEvents
    ) {
        self.events = events
    }

    public func locationServicesEnabled() -> RuntimeHostLocationServicesResponse {
        readLocationServicesEnabled()
    }

    public func locationLastKnown() -> RuntimeHostLocationLastKnownResponse {
        readLocationLastKnown()
    }

    public func locationWatchOpen(
        watchID: String,
        options: RuntimeHostLocationWatchOptions
    ) -> UInt32 {
        openLocationWatch(
            watchID: watchID,
            options: options,
            events: events,
            watchRegistrations: watchRegistrations
        )
    }

    public func locationWatchClose(
        watchID: String
    ) -> UInt32 {
        closeLocationWatch(
            watchID: watchID,
            watchRegistrations: watchRegistrations
        )
    }
}

/// Read whether iOS location services are enabled.
@MainActor
private func readLocationServicesEnabled() -> RuntimeHostLocationServicesResponse {
    RuntimeHostLocationServicesResponse(
        status: hostStatusOk,
        isEnabled: CLLocationManager.locationServicesEnabled()
    )
}

/// Read one last-known iOS location sample.
@MainActor
private func readLocationLastKnown() -> RuntimeHostLocationLastKnownResponse {
    // require active platform location services first
    guard CLLocationManager.locationServicesEnabled() else {
        return RuntimeHostLocationLastKnownResponse(status: hostStatusNotSupported)
    }

    // require one granted location authorization before reading cached state
    let manager = CLLocationManager()
    let authorizationStatus = manager.authorizationStatus
    if !isLocationAuthorized(authorizationStatus) {
        return RuntimeHostLocationLastKnownResponse(status: hostStatusPermissionDenied)
    }

    // return the freshest cached sample when the platform has one
    guard let location = manager.location else {
        return RuntimeHostLocationLastKnownResponse(status: hostStatusNotFound)
    }

    return RuntimeHostLocationLastKnownResponse(
        status: hostStatusOk,
        sample: runtimeHostLocationSample(location, includeHeading: true)
    )
}

/// Open one live iOS location watch.
@MainActor
private func openLocationWatch(
    watchID: String,
    options: RuntimeHostLocationWatchOptions,
    events: any LocationEvents,
    watchRegistrations: NSMutableDictionary
) -> UInt32 {
    // reject empty and duplicate watch identifiers loudly
    if watchID.isEmpty {
        return hostStatusInvalidArgument
    }

    if watchRegistrations.object(forKey: watchID) != nil {
        return hostStatusInvalidArgument
    }

    // require active platform location services before opening one watch
    guard CLLocationManager.locationServicesEnabled() else {
        return hostStatusNotSupported
    }

    // require one granted authorization before starting updates
    let manager = CLLocationManager()
    let authorizationStatus = manager.authorizationStatus
    if !isLocationAuthorized(authorizationStatus) {
        return hostStatusPermissionDenied
    }

    // install one live manager and delegate pair for this watch
    let delegate = LocationWatchDelegate()
    let registration = LocationWatchRegistration(
        watchID: watchID,
        options: options,
        events: events,
        manager: manager,
        delegate: delegate
    )
    delegate.registration = registration
    manager.delegate = delegate
    manager.desiredAccuracy = desiredAccuracy(options.accuracy)
    manager.distanceFilter = max(0, options.minimumDistanceMeters)

    #if os(iOS)
    if options.includeHeading && CLLocationManager.headingAvailable() {
        manager.startUpdatingHeading()
    }
    #endif

    manager.startUpdatingLocation()
    watchRegistrations.setObject(registration, forKey: watchID as NSString)

    return hostStatusOk
}

/// Close one live iOS location watch.
@MainActor
private func closeLocationWatch(
    watchID: String,
    watchRegistrations: NSMutableDictionary
) -> UInt32 {
    // require one live watch registration before tearing it down
    guard
        let registration = watchRegistrations.object(forKey: watchID) as? LocationWatchRegistration
    else {
        return hostStatusNotFound
    }

    registration.manager.stopUpdatingLocation()
    #if os(iOS)
    registration.manager.stopUpdatingHeading()
    #endif
    registration.manager.delegate = nil
    watchRegistrations.removeObject(forKey: watchID)

    return hostStatusOk
}

/// Return whether one iOS authorization status allows location access.
@MainActor
private func isLocationAuthorized(
    _ status: CLAuthorizationStatus
) -> Bool {
    switch status {
    case .authorizedAlways, .authorizedWhenInUse:
        true

    case .notDetermined, .restricted, .denied:
        false

    @unknown default:
        false
    }
}

/// Map one runtime location accuracy into one Core Location accuracy.
@MainActor
private func desiredAccuracy(
    _ accuracy: RuntimeHostLocationAccuracy
) -> CLLocationAccuracy {
    switch accuracy {
    case .passive:
        kCLLocationAccuracyThreeKilometers

    case .low:
        kCLLocationAccuracyKilometer

    case .balanced:
        kCLLocationAccuracyHundredMeters

    case .high:
        kCLLocationAccuracyNearestTenMeters

    case .best:
        kCLLocationAccuracyBest
    }
}

/// Normalize one Core Location sample into one runtime location sample.
private func runtimeHostLocationSample(
    _ location: CLLocation,
    includeHeading: Bool
) -> RuntimeHostLocationSample {
    let verticalAccuracyMeters = location.verticalAccuracy >= 0 ? location.verticalAccuracy : 0
    let speedMetersPerSecond = location.speed >= 0 ? location.speed : 0
    let headingDegrees = includeHeading && location.course >= 0 ? location.course : 0
    let timestampSeconds = location.timestamp.timeIntervalSince1970
    let timestampUnixNs = UInt64(max(0, timestampSeconds * Double(nanosecondsPerSecond)))

    return RuntimeHostLocationSample(
        latitudeDegrees: location.coordinate.latitude,
        longitudeDegrees: location.coordinate.longitude,
        altitudeMeters: location.altitude,
        horizontalAccuracyMeters: max(0, location.horizontalAccuracy),
        verticalAccuracyMeters: verticalAccuracyMeters,
        speedMetersPerSecond: speedMetersPerSecond,
        headingDegrees: headingDegrees,
        timestampUnixNs: timestampUnixNs
    )
}
