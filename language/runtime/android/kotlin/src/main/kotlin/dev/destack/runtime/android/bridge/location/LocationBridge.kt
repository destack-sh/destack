package dev.destack.runtime.android.bridge.location

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.module.location.LocationEvents
import dev.destack.runtime.android.module.location.RuntimeHostLocationLastKnownResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.location.RuntimeHostLocationServicesResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationWatchOptions
/**
 * One location bridge lane for one attached Android runtime host.
 */
internal class LocationBridge(
    private val sessionHandle: HostSessionHandle,
    private val bindings: LocationAbi,
) : LocationEvents {
    /**
     * Send one location sample into the runtime ingress path.
     */
    override fun sendLocationSample(
        watchId: String,
        sample: RuntimeHostLocationSample,
    ) {
        val status = bindings.notifyLocationSample(
            sessionHandle = sessionHandle,
            watchId = watchId,
            sample = sample,
        )

        require(status.code == hostStatusOk) {
            "runtime bridge could not deliver location sample: code ${status.code}, error ${status.errorId}"
        }
    }

    /**
     * Read whether Android location services are enabled through the attached host.
     */
    fun locationServicesEnabled(
        runtimeHost: RuntimeHost,
    ): RuntimeHostLocationServicesResponse {
        return runtimeHost.locationRequests.locationServicesEnabled()
    }

    /**
     * Read one last-known Android location sample through the attached host.
     */
    fun locationLastKnown(
        runtimeHost: RuntimeHost,
    ): RuntimeHostLocationLastKnownResponse {
        return runtimeHost.locationRequests.locationLastKnown()
    }

    /**
     * Open one Android location watch through the attached host.
     */
    fun locationWatchOpen(
        runtimeHost: RuntimeHost,
        watchId: String,
        options: RuntimeHostLocationWatchOptions,
    ): Int {
        return runtimeHost.locationRequests.locationWatchOpen(watchId, options)
    }

    /**
     * Close one Android location watch through the attached host.
     */
    fun locationWatchClose(
        runtimeHost: RuntimeHost,
        watchId: String,
    ): Int {
        return runtimeHost.locationRequests.locationWatchClose(watchId)
    }
}
