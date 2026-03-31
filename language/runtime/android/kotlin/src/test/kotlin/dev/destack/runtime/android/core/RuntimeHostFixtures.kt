package dev.destack.runtime.android

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
import dev.destack.runtime.android.module.calendar.CalendarRequests
import dev.destack.runtime.android.module.calendar.UnsupportedCalendarRequests
import dev.destack.runtime.android.module.contact.ContactRequests
import dev.destack.runtime.android.module.contact.RuntimeHostContactCreateResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactDraft
import dev.destack.runtime.android.module.contact.RuntimeHostContactPageResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactQuery
import dev.destack.runtime.android.module.contact.RuntimeHostContactResponse
import dev.destack.runtime.android.module.contact.UnsupportedContactRequests
import dev.destack.runtime.android.module.document.DocumentEvents
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.intent.IntentEvents
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
import dev.destack.runtime.android.module.intent.IntentRequests
import dev.destack.runtime.android.module.lifecycle.LifecycleEvents
import dev.destack.runtime.android.module.location.LocationEvents
import dev.destack.runtime.android.module.location.LocationRequests
import dev.destack.runtime.android.module.location.RuntimeHostLocationLastKnownResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.location.RuntimeHostLocationServicesResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationWatchOptions
import dev.destack.runtime.android.module.location.UnsupportedLocationRequests
import dev.destack.runtime.android.module.media.MediaRequests
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaListRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaListResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaReadResponse
import dev.destack.runtime.android.module.media.UnsupportedMediaRequests
import dev.destack.runtime.android.module.notification.NotificationEvents
import dev.destack.runtime.android.module.notification.NotificationRequests
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest
import dev.destack.runtime.android.module.permission.PermissionEvents
import dev.destack.runtime.android.module.permission.PermissionRequests

/**
 * Record contact requests submitted through one Android runtime host.
 */
internal class RecordingContactRequestHandler : ContactRequests {
    val listQueries: MutableList<RuntimeHostContactQuery> = mutableListOf()
    val searchQueries: MutableList<Pair<String, RuntimeHostContactQuery>> = mutableListOf()
    val readIdentifiers: MutableList<String> = mutableListOf()
    val createDrafts: MutableList<RuntimeHostContactDraft> = mutableListOf()
    val updateCalls: MutableList<Pair<String, RuntimeHostContactDraft>> = mutableListOf()
    val deleteIdentifiers: MutableList<String> = mutableListOf()
    var listResponse: RuntimeHostContactPageResponse = RuntimeHostContactPageResponse(status = 0)
    var searchResponse: RuntimeHostContactPageResponse = RuntimeHostContactPageResponse(status = 0)
    var readResponse: RuntimeHostContactResponse = RuntimeHostContactResponse(status = 0)
    var createResponse: RuntimeHostContactCreateResponse =
        RuntimeHostContactCreateResponse(status = 0)
    var updateStatus: Int = 0
    var deleteStatus: Int = 0

    override fun list(
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        listQueries += query

        return listResponse
    }

    override fun search(
        queryText: String,
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        searchQueries += queryText to query

        return searchResponse
    }

    override fun read(
        id: String,
    ): RuntimeHostContactResponse {
        readIdentifiers += id

        return readResponse
    }

    override fun create(
        draft: RuntimeHostContactDraft,
    ): RuntimeHostContactCreateResponse {
        createDrafts += draft

        return createResponse
    }

    override fun update(
        id: String,
        draft: RuntimeHostContactDraft,
    ): Int {
        updateCalls += id to draft

        return updateStatus
    }

    override fun deleteContact(
        id: String,
    ): Int {
        deleteIdentifiers += id

        return deleteStatus
    }
}

/**
 * Record intent requests submitted through one Android runtime host.
 */
internal class RecordingIntentRequestHandler : IntentRequests {
    val canOpenUrlCalls: MutableList<String> = mutableListOf()
    val openUrlCalls: MutableList<String> = mutableListOf()
    val openPathCalls: MutableList<String> = mutableListOf()
    val shareTextCalls: MutableList<Pair<String, String?>> = mutableListOf()
    val sharePathCalls: MutableList<Pair<List<String>, String?>> = mutableListOf()
    var isOpenUrlSupported: Boolean = false
    var status: Int = 0

    override fun canOpenUrl(
        url: String,
    ): Boolean {
        canOpenUrlCalls += url

        return isOpenUrlSupported
    }

    override fun openUrl(
        url: String,
    ): Int {
        openUrlCalls += url

        return status
    }

    override fun openPath(
        path: String,
    ): Int {
        openPathCalls += path

        return status
    }

    override fun shareText(
        text: String,
        mimeType: String?,
    ): Int {
        shareTextCalls += text to mimeType

        return status
    }

    override fun sharePaths(
        paths: List<String>,
        mimeType: String?,
    ): Int {
        sharePathCalls += paths to mimeType

        return status
    }
}

/**
 * Record intent events submitted through one Android runtime host.
 */
internal class RecordingIntentEventSink : IntentEvents {
    val events: MutableList<RuntimeHostIntentEvent> = mutableListOf()

    override fun notifyIntentEvent(
        event: RuntimeHostIntentEvent,
    ) {
        events += event
    }
}

/**
 * Record location requests submitted through one Android runtime host.
 */
internal class RecordingLocationRequestHandler : LocationRequests {
    var servicesEnabledResponse: RuntimeHostLocationServicesResponse =
        RuntimeHostLocationServicesResponse(status = 0)
    var lastKnownResponse: RuntimeHostLocationLastKnownResponse =
        RuntimeHostLocationLastKnownResponse(status = 0)
    val watchOpenCalls: MutableList<Pair<String, RuntimeHostLocationWatchOptions>> = mutableListOf()
    val watchCloseCalls: MutableList<String> = mutableListOf()
    var watchOpenStatus: Int = 0
    var watchCloseStatus: Int = 0

    override fun servicesEnabled(): RuntimeHostLocationServicesResponse {
        return servicesEnabledResponse
    }

    override fun lastKnown(): RuntimeHostLocationLastKnownResponse {
        return lastKnownResponse
    }

    override fun watchOpen(
        watchId: String,
        options: RuntimeHostLocationWatchOptions,
    ): Int {
        watchOpenCalls += watchId to options

        return watchOpenStatus
    }

    override fun watchClose(
        watchId: String,
    ): Int {
        watchCloseCalls += watchId

        return watchCloseStatus
    }
}

/**
 * Record location samples sent through one Android runtime host.
 */
internal class RecordingLocationEventSink : LocationEvents {
    val samples: MutableList<Pair<String, RuntimeHostLocationSample>> = mutableListOf()

    override fun notifyLocationSample(
        watchId: String,
        sample: RuntimeHostLocationSample,
    ) {
        samples += watchId to sample
    }
}

/**
 * Record media requests submitted through one Android runtime host.
 */
internal class RecordingMediaRequestHandler : MediaRequests {
    val listRequests: MutableList<RuntimeHostMediaListRequest> = mutableListOf()
    val readIdentifiers: MutableList<String> = mutableListOf()
    val importRequests: MutableList<RuntimeHostMediaImportPathRequest> = mutableListOf()
    val deleteRequests: MutableList<RuntimeHostMediaDeleteRequest> = mutableListOf()
    var listResponse: RuntimeHostMediaListResponse = RuntimeHostMediaListResponse(status = 0)
    var readResponse: RuntimeHostMediaReadResponse = RuntimeHostMediaReadResponse(status = 0)
    var importResponse: RuntimeHostMediaImportPathResponse =
        RuntimeHostMediaImportPathResponse(status = 0)
    var deleteResponse: RuntimeHostMediaDeleteResponse =
        RuntimeHostMediaDeleteResponse(status = 0)

    override fun list(
        request: RuntimeHostMediaListRequest,
    ): RuntimeHostMediaListResponse {
        listRequests += request

        return listResponse
    }

    override fun read(
        identifier: String,
    ): RuntimeHostMediaReadResponse {
        readIdentifiers += identifier

        return readResponse
    }

    override fun importPath(
        request: RuntimeHostMediaImportPathRequest,
    ): RuntimeHostMediaImportPathResponse {
        importRequests += request

        return importResponse
    }

    override fun delete(
        request: RuntimeHostMediaDeleteRequest,
    ): RuntimeHostMediaDeleteResponse {
        deleteRequests += request

        return deleteResponse
    }
}

/**
 * Record notification requests submitted through one Android runtime host.
 */
internal class RecordingNotificationRequestHandler : NotificationRequests {
    val postedRequests: MutableList<RuntimeHostNotificationRequest> = mutableListOf()
    val cancelledIdentifiers: MutableList<String> = mutableListOf()
    var cancelAllCalls: Int = 0
    var status: Int = 0

    override fun post(
        request: RuntimeHostNotificationRequest,
    ): Int {
        postedRequests += request

        return status
    }

    override fun cancel(
        identifier: String,
    ): Int {
        cancelledIdentifiers += identifier

        return status
    }

    override fun cancelAll(): Int {
        cancelAllCalls += 1

        return status
    }
}

/**
 * Record notification events sent through one Android runtime host.
 */
internal class RecordingNotificationEventSink : NotificationEvents {
    val events: MutableList<RuntimeHostNotificationEvent> = mutableListOf()

    override fun notifyNotificationEvent(
        event: RuntimeHostNotificationEvent,
    ) {
        events += event
    }
}

/**
 * Create one Android runtime host fixture.
 */
internal fun createRuntimeHost(
    lifecycleEvents: LifecycleEvents = RecordingLifecycleSink(),
    permissionRequests: PermissionRequests = RecordingPermissionRequestHandler(),
    permissionEvents: PermissionEvents = RecordingPermissionEventSink(),
    documentRequests: DocumentRequests = RecordingDocumentRequestHandler(),
    documentEvents: DocumentEvents = RecordingDocumentEventSink(),
    contactRequests: ContactRequests = UnsupportedContactRequests,
    calendarRequests: CalendarRequests = UnsupportedCalendarRequests,
    intentRequests: IntentRequests = RecordingIntentRequestHandler(),
    intentEvents: IntentEvents = RecordingIntentEventSink(),
    locationRequests: LocationRequests = UnsupportedLocationRequests,
    locationEvents: LocationEvents = RecordingLocationEventSink(),
    mediaRequests: MediaRequests = UnsupportedMediaRequests,
    notificationRequests: NotificationRequests = RecordingNotificationRequestHandler(),
    notificationEvents: NotificationEvents = RecordingNotificationEventSink(),
    surfaceKind: RendererSurfaceKind = RendererSurfaceKind.SurfaceView,
    rendererSurfaceIdentifier: String = "main-surface",
): RuntimeHost {
    return RuntimeHost(
        sessionHandle = HostSessionHandle(rawValue = 7),
        embedderId = HostEmbedderId(rawValue = 11),
        lifecycle = LifecycleHost(lifecycleEvents),
        permission = PermissionHost(permissionRequests, permissionEvents),
        document = DocumentHost(documentRequests, documentEvents),
        contact = ContactHost(contactRequests),
        calendar = CalendarHost(calendarRequests),
        intent = IntentHost(intentRequests, intentEvents),
        location = LocationHost(locationRequests, locationEvents),
        media = MediaHost(mediaRequests),
        notification = NotificationHost(notificationRequests, notificationEvents),
        rendererSurface = RendererSurface(
            kind = surfaceKind,
            identifier = rendererSurfaceIdentifier,
        ),
    )
}
