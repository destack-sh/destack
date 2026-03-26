package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RendererSurface
import dev.destack.runtime.android.core.RendererSurfaceKind
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.background.BackgroundRequests
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundStatus
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundStatusResponse
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskListResponse
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskOptions
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskResult
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTriggerResponse
import dev.destack.runtime.android.module.background.UnsupportedBackgroundRequests
import dev.destack.runtime.android.module.calendar.CalendarRequests
import dev.destack.runtime.android.module.calendar.UnsupportedCalendarRequests
import dev.destack.runtime.android.module.contact.ContactRequests
import dev.destack.runtime.android.module.contact.UnsupportedContactRequests
import dev.destack.runtime.android.module.document.DocumentEvents
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult
import dev.destack.runtime.android.module.intent.IntentEvents
import dev.destack.runtime.android.module.intent.IntentRequests
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
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
    var listResponse = RuntimeHostBackgroundTaskListResponse(status = 0)
    val registerCalls: MutableList<RuntimeHostBackgroundTaskOptions> = mutableListOf()
    val unregisterCalls: MutableList<String> = mutableListOf()
    val triggerCalls: MutableList<String> = mutableListOf()
    val completeCalls: MutableList<Pair<String, RuntimeHostBackgroundTaskResult>> = mutableListOf()
    var registerStatus: Int = 0
    var unregisterStatus: Int = 0
    var triggerResponse = RuntimeHostBackgroundTriggerResponse(
        status = 0,
        isTriggered = true,
    )
    var completeStatus: Int = 0

    override fun backgroundStatus(): RuntimeHostBackgroundStatusResponse {
        return statusResponse
    }

    override fun listBackgroundTasks(): RuntimeHostBackgroundTaskListResponse {
        return listResponse
    }

    override fun registerBackgroundTask(
        options: RuntimeHostBackgroundTaskOptions,
    ): Int {
        registerCalls += options

        return registerStatus
    }

    override fun unregisterBackgroundTask(
        identifier: String,
    ): Int {
        unregisterCalls += identifier

        return unregisterStatus
    }

    override fun triggerBackgroundTask(
        identifier: String,
    ): RuntimeHostBackgroundTriggerResponse {
        triggerCalls += identifier

        return triggerResponse
    }

    override fun completeBackgroundTask(
        executionId: String,
        result: RuntimeHostBackgroundTaskResult,
    ): Int {
        completeCalls += executionId to result

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

    override fun sendPermissionEvent(
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

    override fun sendDocumentResult(
        result: RuntimeHostDocumentResult,
    ) {
        results += result
    }
}

/**
 * Record intent events for bridge tests.
 */
private class RecordingIntentEventSink : IntentEvents {
    val events: MutableList<RuntimeHostIntentEvent> = mutableListOf()

    override fun sendIntentEvent(
        event: RuntimeHostIntentEvent,
    ) {
        events += event
    }
}

/**
 * Record location samples for bridge tests.
 */
private class RecordingLocationEventSink : LocationEvents {
    val samples: MutableList<Pair<String, RuntimeHostLocationSample>> = mutableListOf()

    override fun sendLocationSample(
        watchId: String,
        sample: RuntimeHostLocationSample,
    ) {
        samples += watchId to sample
    }
}

/**
 * Record notification events for bridge tests.
 */
private class RecordingNotificationEventSink : NotificationEvents {
    val events: MutableList<RuntimeHostNotificationEvent> = mutableListOf()

    override fun sendNotificationEvent(
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
        lifecycleEvents = RecordingLifecycleSink(),
        permissionRequests = permissionRequests,
        permissionEvents = RecordingPermissionEventSink(),
        documentRequests = documentRequests,
        documentEvents = RecordingDocumentEventSink(),
        contactRequests = contactRequests,
        calendarRequests = calendarRequests,
        intentRequests = intentRequests,
        intentEvents = RecordingIntentEventSink(),
        locationRequests = locationRequests,
        locationEvents = RecordingLocationEventSink(),
        mediaRequests = mediaRequests,
        notificationRequests = notificationRequests,
        notificationEvents = RecordingNotificationEventSink(),
        rendererSurface = RendererSurface(
            kind = RendererSurfaceKind.SurfaceView,
            identifier = "test",
        ),
    )

    runtimeHost.backgroundRequests = backgroundRequests

    return runtimeHost
}
