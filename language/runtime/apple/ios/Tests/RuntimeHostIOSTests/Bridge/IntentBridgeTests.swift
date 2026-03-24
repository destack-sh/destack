import Foundation
import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testProcessRuntimeAbiRoundtripsIntentCanOpenUrlRequest() throws {
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
  let intentRequests = IntentRequestRecorder()
  intentRequests.isOpenURLSupported = true
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: permissionRequests,
    documentRequests: documentRequests,
    intentRequests: intentRequests,
    mediaRequests: MediaRequestRecorder()
  )

  try bridge.attach(runtimeHost: runtimeHost)

  defer {
    bridge.detach()
  }

  let (status, isSupported) = bindings.testIntentCanOpenURL(
    sessionHandle: sessionHandle,
    url: "https://example.com"
  )

  #expect(status == hostStatusOk)
  #expect(isSupported)
  #expect(intentRequests.canOpenURLCalls == ["https://example.com"])
}

@MainActor
@Test
func testRuntimeBridgeSendsIntentEventsIntoRuntimeIngress() throws {
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

  defer {
    bridge.detach()
  }

  let event = RuntimeHostIntentEvent(
    source: "app://origin",
    payload: .shareText(
      text: "hello",
      contentType: "text/plain"
    )
  )

  bridge.sendIntentEvent(event)

  #expect(bindings.intentEvents.count == 1)
  #expect(bindings.intentEvents[0].0 == sessionHandle)
  #expect(bindings.intentEvents[0].1 == event)
}
