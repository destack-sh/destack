import RuntimeHostAppleCore
import Testing

@MainActor
@Test
func testCreateHostWithPrimarySurface() {
  let sessionHandle = HostSessionHandle(rawValue: 7)
  let embedderID = HostEmbedderID(rawValue: 11)
  let lifecycleEvents = RecordingLifecycleSink()
  let permissionRequests = RecordingPermissionRequestHandler()
  let permissionEvents = RecordingPermissionEventSink()
  let documentRequests = RecordingDocumentRequestHandler()
  let documentEvents = RecordingDocumentEventSink()
  let contactRequests = UnsupportedContactRequests()
  let calendarRequests = UnsupportedCalendarRequests()
  let intentRequests = RecordingIntentRequestHandler()
  let intentEvents = RecordingIntentEventSink()
  let locationRequests = RecordingLocationRequestHandler()
  let locationEvents = RecordingLocationEventSink()
  let mediaRequests = RecordingMediaRequestHandler()
  let notificationRequests = RecordingNotificationRequestHandler()
  let notificationEvents = RecordingNotificationEventSink()
  let rendererSurface = RendererSurface(
    kind: .metalLayer,
    identifier: "main-surface"
  )

  let host = RuntimeHost(
    sessionHandle: sessionHandle,
    embedderID: embedderID,
    lifecycle: LifecycleHost(events: lifecycleEvents),
    permission: PermissionHost(
      requests: permissionRequests,
      events: permissionEvents
    ),
    document: DocumentHost(
      requests: documentRequests,
      events: documentEvents
    ),
    contact: ContactHost(requests: contactRequests),
    calendar: CalendarHost(requests: calendarRequests),
    intent: IntentHost(
      requests: intentRequests,
      events: intentEvents
    ),
    location: LocationHost(
      requests: locationRequests,
      events: locationEvents
    ),
    media: MediaHost(requests: mediaRequests),
    notification: NotificationHost(
      requests: notificationRequests,
      events: notificationEvents
    ),
    rendererSurface: rendererSurface
  )

  #expect(host.sessionHandle == sessionHandle)
  #expect(host.embedderID == embedderID)
  #expect(host.rendererSurface == rendererSurface)
}

@MainActor
@Test
func testSendLifecycleEvent() {
  let lifecycleEvents = RecordingLifecycleSink()
  let host = createRuntimeHost(lifecycleEvents: lifecycleEvents)
  let event = lifecycleEvent(.application, .running)

  host.lifecycle.sendLifecycleEvent(event)

  #expect(lifecycleEvents.events == [event])
}

@MainActor
@Test
func testSubmitPermissionRequest() {
  let permissionRequests = RecordingPermissionRequestHandler()
  let host = createRuntimeHost(
    permissionRequests: permissionRequests
  )
  let request = RuntimeHostPermissionRequest(
    requestID: HostRequestID(rawValue: 1),
    permission: "location"
  )

  host.permission.request(request)

  #expect(permissionRequests.requests == [request])
}

@MainActor
@Test
func testOpenPermissionSettings() {
  let permissionRequests = RecordingPermissionRequestHandler()
  let host = createRuntimeHost(
    permissionRequests: permissionRequests
  )

  let status = host.permission.openSettings()

  #expect(status == hostStatusOk)
  #expect(permissionRequests.openSettingsCalls == 1)
}

@MainActor
@Test
func testSendPermissionEvent() {
  let permissionEvents = RecordingPermissionEventSink()
  let host = createRuntimeHost(
    permissionEvents: permissionEvents
  )
  let event = RuntimeHostPermissionEvent(
    requestID: HostRequestID(rawValue: 2),
    permission: "location",
    isGranted: true
  )

  host.permission.notifyPermissionResult(event)

  #expect(permissionEvents.events == [event])
}

@MainActor
@Test
func testSubmitDocumentRequest() {
  let documentRequests = RecordingDocumentRequestHandler()
  let documentEvents = RecordingDocumentEventSink()
  let host = createRuntimeHost(
    documentRequests: documentRequests,
    documentEvents: documentEvents
  )
  let request = RuntimeHostDocumentRequest(
    requestID: HostRequestID(rawValue: 3),
    allowsMultipleSelection: true,
    contentTypes: ["image/png"]
  )
  let result = RuntimeHostDocumentResult(
    requestID: request.requestID,
    documents: [
      RuntimeHostDocumentDescriptor(
        uri: "file:///tmp/example.png",
        displayName: "example.png",
        contentType: "image/png",
        localPath: "/tmp/example.png"
      )
    ]
  )

  host.document.pick(request)
  host.document.notifyDocumentResult(
    result.requestID,
    documents: result.documents
  )

  #expect(documentRequests.requests == [request])
  #expect(documentEvents.results.first?.documents.first?.displayName == "example.png")
}
