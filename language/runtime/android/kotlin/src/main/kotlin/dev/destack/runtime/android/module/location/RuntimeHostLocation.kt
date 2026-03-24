package dev.destack.runtime.android.module.location

/**
 * The Android location accuracy preference.
 */
public enum class RuntimeHostLocationAccuracy {
    /**
     * Passive updates with minimal power use.
     */
    Passive,

    /**
     * Coarse accuracy.
     */
    Low,

    /**
     * Balanced power and accuracy.
     */
    Balanced,

    /**
     * Fine accuracy.
     */
    High,

    /**
     * Best available accuracy.
     */
    Best,
}

/**
 * One Android location watch request.
 */
public data class RuntimeHostLocationWatchOptions(
    /**
     * The requested accuracy preference.
     */
    val accuracy: RuntimeHostLocationAccuracy,

    /**
     * The minimum interval between updates in nanoseconds.
     */
    val minimumIntervalNs: Long,

    /**
     * The minimum distance delta in meters.
     */
    val minimumDistanceMeters: Double,

    /**
     * Whether heading should be included when available.
     */
    val includeHeading: Boolean,
)

/**
 * One Android location sample.
 */
public data class RuntimeHostLocationSample(
    /**
     * The latitude in degrees.
     */
    val latitudeDegrees: Double,

    /**
     * The longitude in degrees.
     */
    val longitudeDegrees: Double,

    /**
     * The altitude in meters above mean sea level.
     */
    val altitudeMeters: Double,

    /**
     * The horizontal accuracy radius in meters.
     */
    val horizontalAccuracyMeters: Double,

    /**
     * The vertical accuracy in meters.
     */
    val verticalAccuracyMeters: Double,

    /**
     * The speed in meters per second.
     */
    val speedMetersPerSecond: Double,

    /**
     * The heading in degrees.
     */
    val headingDegrees: Double,

    /**
     * The UTC timestamp in nanoseconds.
     */
    val timestampUnixNs: Long,
)

/**
 * One Android location-services response.
 */
public data class RuntimeHostLocationServicesResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * Whether location services are enabled.
     */
    val isEnabled: Boolean = false,
)

/**
 * One Android last-known location response.
 */
public data class RuntimeHostLocationLastKnownResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned sample when available.
     */
    val sample: RuntimeHostLocationSample? = null,
)
