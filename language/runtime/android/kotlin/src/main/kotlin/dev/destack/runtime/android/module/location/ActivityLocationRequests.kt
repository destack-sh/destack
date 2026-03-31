package dev.destack.runtime.android.module.location

import android.annotation.SuppressLint
import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.Looper

import androidx.activity.ComponentActivity
import androidx.core.content.ContextCompat
import dev.destack.runtime.android.core.hostStatusFailed
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.hostStatusPermissionDenied

private const val nanosecondsPerMillisecond: Long = 1_000_000L

/**
 * The Android location request surface backed by one attached activity.
 */
public class ActivityLocationRequests(
    private val activity: ComponentActivity?,
    private val events: LocationEvents,
) : LocationRequests {
    private val locationManager: LocationManager? = activity?.getSystemService(LocationManager::class.java)
    private val watchListeners: MutableMap<String, LocationListener> = mutableMapOf()

    override fun servicesEnabled(): RuntimeHostLocationServicesResponse {
        val locationManager = locationManager
            ?: return RuntimeHostLocationServicesResponse(status = hostStatusNotSupported)

        return servicesEnabled(locationManager)
    }

    override fun lastKnown(): RuntimeHostLocationLastKnownResponse {
        val activity = activity
            ?: return RuntimeHostLocationLastKnownResponse(status = hostStatusNotSupported)
        val locationManager = locationManager
            ?: return RuntimeHostLocationLastKnownResponse(status = hostStatusNotSupported)

        return lastKnown(activity, locationManager)
    }

    override fun watchOpen(
        watchId: String,
        options: RuntimeHostLocationWatchOptions,
    ): Int {
        val activity = activity
            ?: return hostStatusNotSupported
        val locationManager = locationManager
            ?: return hostStatusNotSupported

        return watchOpen(
            context = activity,
            locationManager = locationManager,
            watchListeners = watchListeners,
            watchId = watchId,
            options = options,
            events = events,
        )
    }

    override fun watchClose(
        watchId: String,
    ): Int {
        val locationManager = locationManager
            ?: return hostStatusNotSupported

        return watchClose(
            locationManager = locationManager,
            watchListeners = watchListeners,
            watchId = watchId,
        )
    }
}

private fun servicesEnabled(
    locationManager: LocationManager,
): RuntimeHostLocationServicesResponse {
    // query the platform location-service state without requiring one active watch
    val isEnabled = try {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            locationManager.isLocationEnabled
        }
        else {
            locationManager.getProviders(true).isNotEmpty()
        }
    }
    catch (_: Exception) {
        return RuntimeHostLocationServicesResponse(status = hostStatusFailed)
    }

    return RuntimeHostLocationServicesResponse(
        status = hostStatusOk,
        isEnabled = isEnabled,
    )
}

@SuppressLint("MissingPermission")
private fun lastKnown(
    context: Context,
    locationManager: LocationManager,
): RuntimeHostLocationLastKnownResponse {
    // require one granted location permission before reading host location state
    val permissionStatus = locationPermissionStatus(context)
    if (permissionStatus != hostStatusOk) {
        return RuntimeHostLocationLastKnownResponse(status = permissionStatus)
    }

    // resolve the freshest cached sample from the enabled providers
    val providers = locationManager.getProviders(true)
    if (providers.isEmpty()) {
        return RuntimeHostLocationLastKnownResponse(status = hostStatusNotSupported)
    }

    val cachedLocation = try {
        providers
            .mapNotNull { provider ->
                try {
                    locationManager.getLastKnownLocation(provider)
                }
                catch (_: SecurityException) {
                    return RuntimeHostLocationLastKnownResponse(status = hostStatusPermissionDenied)
                }
                catch (_: Exception) {
                    null
                }
            }
            .maxByOrNull(Location::getTime)
    }
    catch (_: Exception) {
        return RuntimeHostLocationLastKnownResponse(status = hostStatusFailed)
    }

    // return not found when the platform has no cached sample yet
    val lastKnownLocation = cachedLocation
        ?: return RuntimeHostLocationLastKnownResponse(status = hostStatusNotFound)

    return RuntimeHostLocationLastKnownResponse(
        status = hostStatusOk,
        sample = runtimeHostLocationSample(lastKnownLocation),
    )
}

@SuppressLint("MissingPermission")
private fun watchOpen(
    context: Context,
    locationManager: LocationManager,
    watchListeners: MutableMap<String, LocationListener>,
    watchId: String,
    options: RuntimeHostLocationWatchOptions,
    events: LocationEvents,
): Int {
    // validate one stable watch identifier before mutating the live watch set
    if (watchId.isBlank()) {
        return hostStatusInvalidArgument
    }

    // reject duplicate watch identifiers loudly
    if (watchListeners.containsKey(watchId)) {
        return hostStatusInvalidArgument
    }

    // require the necessary platform location permission for the selected accuracy
    val permissionStatus = locationPermissionStatus(context, options.accuracy)
    if (permissionStatus != hostStatusOk) {
        return permissionStatus
    }

    // resolve one enabled provider that can satisfy the requested accuracy
    val provider = preferredProvider(locationManager, options.accuracy)
        ?: return hostStatusNotSupported

    // register one live listener and stream samples into the runtime ingress surface
    val listener = LocationListener { location ->
        events.notifyLocationSample(
            watchId,
            runtimeHostLocationSample(location),
        )
    }
    val minimumIntervalMs = options.minimumIntervalNs / nanosecondsPerMillisecond
    val minimumDistanceMeters = options.minimumDistanceMeters.toFloat()

    return try {
        locationManager.requestLocationUpdates(
            provider,
            minimumIntervalMs,
            minimumDistanceMeters,
            listener,
            Looper.getMainLooper(),
        )
        watchListeners[watchId] = listener
        hostStatusOk
    }
    catch (_: SecurityException) {
        hostStatusPermissionDenied
    }
    catch (_: IllegalArgumentException) {
        hostStatusInvalidArgument
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

private fun watchClose(
    locationManager: LocationManager,
    watchListeners: MutableMap<String, LocationListener>,
    watchId: String,
): Int {
    // require one live watch before attempting to remove it
    val listener = watchListeners.remove(watchId)
        ?: return hostStatusNotFound

    return try {
        locationManager.removeUpdates(listener)
        hostStatusOk
    }
    catch (_: SecurityException) {
        hostStatusPermissionDenied
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

private fun locationPermissionStatus(
    context: Context,
    accuracy: RuntimeHostLocationAccuracy? = null,
): Int {
    val hasCoarsePermission = ContextCompat.checkSelfPermission(
        context,
        Manifest.permission.ACCESS_COARSE_LOCATION,
    ) == PackageManager.PERMISSION_GRANTED
    val hasFinePermission = ContextCompat.checkSelfPermission(
        context,
        Manifest.permission.ACCESS_FINE_LOCATION,
    ) == PackageManager.PERMISSION_GRANTED

    // high-accuracy requests require fine location permission
    if (accuracy == RuntimeHostLocationAccuracy.High || accuracy == RuntimeHostLocationAccuracy.Best) {
        return if (hasFinePermission) hostStatusOk else hostStatusPermissionDenied
    }

    return if (hasCoarsePermission || hasFinePermission) {
        hostStatusOk
    }
    else {
        hostStatusPermissionDenied
    }
}

private fun preferredProvider(
    locationManager: LocationManager,
    accuracy: RuntimeHostLocationAccuracy,
): String? {
    val enabledProviders = locationManager.getProviders(true)
    if (enabledProviders.isEmpty()) {
        return null
    }

    val preferredProviders = when (accuracy) {
        RuntimeHostLocationAccuracy.Passive -> listOf(
            LocationManager.PASSIVE_PROVIDER,
            LocationManager.NETWORK_PROVIDER,
            LocationManager.GPS_PROVIDER,
        )
        RuntimeHostLocationAccuracy.Low -> listOf(
            LocationManager.NETWORK_PROVIDER,
            LocationManager.PASSIVE_PROVIDER,
            LocationManager.GPS_PROVIDER,
        )
        RuntimeHostLocationAccuracy.Balanced -> listOf(
            LocationManager.NETWORK_PROVIDER,
            LocationManager.GPS_PROVIDER,
            LocationManager.PASSIVE_PROVIDER,
        )
        RuntimeHostLocationAccuracy.High -> listOf(
            LocationManager.GPS_PROVIDER,
            LocationManager.NETWORK_PROVIDER,
            LocationManager.PASSIVE_PROVIDER,
        )
        RuntimeHostLocationAccuracy.Best -> listOf(
            LocationManager.GPS_PROVIDER,
            LocationManager.NETWORK_PROVIDER,
            LocationManager.PASSIVE_PROVIDER,
        )
    }

    return preferredProviders.firstOrNull(enabledProviders::contains)
        ?: enabledProviders.firstOrNull()
}

private fun runtimeHostLocationSample(
    location: Location,
): RuntimeHostLocationSample {
    val altitudeMeters = if (location.hasAltitude()) location.altitude else 0.0
    val horizontalAccuracyMeters = if (location.hasAccuracy()) location.accuracy.toDouble() else 0.0
    val verticalAccuracyMeters = if (
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
        location.hasVerticalAccuracy()
    ) {
        location.verticalAccuracyMeters.toDouble()
    }
    else {
        0.0
    }
    val speedMetersPerSecond = if (location.hasSpeed()) location.speed.toDouble() else 0.0
    val headingDegrees = if (location.hasBearing()) location.bearing.toDouble() else 0.0
    val timestampUnixNs = location.time * nanosecondsPerMillisecond

    return RuntimeHostLocationSample(
        latitudeDegrees = location.latitude,
        longitudeDegrees = location.longitude,
        altitudeMeters = altitudeMeters,
        horizontalAccuracyMeters = horizontalAccuracyMeters,
        verticalAccuracyMeters = verticalAccuracyMeters,
        speedMetersPerSecond = speedMetersPerSecond,
        headingDegrees = headingDegrees,
        timestampUnixNs = timestampUnixNs,
    )
}
