package dev.destack.runtime.android.module.location

import dev.destack.runtime.android.core.hostStatusNotSupported

/**
 * The location request surface attached to one Android runtime host.
 */
public interface LocationRequests {
    /**
     * Read whether Android location services are enabled.
     */
    public fun locationServicesEnabled(): RuntimeHostLocationServicesResponse

    /**
     * Read one last-known Android location sample.
     */
    public fun locationLastKnown(): RuntimeHostLocationLastKnownResponse

    /**
     * Open one Android location watch.
     */
    public fun locationWatchOpen(
        watchId: String,
        options: RuntimeHostLocationWatchOptions,
    ): Int

    /**
     * Close one Android location watch.
     */
    public fun locationWatchClose(
        watchId: String,
    ): Int
}

/**
 * The explicit unsupported location request surface for one Android runtime host.
 */
public object UnsupportedLocationRequests : LocationRequests {
    override fun locationServicesEnabled(): RuntimeHostLocationServicesResponse {
        return RuntimeHostLocationServicesResponse(status = hostStatusNotSupported)
    }

    override fun locationLastKnown(): RuntimeHostLocationLastKnownResponse {
        return RuntimeHostLocationLastKnownResponse(status = hostStatusNotSupported)
    }

    override fun locationWatchOpen(
        watchId: String,
        options: RuntimeHostLocationWatchOptions,
    ): Int {
        return hostStatusNotSupported
    }

    override fun locationWatchClose(
        watchId: String,
    ): Int {
        return hostStatusNotSupported
    }
}
