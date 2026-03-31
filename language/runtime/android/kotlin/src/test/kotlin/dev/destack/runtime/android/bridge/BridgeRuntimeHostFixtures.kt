package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.CalendarHost
import dev.destack.runtime.android.core.ContactHost
import dev.destack.runtime.android.core.DocumentHost
import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.IntentHost
import dev.destack.runtime.android.core.LifecycleHost
import dev.destack.runtime.android.core.LocationHost
import dev.destack.runtime.android.core.MediaHost
import dev.destack.runtime.android.core.NotificationHost
import dev.destack.runtime.android.core.PermissionHost
import dev.destack.runtime.android.core.RendererSurface
import dev.destack.runtime.android.core.RendererSurfaceKind
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.background.BackgroundRequests
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundStatus
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundStatusResponse
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundCompleteRequest
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskOptions
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskResult
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTriggerTestRequest
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTriggerTestResponse
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundListResponse
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundUnregisterRequest
import dev.destack.runtime.android.module.background.UnsupportedBackgroundRequests
import dev.destack.runtime.android.module.calendar.CalendarRequests
import dev.destack.runtime.android.module.calendar.UnsupportedCalendarRequests
import dev.destack.runtime.android.module.contact.ContactRequests
import dev.destack.runtime.android.module.contact.UnsupportedContactRequests
import dev.destack.runtime.android.module.document.DocumentEvents
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult
import dev.destack.runtime.android.module.intent.IntentEvents
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
import dev.destack.runtime.android.module.intent.IntentRequests
import dev.destack.runtime.android.module.lifecycle.LifecycleEvents
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleEvent
import dev.destack.runtime.android.module.location.LocationEvents
import dev.destack.runtime.android.module.location.LocationRequests
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.location.UnsupportedLocationRequests
import dev.destack.runtime.android.module.media.MediaRequests
import dev.destack.runtime.android.module.media.UnsupportedMediaRequests
import dev.destack.runtime.android.module.notification.NotificationEvents
import dev.destack.runtime.android.module.notification.NotificationRequests
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.permission.PermissionEvents
import dev.destack.runtime.android.module.permission.PermissionRequests
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent

/**
 * Record background requests for bridge tests.
 */
class BackgroundRequestRecorder : BackgroundRequests {
    var statusResponse = RuntimeHostBackgroundStatusResponse(
        status = 0,
        schedulerStatus = RuntimeHostBackgroundStatus.Available,
    )
    var listResponse = RuntimeHostBackgroundListResponse(status = 0)
    val registerCalls: MutableList<RuntimeHostBackgroundTaskOptions> = mutableListOf()
    val unregisterCalls: MutableList<String> = mutableListOf()
    val triggerCalls: MutableList<String> = mutableListOf()
    val completeCalls: MutableList<Pair<String, RuntimeHostBackgroundTaskResult>> = mutableListOf()
    var registerStatus: Int = 0
    var unregisterStatus: Int = 0
    var triggerResponse = RuntimeHostBackgroundTriggerTestResponse(
        status = 0,
        isTriggered = true,
    )
    var completeStatus: Int = 0

    override fun status(): RuntimeHostBackgroundStatusResponse {
        return statusResponse
    }

    override fun list(): RuntimeHostBackgroundListResponse {
        return listResponse
    }

    override fun registerTask(
        request: RuntimeHostBackgroundTaskOptions,
    ): Int {
        registerCalls += request

        return registerStatus
    }

    override fun unregister(
        request: RuntimeHostBackgroundUnregisterRequest,
    ): Int {
        unregisterCalls += request.identifier

        return unregisterStatus
    }

    override fun triggerTest(
        request: RuntimeHostBackgroundTriggerTestRequest,
    ): RuntimeHostBackgroundTriggerTestResponse {
        triggerCalls += request.identifier

        return triggerResponse
    }

    override fun complete(
        request: RuntimeHostBackgroundCompleteRequest,
    ): Int {
        completeCalls += request.executionId to request.result

        return completeStatus
    }
}

/**
 * Record lifecycle events for bridge tests.
 */
private class RecordingLifecycleSink : LifecycleEvents {
    val events: MutableList<RuntimeHostLifecycleEvent> = mutableListOf()

    override fun sendLifecycleEvent(
        event: RuntimeHostLifecycleEvent,
    ) {
        events += event
    }
}

/**
 * Record permission events for bridge tests.
 */
private class RecordingPermissionEventSink : PermissionEvents {
    val events: MutableList<RuntimeHostPermissionEvent> = mutableListOf()

    override fun notifyPermissionResult(
        event: RuntimeHostPermissionEvent,
    ) {
        events += event
    }
}

/**
 * Record document results for bridge tests.
 */
private class RecordingDocumentEventSink : DocumentEvents {
    val results: MutableList<RuntimeHostDocumentResult> = mutableListOf()

    override fun notifyDocumentResult(
        requestId: dev.destack.runtime.android.core.HostRequestId,
        documents: List<RuntimeHostDocumentDescriptor>,
    ) {
        results += RuntimeHostDocumentResult(
            requestId = requestId,
            documents = documents,
        )
    }
}

/**
 * Record location samples for bridge tests.
 */
private class RecordingLocationEventSink : LocationEvents {
    val samples: MutableList<Pair<String, RuntimeHostLocationSample>> = mutableListOf()

    override fun notifyLocationSample(
        watchId: String,
        sample: RuntimeHostLocationSample,
    ) {
        samples += watchId to sample
    }
}

/**
 * Record intent events for bridge tests.
 */
private class RecordingIntentEventSink : IntentEvents {
    val events: MutableList<RuntimeHostIntentEvent> = mutableListOf()

    override fun notifyIntentEvent(
        event: RuntimeHostIntentEvent,
    ) {
        events += event
    }
}

/**
 * Record notification events for bridge tests.
 */
private class RecordingNotificationEventSink : NotificationEvents {
    val events: MutableList<RuntimeHostNotificationEvent> = mutableListOf()

    override fun notifyNotificationEvent(
        event: RuntimeHostNotificationEvent,
    ) {
        events += event
    }
}

/**
 * Create one runtime host fixture for bridge tests.
 */
fun createBridgeRuntimeHost(
    sessionHandle: HostSessionHandle,
    permissionRequests: PermissionRequests = PermissionRequestRecorder(),
    documentRequests: DocumentRequests = DocumentRequestRecorder(),
    backgroundRequests: BackgroundRequests = UnsupportedBackgroundRequests,
    contactRequests: ContactRequests = UnsupportedContactRequests,
    calendarRequests: CalendarRequests = UnsupportedCalendarRequests,
    intentRequests: IntentRequests = IntentRequestRecorder(),
    locationRequests: LocationRequests = UnsupportedLocationRequests,
    mediaRequests: MediaRequests = UnsupportedMediaRequests,
    notificationRequests: NotificationRequests = NotificationRequestRecorder(),
): RuntimeHost {
    val runtimeHost = RuntimeHost(
        sessionHandle = sessionHandle,
        embedderId = HostEmbedderId(rawValue = 11),
        lifecycle = LifecycleHost(RecordingLifecycleSink()),
        permission = PermissionHost(permissionRequests, RecordingPermissionEventSink()),
        document = DocumentHost(documentRequests, RecordingDocumentEventSink()),
        contact = ContactHost(contactRequests),
        calendar = CalendarHost(calendarRequests),
        intent = IntentHost(intentRequests, RecordingIntentEventSink()),
        location = LocationHost(locationRequests, RecordingLocationEventSink()),
        media = MediaHost(mediaRequests),
        notification = NotificationHost(
            notificationRequests,
            RecordingNotificationEventSink(),
        ),
        rendererSurface = RendererSurface(
            kind = RendererSurfaceKind.SurfaceView,
            identifier = "test",
        ),
    )

    runtimeHost.updateBackgroundRequests(backgroundRequests)

    return runtimeHost
}
