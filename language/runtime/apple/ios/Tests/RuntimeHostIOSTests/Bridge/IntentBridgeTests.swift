import Foundation
import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testRuntimeBridgeSendsIntentEventsIntoRuntimeIngress() throws {
  let bindings = RuntimeIngressSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    runtimeApi: bindings
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

  bridge.notifyIntentEvent(event)

  #expect(bindings.intentEvents.count == 1)
  #expect(bindings.intentEvents[0].0 == sessionHandle)
  #expect(bindings.intentEvents[0].1 == event)
}
