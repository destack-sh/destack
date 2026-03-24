package dev.destack.runtime.android.bridge.location

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.bridge.RuntimeHostLibraryLoader
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample

/**
 * The default location ABI resolved through the Android JNI bridge.
 */
internal object ProcessLocationAbi : LocationAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun notifyLocationSample(
        sessionHandle: HostSessionHandle,
        watchId: String,
        sample: RuntimeHostLocationSample,
    ): RuntimeAbiStatus {
        val values = nativeNotifyLocationSample(
            sessionHandle.rawValue,
            watchId,
            sample.latitudeDegrees,
            sample.longitudeDegrees,
            sample.altitudeMeters,
            sample.horizontalAccuracyMeters,
            sample.verticalAccuracyMeters,
            sample.speedMetersPerSecond,
            sample.headingDegrees,
            sample.timestampUnixNs,
        )

        return RuntimeAbiStatus(
            code = values[0].toInt(),
            errorId = values[1],
        )
    }

    private external fun nativeNotifyLocationSample(
        sessionHandle: Long,
        watchId: String,
        latitudeDegrees: Double,
        longitudeDegrees: Double,
        altitudeMeters: Double,
        horizontalAccuracyMeters: Double,
        verticalAccuracyMeters: Double,
        speedMetersPerSecond: Double,
        headingDegrees: Double,
        timestampUnixNs: Long,
    ): LongArray
}
