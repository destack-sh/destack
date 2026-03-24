package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RendererSurface
import dev.destack.runtime.android.core.RendererSurfaceKind
import dev.destack.runtime.android.core.RuntimeHost
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
    contactRequests: ContactRequests = UnsupportedContactRequests,
    calendarRequests: CalendarRequests = UnsupportedCalendarRequests,
    intentRequests: IntentRequests = IntentRequestRecorder(),
    locationRequests: LocationRequests = UnsupportedLocationRequests,
    mediaRequests: MediaRequests = UnsupportedMediaRequests,
    notificationRequests: NotificationRequests = NotificationRequestRecorder(),
): RuntimeHost {
    return RuntimeHost(
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
}
