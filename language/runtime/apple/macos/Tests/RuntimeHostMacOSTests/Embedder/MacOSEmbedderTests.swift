import RuntimeHostAppleCore
import RuntimeHostMacOS
import Testing

@MainActor
@Test
func testApplicationEmbedderSmokePath() {
  let lifecycleEvents = MacOSRecordingLifecycleSink()
  let permissionRequests = MacOSRecordingPermissionRequestHandler()
  let permissionEvents = MacOSRecordingPermissionEventSink()
  let documentRequests = MacOSRecordingDocumentRequestHandler()
  let documentEvents = MacOSRecordingDocumentEventSink()
  let contactRequests = MacOSRecordingContactRequestHandler()
  let runtimeHost = createMacOSRuntimeHost(
    lifecycleEvents: lifecycleEvents,
    permissionRequests: permissionRequests,
    permissionEvents: permissionEvents,
    documentRequests: documentRequests,
    documentEvents: documentEvents,
    contactRequests: contactRequests
  )
  let applicationEmbedder = ApplicationEmbedder(runtimeHost: runtimeHost)
  let permissionRequest = RuntimeHostPermissionRequest(
    requestID: HostRequestID(rawValue: 1),
    permission: "camera"
  )
  let permissionEvent = RuntimeHostPermissionEvent(
    requestID: HostRequestID(rawValue: 1),
    permission: "camera",
    isGranted: true
  )
  let documentRequest = RuntimeHostDocumentRequest(
    requestID: HostRequestID(rawValue: 2),
    allowsMultipleSelection: false,
    contentTypes: ["public.image"]
  )
  let documentResult = RuntimeHostDocumentResult(
    requestID: HostRequestID(rawValue: 2),
    documents: []
  )

  applicationEmbedder.applicationDelegate.applicationDidFinishLaunching()
  applicationEmbedder.applicationDelegate.applicationDidBecomeActive()
  runtimeHost.permission.request(permissionRequest)
  runtimeHost.permission.notifyPermissionResult(permissionEvent)
  runtimeHost.document.pick(documentRequest)
  runtimeHost.document.notifyDocumentResult(
    documentResult.requestID,
    documents: documentResult.documents
  )
  _ = runtimeHost.contact.list(
    RuntimeHostContactQuery(includeEmails: true)
  )

  #expect(
    lifecycleEvents.events == [
      macOSApplicationLifecycleEvent(.initializing),
      macOSApplicationLifecycleEvent(.running),
    ])
  #expect(permissionRequests.requests == [permissionRequest])
  #expect(permissionEvents.events == [permissionEvent])
  #expect(documentRequests.requests == [documentRequest])
  #expect(documentEvents.results == [documentResult])
  #expect(
    contactRequests.listQueries == [
      RuntimeHostContactQuery(includeEmails: true)
    ])
}
