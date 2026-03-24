import Foundation

/// The Apple location accuracy preference.
public enum RuntimeHostLocationAccuracy: Int32, Sendable, Hashable, Codable {
    /// Passive updates with minimal power use.
    case passive = 1
    /// Coarse accuracy.
    case low = 2
    /// Balanced power and accuracy.
    case balanced = 3
    /// Fine accuracy.
    case high = 4
    /// Best available accuracy.
    case best = 5
}

/// One Apple location watch request.
public struct RuntimeHostLocationWatchOptions: Sendable, Hashable, Codable {
    /// The requested accuracy preference.
    public let accuracy: RuntimeHostLocationAccuracy
    /// The minimum interval between updates in nanoseconds.
    public let minimumIntervalNs: UInt64
    /// The minimum distance delta in meters.
    public let minimumDistanceMeters: Double
    /// Whether heading should be included when available.
    public let includeHeading: Bool

    /// Create one location watch request.
    public init(
        accuracy: RuntimeHostLocationAccuracy,
        minimumIntervalNs: UInt64,
        minimumDistanceMeters: Double,
        includeHeading: Bool
    ) {
        self.accuracy = accuracy
        self.minimumIntervalNs = minimumIntervalNs
        self.minimumDistanceMeters = minimumDistanceMeters
        self.includeHeading = includeHeading
    }
}

/// One Apple location sample.
public struct RuntimeHostLocationSample: Sendable, Hashable, Codable {
    /// The latitude in degrees.
    public let latitudeDegrees: Double
    /// The longitude in degrees.
    public let longitudeDegrees: Double
    /// The altitude in meters above mean sea level.
    public let altitudeMeters: Double
    /// The horizontal accuracy radius in meters.
    public let horizontalAccuracyMeters: Double
    /// The vertical accuracy in meters.
    public let verticalAccuracyMeters: Double
    /// The speed in meters per second.
    public let speedMetersPerSecond: Double
    /// The heading in degrees.
    public let headingDegrees: Double
    /// The UTC timestamp in nanoseconds.
    public let timestampUnixNs: UInt64

    /// Create one Apple location sample.
    public init(
        latitudeDegrees: Double,
        longitudeDegrees: Double,
        altitudeMeters: Double,
        horizontalAccuracyMeters: Double,
        verticalAccuracyMeters: Double,
        speedMetersPerSecond: Double,
        headingDegrees: Double,
        timestampUnixNs: UInt64
    ) {
        self.latitudeDegrees = latitudeDegrees
        self.longitudeDegrees = longitudeDegrees
        self.altitudeMeters = altitudeMeters
        self.horizontalAccuracyMeters = horizontalAccuracyMeters
        self.verticalAccuracyMeters = verticalAccuracyMeters
        self.speedMetersPerSecond = speedMetersPerSecond
        self.headingDegrees = headingDegrees
        self.timestampUnixNs = timestampUnixNs
    }
}

/// One Apple location-services response.
public struct RuntimeHostLocationServicesResponse: Sendable, Hashable, Codable {
    /// The host status code.
    public let status: UInt32
    /// Whether location services are enabled.
    public let isEnabled: Bool

    /// Create one Apple location-services response.
    public init(
        status: UInt32,
        isEnabled: Bool = false
    ) {
        self.status = status
        self.isEnabled = isEnabled
    }
}

/// One Apple last-known location response.
public struct RuntimeHostLocationLastKnownResponse: Sendable, Hashable, Codable {
    /// The host status code.
    public let status: UInt32
    /// The returned sample when available.
    public let sample: RuntimeHostLocationSample?

    /// Create one Apple last-known location response.
    public init(
        status: UInt32,
        sample: RuntimeHostLocationSample? = nil
    ) {
        self.status = status
        self.sample = sample
    }
}
