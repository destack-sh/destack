import RuntimeHostAppleCore

@testable import RuntimeHostIOS

/// One bridge permission request recorder for iOS bridge tests.
@MainActor
final class PermissionRequestRecorder: PermissionRequests {
    var requests: [RuntimeHostPermissionRequest] = []
    var openSettingsCalls: Int = 0
    var openSettingsStatus: UInt32 = hostStatusOk

    func submitPermissionRequest(_ request: RuntimeHostPermissionRequest) {
        requests.append(request)
    }

    func openPermissionSettings() -> UInt32 {
        openSettingsCalls += 1

        return openSettingsStatus
    }
}

/// One bridge document request recorder for iOS bridge tests.
@MainActor
final class DocumentRequestRecorder: DocumentRequests {
    var requests: [RuntimeHostDocumentRequest] = []

    func submitDocumentRequest(_ request: RuntimeHostDocumentRequest) {
        requests.append(request)
    }
}

/// One bridge contact request recorder for iOS bridge tests.
@MainActor
final class ContactRequestRecorder: ContactRequests {
    var listQueries: [RuntimeHostContactQuery] = []
    var searchQueries: [(String, RuntimeHostContactQuery)] = []
    var readIdentifiers: [String] = []
    var createDrafts: [RuntimeHostContactDraft] = []
    var updateCalls: [(String, RuntimeHostContactDraft)] = []
    var deleteIdentifiers: [String] = []
    var listResponse = RuntimeHostContactPageResponse(status: hostStatusNotSupported)
    var searchResponse = RuntimeHostContactPageResponse(status: hostStatusNotSupported)
    var readResponse = RuntimeHostContactResponse(status: hostStatusNotSupported)
    var createResponse = RuntimeHostContactCreateResponse(status: hostStatusNotSupported)
    var updateStatus: UInt32 = hostStatusNotSupported
    var deleteStatus: UInt32 = hostStatusNotSupported

    func listContacts(
        _ query: RuntimeHostContactQuery
    ) -> RuntimeHostContactPageResponse {
        listQueries.append(query)

        return listResponse
    }

    func searchContacts(
        _ queryText: String,
        query: RuntimeHostContactQuery
    ) -> RuntimeHostContactPageResponse {
        searchQueries.append((queryText, query))

        return searchResponse
    }

    func readContact(
        id: String
    ) -> RuntimeHostContactResponse {
        readIdentifiers.append(id)

        return readResponse
    }

    func createContact(
        _ draft: RuntimeHostContactDraft
    ) -> RuntimeHostContactCreateResponse {
        createDrafts.append(draft)

        return createResponse
    }

    func updateContact(
        id: String,
        draft: RuntimeHostContactDraft
    ) -> UInt32 {
        updateCalls.append((id, draft))

        return updateStatus
    }

    func deleteContact(
        id: String
    ) -> UInt32 {
        deleteIdentifiers.append(id)

        return deleteStatus
    }
}

/// One bridge calendar request recorder for iOS bridge tests.
@MainActor
final class CalendarRequestRecorder: CalendarRequests {
    var listResponse: (status: UInt32, calendars: [RuntimeHostCalendarDescriptor]?) =
      (hostStatusNotSupported, nil)
    var eventQueries: [RuntimeHostCalendarEventQuery] = []
    var eventListResponse: (status: UInt32, events: [RuntimeHostCalendarEvent]?) =
      (hostStatusNotSupported, nil)
    var readIdentifiers: [String] = []
    var readResponse: (status: UInt32, event: RuntimeHostCalendarEvent?) =
      (hostStatusNotSupported, nil)
    var createDrafts: [RuntimeHostCalendarEventDraft] = []
    var createResponse: (status: UInt32, id: String?) = (hostStatusNotSupported, nil)
    var updateCalls: [(String, RuntimeHostCalendarEventDraft)] = []
    var updateStatus: UInt32 = hostStatusNotSupported
    var deleteIdentifiers: [String] = []
    var deleteStatus: UInt32 = hostStatusNotSupported

    func listCalendars() -> (status: UInt32, calendars: [RuntimeHostCalendarDescriptor]?) {
        listResponse
    }

    func listCalendarEvents(
        _ query: RuntimeHostCalendarEventQuery
    ) -> (status: UInt32, events: [RuntimeHostCalendarEvent]?) {
        eventQueries.append(query)

        return eventListResponse
    }

    func readCalendarEvent(
        id: String
    ) -> (status: UInt32, event: RuntimeHostCalendarEvent?) {
        readIdentifiers.append(id)

        return readResponse
    }

    func createCalendarEvent(
        _ draft: RuntimeHostCalendarEventDraft
    ) -> (status: UInt32, id: String?) {
        createDrafts.append(draft)

        return createResponse
    }

    func updateCalendarEvent(
        id: String,
        draft: RuntimeHostCalendarEventDraft
    ) -> UInt32 {
        updateCalls.append((id, draft))

        return updateStatus
    }

    func deleteCalendarEvent(
        id: String
    ) -> UInt32 {
        deleteIdentifiers.append(id)

        return deleteStatus
    }
}

/// One bridge intent request recorder for iOS bridge tests.
@MainActor
final class IntentRequestRecorder: IntentRequests {
    var canOpenURLCalls: [String] = []
    var openURLCalls: [String] = []
    var openPathCalls: [String] = []
    var shareTextCalls: [(String, String?)] = []
    var sharePathCalls: [([String], String?)] = []
    var isOpenURLSupported: Bool = false
    var status: UInt32 = hostStatusOk

    func canOpenURL(_ url: String) -> Bool {
        canOpenURLCalls.append(url)

        return isOpenURLSupported
    }

    func openURL(_ url: String) -> UInt32 {
        openURLCalls.append(url)

        return status
    }

    func openPath(_ path: String) -> UInt32 {
        openPathCalls.append(path)

        return status
    }

    func shareText(
        _ text: String,
        contentType: String?
    ) -> UInt32 {
        shareTextCalls.append((text, contentType))

        return status
    }

    func sharePaths(
        _ paths: [String],
        contentType: String?
    ) -> UInt32 {
        sharePathCalls.append((paths, contentType))

        return status
    }
}

/// One bridge location request recorder for iOS bridge tests.
@MainActor
final class LocationRequestRecorder: LocationRequests {
    var servicesEnabledResponse = RuntimeHostLocationServicesResponse(status: hostStatusOk)
    var lastKnownResponse = RuntimeHostLocationLastKnownResponse(status: hostStatusOk)
    var watchOpenCalls: [(String, RuntimeHostLocationWatchOptions)] = []
    var watchCloseCalls: [String] = []
    var watchOpenStatus: UInt32 = hostStatusOk
    var watchCloseStatus: UInt32 = hostStatusOk

    func locationServicesEnabled() -> RuntimeHostLocationServicesResponse {
        servicesEnabledResponse
    }

    func locationLastKnown() -> RuntimeHostLocationLastKnownResponse {
        lastKnownResponse
    }

    func locationWatchOpen(
        watchID: String,
        options: RuntimeHostLocationWatchOptions
    ) -> UInt32 {
        watchOpenCalls.append((watchID, options))

        return watchOpenStatus
    }

    func locationWatchClose(
        watchID: String
    ) -> UInt32 {
        watchCloseCalls.append(watchID)

        return watchCloseStatus
    }
}

/// One bridge media request recorder for iOS bridge tests.
@MainActor
final class MediaRequestRecorder: MediaRequests {
    var listRequests: [RuntimeHostMediaListRequest] = []
    var readIdentifiers: [String] = []
    var importRequests: [RuntimeHostMediaImportPathRequest] = []
    var deleteRequests: [RuntimeHostMediaDeleteRequest] = []
    var listResponse = RuntimeHostMediaListResponse(status: hostStatusNotSupported)
    var readResponse = RuntimeHostMediaReadResponse(status: hostStatusNotSupported)
    var importResponse = RuntimeHostMediaImportPathResponse(status: hostStatusNotSupported)
    var deleteResponse = RuntimeHostMediaDeleteResponse(status: hostStatusNotSupported)

    func listMedia(
        _ request: RuntimeHostMediaListRequest
    ) -> RuntimeHostMediaListResponse {
        listRequests.append(request)

        return listResponse
    }

    func readMedia(
        identifier: String
    ) -> RuntimeHostMediaReadResponse {
        readIdentifiers.append(identifier)

        return readResponse
    }

    func importMediaPath(
        _ request: RuntimeHostMediaImportPathRequest
    ) -> RuntimeHostMediaImportPathResponse {
        importRequests.append(request)

        return importResponse
    }

    func deleteMedia(
        _ request: RuntimeHostMediaDeleteRequest
    ) -> RuntimeHostMediaDeleteResponse {
        deleteRequests.append(request)

        return deleteResponse
    }
}

/// One bridge notification request recorder for iOS bridge tests.
@MainActor
final class NotificationRequestRecorder: NotificationRequests {
    var postedRequests: [RuntimeHostNotificationRequest] = []
    var cancelledIdentifiers: [String] = []
    var cancelAllCalls: Int = 0
    var status: UInt32 = hostStatusOk

    func postNotification(
        _ request: RuntimeHostNotificationRequest
    ) -> UInt32 {
        postedRequests.append(request)

        return status
    }

    func cancelNotification(
        identifier: String
    ) -> UInt32 {
        cancelledIdentifiers.append(identifier)

        return status
    }

    func cancelAllNotifications() -> UInt32 {
        cancelAllCalls += 1

        return status
    }
}

/// Create one bridge runtime host fixture.
@MainActor
func createBridgeRuntimeHost(
    sessionHandle: HostSessionHandle,
    permissionRequests: any PermissionRequests = PermissionRequestRecorder(),
    documentRequests: any DocumentRequests = DocumentRequestRecorder(),
    contactRequests: any ContactRequests = ContactRequestRecorder(),
    calendarRequests: any CalendarRequests = CalendarRequestRecorder(),
    intentRequests: any IntentRequests = IntentRequestRecorder(),
    locationRequests: any LocationRequests = LocationRequestRecorder(),
    mediaRequests: any MediaRequests = MediaRequestRecorder(),
    notificationRequests: any NotificationRequests = NotificationRequestRecorder()
) -> RuntimeHost {
    RuntimeHost(
        sessionHandle: sessionHandle,
        embedderID: makeTestEmbedderID(),
        lifecycleEvents: IOSRecordingLifecycleSink(),
        permissionRequests: permissionRequests,
        permissionEvents: IOSRecordingPermissionEventSink(),
        documentRequests: documentRequests,
        documentEvents: IOSRecordingDocumentEventSink(),
        contactRequests: contactRequests,
        calendarRequests: calendarRequests,
        intentRequests: intentRequests,
        intentEvents: IOSRecordingIntentEventSink(),
        locationRequests: locationRequests,
        locationEvents: IOSNoopLocationEventSink(),
        mediaRequests: mediaRequests,
        notificationRequests: notificationRequests,
        notificationEvents: IOSRecordingNotificationEventSink(),
        rendererSurface: RendererSurface(
            kind: .metalLayer,
            identifier: "main-surface"
        )
    )
}
