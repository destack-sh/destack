import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testProcessRuntimeAbiRoundtripsPermissionRequest() throws {
  guard hasRuntimeBridgeLibrary() else {
    return
  }

  let bindings = ProcessRuntimeAbi()
  let sessionHandle = bindings.openTestSession()

  defer {
    bindings.closeTestSession(sessionHandle: sessionHandle)
  }

  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings
  )
  let permissionRequests = PermissionRequestRecorder()
  let documentRequests = DocumentRequestRecorder()
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: permissionRequests,
    documentRequests: documentRequests,
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let expectedRequest = RuntimeHostPermissionRequest(
    requestID: HostRequestID(rawValue: 5),
    permission: "camera"
  )

  try bridge.attach(runtimeHost: runtimeHost)

  defer {
    bridge.detach()
  }

  let submitStatus = bindings.submitTestPermissionRequest(
    sessionHandle: sessionHandle,
    requestID: expectedRequest.requestID,
    permission: expectedRequest.permission
  )

  #expect(submitStatus == hostStatusOk)
  #expect(permissionRequests.requests == [expectedRequest])
}

@MainActor
@Test
func testRuntimeBridgeRoutesPermissionRequestsIntoRuntimeHost() throws {
  let bindings = RuntimeAbiSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings
  )
  let permissionRequests = PermissionRequestRecorder()
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: permissionRequests,
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let request = RuntimeHostPermissionRequest(
    requestID: HostRequestID(rawValue: 5),
    permission: "camera"
  )

  try bridge.attach(runtimeHost: runtimeHost)
  let status = withBridgePermissionRequest(request) { bridgeRequest in
    bindings.permissionRequestCallback!(sessionHandle.rawValue, bridgeRequest)
  }

  #expect(status == 0)
  #expect(permissionRequests.requests == [request])
}

@MainActor
@Test
func testRuntimeBridgeRoutesPermissionSettingsRequestsIntoRuntimeHost() throws {
  let bindings = RuntimeAbiSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings
  )
  let permissionRequests = PermissionRequestRecorder()
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: permissionRequests,
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )

  try bridge.attach(runtimeHost: runtimeHost)
  let status = bindings.permissionOpenSettingsCallback!(sessionHandle.rawValue)

  #expect(status == hostStatusOk)
  #expect(permissionRequests.openSettingsCalls == 1)
}

@MainActor
@Test
func testRuntimeBridgeSendsPermissionEventsIntoRuntimeIngress() throws {
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
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let event = RuntimeHostPermissionEvent(
    requestID: HostRequestID(rawValue: 9),
    permission: "camera",
    isGranted: true
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.sendPermissionEvent(event)

  #expect(bindings.permissionEvents.count == 1)
  #expect(bindings.permissionEvents[0].0 == sessionHandle)
  #expect(bindings.permissionEvents[0].1 == event.requestID)
  #expect(bindings.permissionEvents[0].2 == event.permission)
  #expect(bindings.permissionEvents[0].3 == event.isGranted)
}

@MainActor
@Test
func testRuntimeBridgeRejectsInvalidPermissionRequest() throws {
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
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )

  try bridge.attach(runtimeHost: runtimeHost)
  let invalidRequest = DestackRustPermissionRequest(
    request_id: 5,
    permissions: DestackRustStringSlice(data: nil, len: 1)
  )
  let status = bindings.permissionRequestCallback!(sessionHandle.rawValue, invalidRequest)

  #expect(status == 2)
}
