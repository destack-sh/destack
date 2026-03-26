import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func
  testRuntimeBridgeRegistersDocumentPermissionBackgroundCalendarContactIntentLocationMediaAndNotificationLanes()
  throws
{
  let bindings = RuntimeAbiSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings,
    notificationTimestampNs: { 42 }
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    contactRequests: ContactRequestRecorder(),
    calendarRequests: CalendarRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )

  try bridge.attach(runtimeHost: runtimeHost)

  #expect(bindings.attachedSessionHandles == [sessionHandle])
  #expect(bindings.documentCallback != nil)
  #expect(bindings.permissionRequestCallback != nil)
  #expect(bindings.permissionOpenSettingsCallback != nil)
  #expect(bindings.backgroundStatusCallback != nil)
  #expect(bindings.backgroundListCallback != nil)
  #expect(bindings.backgroundRegisterCallback != nil)
  #expect(bindings.backgroundUnregisterCallback != nil)
  #expect(bindings.backgroundTriggerTestCallback != nil)
  #expect(bindings.backgroundCompleteCallback != nil)
  #expect(bindings.calendarListCallback != nil)
  #expect(bindings.calendarEventListCallback != nil)
  #expect(bindings.calendarEventReadCallback != nil)
  #expect(bindings.calendarEventCreateCallback != nil)
  #expect(bindings.calendarEventUpdateCallback != nil)
  #expect(bindings.calendarEventDeleteCallback != nil)
  #expect(bindings.contactListCallback != nil)
  #expect(bindings.contactSearchCallback != nil)
  #expect(bindings.contactReadCallback != nil)
  #expect(bindings.contactCreateCallback != nil)
  #expect(bindings.contactUpdateCallback != nil)
  #expect(bindings.contactDeleteCallback != nil)
  #expect(bindings.intentCanOpenURLCallback != nil)
  #expect(bindings.intentOpenURLCallback != nil)
  #expect(bindings.intentOpenPathCallback != nil)
  #expect(bindings.intentShareTextCallback != nil)
  #expect(bindings.intentSharePathsCallback != nil)
  #expect(bindings.locationServicesEnabledCallback != nil)
  #expect(bindings.locationLastKnownCallback != nil)
  #expect(bindings.locationWatchOpenCallback != nil)
  #expect(bindings.locationWatchCloseCallback != nil)
  #expect(bindings.mediaListCallback != nil)
  #expect(bindings.mediaReadCallback != nil)
  #expect(bindings.mediaImportPathCallback != nil)
  #expect(bindings.mediaDeleteCallback != nil)
  #expect(bindings.notificationPostCallback != nil)
  #expect(bindings.notificationCancelCallback != nil)
  #expect(bindings.notificationCancelAllCallback != nil)
}

@MainActor
@Test
func
  testRuntimeBridgeDetachUnregistersDocumentPermissionBackgroundCalendarContactIntentLocationMediaAndNotificationLanesForReattach()
  throws
{
  let bindings = RuntimeAbiSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    contactRequests: ContactRequestRecorder(),
    calendarRequests: CalendarRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.detach()
  try bridge.attach(runtimeHost: runtimeHost)

  #expect(bindings.detachedSessionHandles == [sessionHandle])
  #expect(bindings.attachedSessionHandles == [sessionHandle, sessionHandle])
}

@MainActor
@Test
func testRuntimeBridgeRejectsMismatchedRuntimeHostSessionHandle() {
  let bindings = RuntimeAbiSpy()
  let bridgeSessionHandle = makeTestSessionHandle()
  let hostSessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: bridgeSessionHandle,
    bindings: bindings
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: hostSessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    contactRequests: ContactRequestRecorder(),
    calendarRequests: CalendarRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )

  do {
    try bridge.attach(runtimeHost: runtimeHost)
    #expect(Bool(false))
  } catch let error as BridgeError {
    #expect(
      error.message == "runtime bridge session handle does not match the attached runtime host"
    )
  } catch {
    #expect(Bool(false))
  }
}

@MainActor
@Test
func testRuntimeBridgeClearsRuntimeHostAfterFailedRegistration() throws {
  let bindings = RuntimeAbiSpy()
  bindings.attachStatus = 6
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    contactRequests: ContactRequestRecorder(),
    calendarRequests: CalendarRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let request = RuntimeHostDocumentRequest(requestID: HostRequestID(rawValue: 3))

  do {
    try bridge.attach(runtimeHost: runtimeHost)
    #expect(Bool(false))
  } catch let error as BridgeError {
    #expect(error.message == "runtime bridge could not attach runtime abi bindings: 6")
  } catch {
    #expect(Bool(false))
  }

  let status = withBridgeDocumentRequest(request) { bridgeRequest in
    bindings.documentCallback!(sessionHandle.rawValue, bridgeRequest)
  }

  #expect(status == 3)
}
