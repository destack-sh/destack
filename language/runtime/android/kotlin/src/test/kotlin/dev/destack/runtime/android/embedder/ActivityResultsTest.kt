package dev.destack.runtime.android.embedder

import android.net.Uri

import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.contract.ActivityResultContract
import androidx.core.app.ActivityOptionsCompat

import dev.destack.runtime.android.core.CalendarHost
import dev.destack.runtime.android.core.ContactHost
import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.DocumentHost
import dev.destack.runtime.android.core.IntentHost
import dev.destack.runtime.android.core.LifecycleHost
import dev.destack.runtime.android.core.LocationHost
import dev.destack.runtime.android.core.MediaHost
import dev.destack.runtime.android.core.NotificationHost
import dev.destack.runtime.android.core.PermissionHost
import dev.destack.runtime.android.core.RendererSurface
import dev.destack.runtime.android.core.RendererSurfaceKind
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.document.DocumentActivityResults
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult
import dev.destack.runtime.android.module.document.DocumentEvents
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.calendar.UnsupportedCalendarRequests
import dev.destack.runtime.android.module.contact.UnsupportedContactRequests
import dev.destack.runtime.android.module.intent.IntentEvents
import dev.destack.runtime.android.module.intent.IntentRequests
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleEvent
import dev.destack.runtime.android.module.location.LocationEvents
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.location.UnsupportedLocationRequests
import dev.destack.runtime.android.module.media.UnsupportedMediaRequests
import dev.destack.runtime.android.module.permission.PermissionActivityResults
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest
import dev.destack.runtime.android.module.lifecycle.LifecycleEvents
import dev.destack.runtime.android.module.notification.NotificationEvents
import dev.destack.runtime.android.module.notification.NotificationRequests
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest
import dev.destack.runtime.android.module.permission.PermissionEvents
import dev.destack.runtime.android.module.permission.PermissionRequests

import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry

import org.junit.Assert.assertEquals
import org.junit.Test

private class RecordingActivityResultLifecycleSink : LifecycleEvents {
    val events: MutableList<RuntimeHostLifecycleEvent> = mutableListOf()

    override fun sendLifecycleEvent(
        event: RuntimeHostLifecycleEvent,
    ) {
        events += event
    }
}

private class NoopDocumentRequestHandler : DocumentRequests {
    override fun pick(
        request: RuntimeHostDocumentRequest,
    ): Int = 0
}

private class NoopPermissionRequestHandler : PermissionRequests {
    override fun request(
        request: RuntimeHostPermissionRequest,
    ): Int = 0

    override fun openSettings(): Int {
        return 1
    }
}

private class RecordingActivityResultPermissionEventSink : PermissionEvents {
    val events: MutableList<RuntimeHostPermissionEvent> = mutableListOf()

    override fun notifyPermissionResult(
        event: RuntimeHostPermissionEvent,
    ) {
        events += event
    }
}

private class RecordingActivityResultDocumentEventSink : DocumentEvents {
    val results: MutableList<RuntimeHostDocumentResult> = mutableListOf()

    override fun notifyDocumentResult(
        requestId: HostRequestId,
        documents: List<RuntimeHostDocumentDescriptor>,
    ) {
        results += RuntimeHostDocumentResult(
            requestId = requestId,
            documents = documents,
        )
    }
}

private class NoopLocationEventSink : LocationEvents {
    override fun notifyLocationSample(
        watchId: String,
        sample: RuntimeHostLocationSample,
    ) {}
}

private class TestActivityResultLifecycleOwner : LifecycleOwner {
    private val lifecycleRegistry = LifecycleRegistry.createUnsafe(this)

    override val lifecycle: Lifecycle
        get() = lifecycleRegistry

    fun handleEvent(
        event: Lifecycle.Event,
    ) {
        lifecycleRegistry.handleLifecycleEvent(event)
    }
}

private class RecordingActivityResultRegistry : ActivityResultRegistry() {
    val launchedContracts: MutableList<String> = mutableListOf()
    val launchedInputs: MutableMap<Int, Any?> = mutableMapOf()
    val launchedRequestCodes: MutableList<Int> = mutableListOf()

    override fun <I, O> onLaunch(
        requestCode: Int,
        contract: ActivityResultContract<I, O>,
        input: I,
        options: ActivityOptionsCompat?,
    ) {
        launchedRequestCodes += requestCode
        launchedContracts += contract.javaClass.name
        launchedInputs[requestCode] = input
    }

    fun <O> completeLastLaunch(
        result: O,
    ) {
        val requestCode = launchedRequestCodes.last()

        dispatchResult(requestCode, result)
    }
}

private fun createActivityResultRuntimeHost(
    lifecycleEvents: LifecycleEvents = RecordingActivityResultLifecycleSink(),
    permissionRequests: PermissionRequests = NoopPermissionRequestHandler(),
    permissionEvents: PermissionEvents = RecordingActivityResultPermissionEventSink(),
    documentRequests: DocumentRequests = NoopDocumentRequestHandler(),
    documentEvents: DocumentEvents = RecordingActivityResultDocumentEventSink(),
): RuntimeHost {
    return RuntimeHost(
        sessionHandle = HostSessionHandle(rawValue = 7),
        embedderId = HostEmbedderId(rawValue = 11),
        lifecycle = LifecycleHost(lifecycleEvents),
        permission = PermissionHost(permissionRequests, permissionEvents),
        document = DocumentHost(documentRequests, documentEvents),
        contact = ContactHost(UnsupportedContactRequests),
        calendar = CalendarHost(UnsupportedCalendarRequests),
        intent = IntentHost(object : IntentRequests {
            override fun canOpenUrl(
                url: String,
            ): Boolean = false

            override fun openUrl(
                url: String,
            ): Int = 0

            override fun openPath(
                path: String,
            ): Int = 0

            override fun shareText(
                text: String,
                mimeType: String?,
            ): Int = 0

            override fun sharePaths(
                paths: List<String>,
                mimeType: String?,
            ): Int = 0
        }, IntentEvents { _ -> }),
        location = LocationHost(UnsupportedLocationRequests, NoopLocationEventSink()),
        media = MediaHost(UnsupportedMediaRequests),
        notification = NotificationHost(object : NotificationRequests {
            override fun post(
                request: RuntimeHostNotificationRequest,
            ): Int = 0

            override fun cancel(
                identifier: String,
            ): Int = 0

            override fun cancelAll(): Int = 0
        }, object : NotificationEvents {
            override fun notifyNotificationEvent(
                event: RuntimeHostNotificationEvent,
            ) {}
        }),
        rendererSurface = RendererSurface(
            kind = RendererSurfaceKind.SurfaceView,
            identifier = "main-surface",
        ),
    )
}

/**
 * Register one stable launcher set per runtime session and complete one document request.
 */
class ActivityResultsTest {
    @Test
    fun testDocumentActivityResultsRegisterStableLaunchersAndCompleteRequests() {
        val registry = RecordingActivityResultRegistry()
        val owner = TestActivityResultLifecycleOwner()
        val documentEvents = RecordingActivityResultDocumentEventSink()
        val runtimeHost = createActivityResultRuntimeHost(
            documentEvents = documentEvents,
        )
        val activityResults = ActivityResults(
            embedderId = HostEmbedderId(rawValue = 11),
            registry = registry,
            lifecycleOwner = owner,
        )
        val documentActivityResults = DocumentActivityResults(
            runtimeHost = runtimeHost,
            activityResults = activityResults,
        )
        val request = RuntimeHostDocumentRequest(
            requestId = HostRequestId(rawValue = 1),
            allowsMultipleSelection = true,
            contentTypes = listOf("image/png"),
        )

        owner.handleEvent(Lifecycle.Event.ON_CREATE)
        owner.handleEvent(Lifecycle.Event.ON_START)
        documentActivityResults.pick(request)
        val requestCode = registry.launchedRequestCodes.last()

        assertEquals(
            listOf(
                "androidx.activity.result.contract.ActivityResultContracts\$OpenMultipleDocuments",
            ),
            registry.launchedContracts,
        )
        assertEquals(
            listOf("image/png"),
            (registry.launchedInputs.getValue(requestCode) as Array<*>).map { it as String },
        )

        registry.completeLastLaunch(
            result = emptyList<Uri>(),
        )

        assertEquals(
            listOf(
                RuntimeHostDocumentResult(
                    requestId = HostRequestId(rawValue = 1),
                    documents = emptyList<RuntimeHostDocumentDescriptor>(),
                ),
            ),
            documentEvents.results,
        )
    }

    /**
     * Register one stable launcher and emit one normalized permission event.
     */
    @Test
    fun testPermissionActivityResultsEmitPermissionEvents() {
        val registry = RecordingActivityResultRegistry()
        val owner = TestActivityResultLifecycleOwner()
        val permissionEvents = RecordingActivityResultPermissionEventSink()
        val runtimeHost = createActivityResultRuntimeHost(
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
