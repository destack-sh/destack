package dev.destack.runtime.android

import android.net.Uri

import androidx.lifecycle.Lifecycle

import dev.destack.runtime.android.embedder.ActivityEmbedder
import dev.destack.runtime.android.embedder.SavedInteractiveState
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.module.contact.RuntimeHostContactQuery
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleState
import dev.destack.runtime.android.module.permission.RuntimeHostPermission
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Attach one runtime host to one activity-equivalent environment and roundtrip host flows.
 */
class ActivityEmbedderTest {
    @Test
    fun testActivityEmbedderSmokePath() {
        val lifecycleEvents = RecordingLifecycleSink()
        val permissionRequests = RecordingPermissionRequestHandler()
        val permissionEvents = RecordingPermissionEventSink()
        val documentRequests = RecordingDocumentRequestHandler()
        val documentEvents = RecordingDocumentEventSink()
        val contactRequests = RecordingContactRequestHandler()
        val runtimeHost = createRuntimeHost(
            lifecycleEvents = lifecycleEvents,
            permissionRequests = permissionRequests,
            permissionEvents = permissionEvents,
            documentRequests = documentRequests,
            documentEvents = documentEvents,
            contactRequests = contactRequests,
        )
        val owner = TestLifecycleOwner()
        val registry = RecordingActivityResultRegistry()
        val embedder = ActivityEmbedder.attach(
            lifecycleOwner = owner,
            activityResultRegistry = registry,
            runtimeHost = runtimeHost,
        )

        owner.handleEvent(Lifecycle.Event.ON_CREATE)
        owner.handleEvent(Lifecycle.Event.ON_START)
        owner.handleEvent(Lifecycle.Event.ON_RESUME)

        embedder.permission.request(
            RuntimeHostPermissionRequest(
                requestId = HostRequestId(rawValue = 1),
                permission = RuntimeHostPermission.Camera,
            ),
        )
        registry.completeLastLaunch(true)

        embedder.document.pick(
            RuntimeHostDocumentRequest(
                requestId = HostRequestId(rawValue = 2),
                multiple = true,
                contentTypes = listOf("image/png"),
            ),
        )
        registry.completeLastLaunch(emptyList<Uri>())
        runtimeHost.contact.list(
            RuntimeHostContactQuery(
                includeEmails = true,
            ),
        )

        owner.handleEvent(Lifecycle.Event.ON_PAUSE)
        owner.handleEvent(Lifecycle.Event.ON_STOP)
        embedder.detach()

        assertEquals(
            listOf(
                activityLifecycleEvent(RuntimeHostLifecycleState.Initializing),
                activityLifecycleEvent(RuntimeHostLifecycleState.Running),
                activityLifecycleEvent(RuntimeHostLifecycleState.Running),
                activityLifecycleEvent(RuntimeHostLifecycleState.Paused),
                activityLifecycleEvent(RuntimeHostLifecycleState.Stopped),
            ),
            lifecycleEvents.events,
        )
        assertEquals(
            listOf(
                RuntimeHostPermissionEvent(
                    requestId = HostRequestId(rawValue = 1),
                    permission = RuntimeHostPermission.Camera,
                    isGranted = true,
                ),
            ),
            permissionEvents.events,
        )
        assertEquals(
            listOf(
                RuntimeHostDocumentResult(
                    requestId = HostRequestId(rawValue = 2),
                    documents = emptyList(),
                ),
            ),
            documentEvents.results,
        )
        assertTrue(permissionRequests.requests.isEmpty())
        assertTrue(documentRequests.requests.isEmpty())
        assertEquals(
            listOf(
                RuntimeHostContactQuery(
                    includeEmails = true,
                ),
            ),
            contactRequests.listQueries,
        )
    }

    /**
     * Restore one in-flight interactive request across activity embedder recreation.
     */
    @Test
    fun testActivityEmbedderRestoresInteractiveRequestState() {
        val owner = TestLifecycleOwner()
        val registry = RecordingActivityResultRegistry()
        val runtimeHost = createRuntimeHost()
        val savedInteractiveState = SavedInteractiveState()
        val embedder = ActivityEmbedder.attach(
            lifecycleOwner = owner,
            savedInteractiveState = savedInteractiveState,
            activityResultRegistry = registry,
            runtimeHost = runtimeHost,
        )
        val request = RuntimeHostPermissionRequest(
            requestId = HostRequestId(rawValue = 9),
            permission = RuntimeHostPermission.Camera,
        )

        owner.handleEvent(Lifecycle.Event.ON_CREATE)
        owner.handleEvent(Lifecycle.Event.ON_START)
        embedder.permission.request(request)

        embedder.detach()

        val restoredOwner = TestLifecycleOwner()
        val restoredRuntimeHost = createRuntimeHost()
        ActivityEmbedder.attach(
            lifecycleOwner = restoredOwner,
            savedInteractiveState = savedInteractiveState,
            activityResultRegistry = RecordingActivityResultRegistry(),
            runtimeHost = restoredRuntimeHost,
        )

        assertEquals(request.requestId, restoredRuntimeHost.pendingPermissionRequestId())
        assertEquals(request.permission, restoredRuntimeHost.pendingPermission())
    }
}
