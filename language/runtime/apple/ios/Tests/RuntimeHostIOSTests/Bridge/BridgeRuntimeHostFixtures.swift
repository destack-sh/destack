import RuntimeHostAppleCore

@testable import RuntimeHostIOS

/// One bridge permission request recorder for iOS bridge tests.
@MainActor
final class PermissionRequestRecorder: PermissionRequests {
  var requests: [RuntimeHostPermissionRequest] = []
  var openSettingsCalls: Int = 0
  var openSettingsStatus: UInt32 = hostStatusOk

  func request(_ request: RuntimeHostPermissionRequest) -> UInt32 {
    requests.append(request)

    return hostStatusOk
  }

  func openSettings() -> UInt32 {
    openSettingsCalls += 1

    return openSettingsStatus
  }
}

/// One bridge document request recorder for iOS bridge tests.
@MainActor
final class DocumentRequestRecorder: DocumentRequests {
  var requests: [RuntimeHostDocumentRequest] = []

  func pick(_ request: RuntimeHostDocumentRequest) -> UInt32 {
    requests.append(request)

    return hostStatusOk
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

  func list(
    _ query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    listQueries.append(query)

    return listResponse
  }

  func search(
    _ queryText: String,
    query: RuntimeHostContactQuery
  ) -> RuntimeHostContactPageResponse {
    searchQueries.append((queryText, query))

    return searchResponse
  }

  func read(
    _ id: String
  ) -> RuntimeHostContactResponse {
    readIdentifiers.append(id)

    return readResponse
  }

  func create(
    _ draft: RuntimeHostContactDraft
  ) -> RuntimeHostContactCreateResponse {
    createDrafts.append(draft)

    return createResponse
  }

  func update(
    _ id: String,
    draft: RuntimeHostContactDraft
  ) -> UInt32 {
    updateCalls.append((id, draft))

    return updateStatus
  }

  func deleteContact(
    _ id: String
  ) -> UInt32 {
    deleteIdentifiers.append(id)

    return deleteStatus
  }
}

/// One bridge calendar request recorder for iOS bridge tests.
@MainActor
final class CalendarRequestRecorder: CalendarRequests {
  var listResponse = RuntimeHostCalendarListResponse(status: hostStatusNotSupported)
  var eventQueries: [RuntimeHostCalendarEventQuery] = []
  var eventListResponse = RuntimeHostCalendarEventListResponse(status: hostStatusNotSupported)
  var readIdentifiers: [String] = []
  var readResponse = RuntimeHostCalendarEventReadResponse(status: hostStatusNotSupported)
  var createDrafts: [RuntimeHostCalendarEventDraft] = []
  var createResponse = RuntimeHostCalendarEventCreateResponse(status: hostStatusNotSupported)
  var updateCalls: [(String, RuntimeHostCalendarEventDraft)] = []
  var updateStatus: UInt32 = hostStatusNotSupported
  var deleteIdentifiers: [String] = []
  var deleteStatus: UInt32 = hostStatusNotSupported

  func list() -> RuntimeHostCalendarListResponse {
    listResponse
  }

  func eventList(
    _ query: RuntimeHostCalendarEventQuery
  ) -> RuntimeHostCalendarEventListResponse {
    eventQueries.append(query)

    return eventListResponse
  }

  func eventRead(
    _ id: String
  ) -> RuntimeHostCalendarEventReadResponse {
    readIdentifiers.append(id)

    return readResponse
  }

  func eventCreate(
    _ draft: RuntimeHostCalendarEventDraft
  ) -> RuntimeHostCalendarEventCreateResponse {
    createDrafts.append(draft)

    return createResponse
  }

  func eventUpdate(
    _ id: String,
    draft: RuntimeHostCalendarEventDraft
  ) -> UInt32 {
    updateCalls.append((id, draft))

    return updateStatus
  }

  func eventDelete(
    _ id: String
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
    mimeType: String?
  ) -> UInt32 {
    shareTextCalls.append((text, mimeType))

    return status
  }

  func sharePaths(
    _ paths: [String],
    mimeType: String?
  ) -> UInt32 {
    sharePathCalls.append((paths, mimeType))

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

  func servicesEnabled() -> RuntimeHostLocationServicesResponse {
    servicesEnabledResponse
  }

  func lastKnown() -> RuntimeHostLocationLastKnownResponse {
    lastKnownResponse
  }

  func watchOpen(
    _ watchID: String,
    options: RuntimeHostLocationWatchOptions
  ) -> UInt32 {
    watchOpenCalls.append((watchID, options))

    return watchOpenStatus
  }

  func watchClose(
    _ watchID: String
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

  func list(
    _ request: RuntimeHostMediaListRequest
  ) -> RuntimeHostMediaListResponse {
    listRequests.append(request)

    return listResponse
  }

  func read(
    _ identifier: String
  ) -> RuntimeHostMediaReadResponse {
    readIdentifiers.append(identifier)

    return readResponse
  }

  func importPath(
    _ request: RuntimeHostMediaImportPathRequest
  ) -> RuntimeHostMediaImportPathResponse {
    importRequests.append(request)

    return importResponse
  }

  func delete(
    _ request: RuntimeHostMediaDeleteRequest
  ) -> RuntimeHostMediaDeleteResponse {
    deleteRequests.append(request)

    return deleteResponse
  }
}

/// One bridge background request recorder for iOS bridge tests.
@MainActor
final class BackgroundRequestRecorder: BackgroundRequests {
  var statusResponse = RuntimeHostBackgroundStatusResponse(
    status: hostStatusOk,
    schedulerStatus: .available
  )
  var listResponse = RuntimeHostBackgroundListResponse(status: hostStatusOk)
  var registerCalls: [RuntimeHostBackgroundTaskOptions] = []
  var unregisterCalls: [String] = []
  var triggerCalls: [String] = []
  var completeCalls: [(String, RuntimeHostBackgroundTaskResult)] = []
  var registerStatus: UInt32 = hostStatusOk
  var unregisterStatus: UInt32 = hostStatusOk
  var triggerResponse = RuntimeHostBackgroundTriggerTestResponse(
    status: hostStatusOk,
    isTriggered: true
  )
  var completeStatus: UInt32 = hostStatusOk

  func status() -> RuntimeHostBackgroundStatusResponse {
    statusResponse
  }

  func list() -> RuntimeHostBackgroundListResponse {
    listResponse
  }

  func registerTask(
    _ request: RuntimeHostBackgroundTaskOptions
  ) -> UInt32 {
    registerCalls.append(request)

    return registerStatus
  }

  func unregister(
    _ request: RuntimeHostBackgroundUnregisterRequest
  ) -> UInt32 {
    unregisterCalls.append(request.identifier)

    return unregisterStatus
  }

  func triggerTest(
    _ request: RuntimeHostBackgroundTriggerTestRequest
  ) -> RuntimeHostBackgroundTriggerTestResponse {
    triggerCalls.append(request.identifier)

    return triggerResponse
  }

  func complete(
    _ request: RuntimeHostBackgroundCompleteRequest
  ) -> UInt32 {
    completeCalls.append((request.executionID, request.result))

    return completeStatus
  }
}

/// One bridge notification request recorder for iOS bridge tests.
@MainActor
final class NotificationRequestRecorder: NotificationRequests {
  var postedRequests: [RuntimeHostNotificationRequest] = []
  var cancelledIdentifiers: [String] = []
  var cancelAllCalls: Int = 0
  var status: UInt32 = hostStatusOk

  func post(
    _ request: RuntimeHostNotificationRequest
  ) -> UInt32 {
    postedRequests.append(request)

    return status
  }

  func cancel(
    _ identifier: String
  ) -> UInt32 {
    cancelledIdentifiers.append(identifier)

    return status
  }

  func cancelAll() -> UInt32 {
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
  backgroundRequests: any BackgroundRequests = UnsupportedBackgroundRequests(),
  contactRequests: any ContactRequests = ContactRequestRecorder(),
  calendarRequests: any CalendarRequests = CalendarRequestRecorder(),
  intentRequests: any IntentRequests = IntentRequestRecorder(),
  locationRequests: any LocationRequests = LocationRequestRecorder(),
  mediaRequests: any MediaRequests = MediaRequestRecorder(),
  notificationRequests: any NotificationRequests = NotificationRequestRecorder()
) -> RuntimeHost {
  let runtimeHost = RuntimeHost(
    sessionHandle: sessionHandle,
    embedderID: makeTestEmbedderID(),
    lifecycle: LifecycleHost(events: IOSRecordingLifecycleSink()),
    permission: PermissionHost(
      requests: permissionRequests,
      events: IOSRecordingPermissionEventSink()
    ),
    document: DocumentHost(
      requests: documentRequests,
      events: IOSRecordingDocumentEventSink()
    ),
    contact: ContactHost(requests: contactRequests),
    calendar: CalendarHost(requests: calendarRequests),
    intent: IntentHost(
      requests: intentRequests,
      events: IOSRecordingIntentEventSink()
    ),
    location: LocationHost(
      requests: locationRequests,
      events: IOSNoopLocationEventSink()
    ),
    media: MediaHost(requests: mediaRequests),
    notification: NotificationHost(
      requests: notificationRequests,
      events: IOSRecordingNotificationEventSink()
    ),
    rendererSurface: RendererSurface(
      kind: .metalLayer,
      identifier: "main-surface"
    )
  )

  runtimeHost.updateBackgroundRequests(backgroundRequests)

  return runtimeHost
}
