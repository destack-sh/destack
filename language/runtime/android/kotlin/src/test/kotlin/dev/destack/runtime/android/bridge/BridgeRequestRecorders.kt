package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.module.calendar.CalendarRequests
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventCreateResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventDraft
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventListResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventQuery
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarListResponse
import dev.destack.runtime.android.module.contact.ContactRequests
import dev.destack.runtime.android.module.contact.RuntimeHostContactCreateResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactDraft
import dev.destack.runtime.android.module.contact.RuntimeHostContactPageResponse
import dev.destack.runtime.android.module.contact.RuntimeHostContactQuery
import dev.destack.runtime.android.module.contact.RuntimeHostContactResponse
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.intent.IntentRequests
import dev.destack.runtime.android.module.location.LocationRequests
import dev.destack.runtime.android.module.location.RuntimeHostLocationLastKnownResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationServicesResponse
import dev.destack.runtime.android.module.location.RuntimeHostLocationWatchOptions
import dev.destack.runtime.android.module.media.MediaRequests
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaListRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaListResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaReadResponse
import dev.destack.runtime.android.module.notification.NotificationRequests
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest
import dev.destack.runtime.android.module.permission.PermissionRequests
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

/**
 * Return whether one live runtime bridge library path is available for tests.
 */
fun hasRuntimeBridgeLibrary(): Boolean {
    return !System.getenv("DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY").isNullOrBlank()
}

/**
 * Record permission requests for bridge tests.
 */
class PermissionRequestRecorder : PermissionRequests {
    val requests: MutableList<RuntimeHostPermissionRequest> = mutableListOf()
    var openSettingsCalls: Int = 0
    var openSettingsStatus: Int = 0

    override fun submitPermissionRequest(
        request: RuntimeHostPermissionRequest,
    ) {
        requests += request
    }

    override fun openPermissionSettings(): Int {
        openSettingsCalls += 1

        return openSettingsStatus
    }
}

/**
 * Record document requests for bridge tests.
 */
class DocumentRequestRecorder : DocumentRequests {
    val requests: MutableList<RuntimeHostDocumentRequest> = mutableListOf()

    override fun submitDocumentRequest(
        request: RuntimeHostDocumentRequest,
    ) {
        requests += request
    }
}

/**
 * Record calendar requests for bridge tests.
 */
class CalendarRequestRecorder : CalendarRequests {
    var listResponse: RuntimeHostCalendarListResponse =
        RuntimeHostCalendarListResponse(status = 1)
    val eventQueries: MutableList<RuntimeHostCalendarEventQuery> = mutableListOf()
    var eventListResponse: RuntimeHostCalendarEventListResponse =
        RuntimeHostCalendarEventListResponse(status = 1)
    val readIdentifiers: MutableList<String> = mutableListOf()
    var readResponse: RuntimeHostCalendarEventResponse =
        RuntimeHostCalendarEventResponse(status = 1)
    val createDrafts: MutableList<RuntimeHostCalendarEventDraft> = mutableListOf()
    var createResponse: RuntimeHostCalendarEventCreateResponse =
        RuntimeHostCalendarEventCreateResponse(status = 1)
    val updateCalls: MutableList<Pair<String, RuntimeHostCalendarEventDraft>> = mutableListOf()
    var updateStatus: Int = 1
    val deleteIdentifiers: MutableList<String> = mutableListOf()
    var deleteStatus: Int = 1

    override fun listCalendars(): RuntimeHostCalendarListResponse {
        return listResponse
    }

    override fun listCalendarEvents(
        query: RuntimeHostCalendarEventQuery,
    ): RuntimeHostCalendarEventListResponse {
        eventQueries += query

        return eventListResponse
    }

    override fun readCalendarEvent(
        id: String,
    ): RuntimeHostCalendarEventResponse {
        readIdentifiers += id

        return readResponse
    }

    override fun createCalendarEvent(
        draft: RuntimeHostCalendarEventDraft,
    ): RuntimeHostCalendarEventCreateResponse {
        createDrafts += draft

        return createResponse
    }

    override fun updateCalendarEvent(
        id: String,
        draft: RuntimeHostCalendarEventDraft,
    ): Int {
        updateCalls += id to draft

        return updateStatus
    }

    override fun deleteCalendarEvent(
        id: String,
    ): Int {
        deleteIdentifiers += id

        return deleteStatus
    }
}

/**
 * Record contact requests for bridge tests.
 */
class ContactRequestRecorder : ContactRequests {
    val listQueries: MutableList<RuntimeHostContactQuery> = mutableListOf()
    val searchQueries: MutableList<Pair<String, RuntimeHostContactQuery>> = mutableListOf()
    val readIdentifiers: MutableList<String> = mutableListOf()
    val createDrafts: MutableList<RuntimeHostContactDraft> = mutableListOf()
    val updateCalls: MutableList<Pair<String, RuntimeHostContactDraft>> = mutableListOf()
    val deleteIdentifiers: MutableList<String> = mutableListOf()
    var listResponse: RuntimeHostContactPageResponse = RuntimeHostContactPageResponse(status = 1)
    var searchResponse: RuntimeHostContactPageResponse =
        RuntimeHostContactPageResponse(status = 1)
    var readResponse: RuntimeHostContactResponse = RuntimeHostContactResponse(status = 1)
    var createResponse: RuntimeHostContactCreateResponse =
        RuntimeHostContactCreateResponse(status = 1)
    var updateStatus: Int = 1
    var deleteStatus: Int = 1

    override fun listContacts(
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        listQueries += query

        return listResponse
    }

    override fun searchContacts(
        queryText: String,
        query: RuntimeHostContactQuery,
    ): RuntimeHostContactPageResponse {
        searchQueries += queryText to query

        return searchResponse
    }

    override fun readContact(
        id: String,
    ): RuntimeHostContactResponse {
        readIdentifiers += id

        return readResponse
    }

    override fun createContact(
        draft: RuntimeHostContactDraft,
    ): RuntimeHostContactCreateResponse {
        createDrafts += draft

        return createResponse
    }

    override fun updateContact(
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
 * Record intent requests for bridge tests.
 */
class IntentRequestRecorder : IntentRequests {
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
        contentType: String?,
    ): Int {
        shareTextCalls += text to contentType

        return status
    }

    override fun sharePaths(
        paths: List<String>,
        contentType: String?,
    ): Int {
        sharePathCalls += paths to contentType

        return status
    }
}

/**
 * Record location requests for bridge tests.
 */
class LocationRequestRecorder : LocationRequests {
    var servicesEnabledResponse: RuntimeHostLocationServicesResponse =
        RuntimeHostLocationServicesResponse(status = 0)
    var lastKnownResponse: RuntimeHostLocationLastKnownResponse =
        RuntimeHostLocationLastKnownResponse(status = 0)
    val watchOpenCalls: MutableList<Pair<String, RuntimeHostLocationWatchOptions>> = mutableListOf()
    val watchCloseCalls: MutableList<String> = mutableListOf()
    var watchOpenStatus: Int = 0
    var watchCloseStatus: Int = 0

    override fun locationServicesEnabled(): RuntimeHostLocationServicesResponse {
        return servicesEnabledResponse
    }

    override fun locationLastKnown(): RuntimeHostLocationLastKnownResponse {
        return lastKnownResponse
    }

    override fun locationWatchOpen(
        watchId: String,
        options: RuntimeHostLocationWatchOptions,
    ): Int {
        watchOpenCalls += watchId to options

        return watchOpenStatus
    }

    override fun locationWatchClose(
        watchId: String,
    ): Int {
        watchCloseCalls += watchId

        return watchCloseStatus
    }
}

/**
 * Record media requests for bridge tests.
 */
class MediaRequestRecorder : MediaRequests {
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

    override fun listMedia(
        request: RuntimeHostMediaListRequest,
    ): RuntimeHostMediaListResponse {
        listRequests += request

        return listResponse
    }

    override fun readMedia(
        identifier: String,
    ): RuntimeHostMediaReadResponse {
        readIdentifiers += identifier

        return readResponse
    }

    override fun importMediaPath(
        request: RuntimeHostMediaImportPathRequest,
    ): RuntimeHostMediaImportPathResponse {
        importRequests += request

        return importResponse
    }

    override fun deleteMedia(
        request: RuntimeHostMediaDeleteRequest,
    ): RuntimeHostMediaDeleteResponse {
        deleteRequests += request

        return deleteResponse
    }
}

/**
 * Record notification requests for bridge tests.
 */
class NotificationRequestRecorder : NotificationRequests {
    val postedRequests: MutableList<RuntimeHostNotificationRequest> = mutableListOf()
    val cancelledIdentifiers: MutableList<String> = mutableListOf()
    var cancelAllCalls: Int = 0
    var status: Int = 0

    override fun postNotification(
        request: RuntimeHostNotificationRequest,
    ): Int {
        postedRequests += request

        return status
    }

    override fun cancelNotification(
        identifier: String,
    ): Int {
        cancelledIdentifiers += identifier

        return status
    }

    override fun cancelAllNotifications(): Int {
        cancelAllCalls += 1

        return status
    }
}
