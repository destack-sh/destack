package dev.destack.runtime.android

import androidx.lifecycle.Lifecycle

import dev.destack.runtime.android.embedder.ActivityResults
import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.module.permission.PermissionActivityResults
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Register one stable launcher and emit one normalized permission event.
 */
class PermissionActivityResultsTest {
    @Test
    fun testPermissionActivityResultsEmitPermissionEvents() {
        val registry = RecordingActivityResultRegistry()
        val owner = TestLifecycleOwner()
        val permissionEvents = RecordingPermissionEventSink()
        val runtimeHost = createRuntimeHost(
            permissionEvents = permissionEvents,
        )
        val activityResults = ActivityResults(
            embedderId = HostEmbedderId(rawValue = 11),
            registry = registry,
            lifecycleOwner = owner,
        )
        val permissionActivityResults = PermissionActivityResults(
            runtimeHost = runtimeHost,
            activity = null,
            activityResults = activityResults,
            permissionEvents = permissionEvents,
        )
        val request = RuntimeHostPermissionRequest(
            requestId = HostRequestId(rawValue = 2),
            permission = "android.permission.CAMERA",
        )

        owner.handleEvent(Lifecycle.Event.ON_CREATE)
        owner.handleEvent(Lifecycle.Event.ON_START)
        permissionActivityResults.request(request)
        val requestCode = registry.launchedRequestCodes.last()

        assertEquals(
            listOf(
                "androidx.activity.result.contract.ActivityResultContracts\$RequestPermission",
            ),
            registry.launchedContracts,
        )
        assertEquals(
            request.permission,
            registry.launchedInputs.getValue(requestCode),
        )

        registry.completeLastLaunch(
            result = true,
        )

        assertEquals(
            listOf(
                RuntimeHostPermissionEvent(
                    requestId = HostRequestId(rawValue = 2),
                    permission = "android.permission.CAMERA",
                    isGranted = true,
                ),
            ),
            permissionEvents.events,
        )
    }
}
