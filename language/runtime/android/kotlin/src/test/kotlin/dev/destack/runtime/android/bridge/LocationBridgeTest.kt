package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.location.RuntimeHostLocationAccuracy
import dev.destack.runtime.android.module.location.RuntimeHostLocationLastKnownResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.location.RuntimeHostLocationServicesResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationWatchOptions

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Exercise the location bridge lane.
 */
class LocationBridgeTest {
    /**
     * Route location requests and samples into the attached runtime host.
     */
    @Test
    fun testLocationRequestsAndIngressRouteThroughRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val locationRequests = LocationRequestRecorder().also {
            it.servicesEnabledResponse = RuntimeHostLocationServicesResponse(
                status = 0,
                isEnabled = true,
            )
            it.lastKnownResponse = RuntimeHostLocationLastKnownResponse(
                status = 0,
                sample = sampleLocation(),
            )
        }
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
            locationRequests = locationRequests,
        )

        bridge.attach(runtimeHost)

        val servicesEnabled = bridge.locationServicesEnabled()
        val lastKnown = bridge.locationLastKnown()
        val openStatus = bridge.locationWatchOpen(
            watchId = "watch-1",
            accuracy = 4,
            minimumIntervalNs = 50_000_000L,
            minimumDistanceMeters = 2.5,
            includeHeading = true,
        )

        bridge.notifyLocationSample("watch-1", sampleLocation())

        val closeStatus = bridge.locationWatchClose("watch-1")

        assertEquals(RuntimeHostLocationServicesResponse(status = 0, isEnabled = true), servicesEnabled)
        assertEquals(RuntimeHostLocationLastKnownResponse(status = 0, sample = sampleLocation()), lastKnown)
        assertEquals(
            listOf(
                "watch-1" to RuntimeHostLocationWatchOptions(
                    accuracy = RuntimeHostLocationAccuracy.High,
                    minimumIntervalNs = 50_000_000L,
                    minimumDistanceMeters = 2.5,
                    includeHeading = true,
                ),
            ),
            locationRequests.watchOpenCalls,
        )
        assertEquals(listOf("watch-1"), locationRequests.watchCloseCalls)
        assertEquals(0, openStatus)
        assertEquals(0, closeStatus)
        assertEquals(
            listOf(
                Triple(sessionHandle, "watch-1", sampleLocation()),
            ),
            runtimeApi.locationSamples,
        )
    }

    /**
     * Reject invalid location accuracies before touching the runtime host.
     */
    @Test
    fun testLocationBridgeRejectsInvalidAccuracy() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val locationRequests = LocationRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
            locationRequests = locationRequests,
        )

        bridge.attach(runtimeHost)

        val openStatus = bridge.locationWatchOpen(
            watchId = "watch-1",
            accuracy = 99,
            minimumIntervalNs = 0L,
            minimumDistanceMeters = 0.0,
            includeHeading = false,
        )

        assertEquals(2, openStatus)
        assertTrue(locationRequests.watchOpenCalls.isEmpty())
    }

}

/**
 * Build one representative location sample fixture.
 */
private fun sampleLocation(): RuntimeHostLocationSample {
    return RuntimeHostLocationSample(
        latitudeDegrees = 47.3769,
        longitudeDegrees = 8.5417,
        altitudeMeters = 408.0,
        horizontalAccuracyMeters = 5.0,
        verticalAccuracyMeters = 8.0,
        speedMetersPerSecond = 1.25,
        headingDegrees = 180.0,
        timestampUnixNs = 1_700_000_000_000_000_000L,
    )
}
