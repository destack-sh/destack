import Foundation
import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testRuntimeBridgeSendsNotificationEventsIntoRuntimeIngress() throws {
  let bindings = RuntimeIngressSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    runtimeApi: bindings,
    notificationTimestampNs: { 42 }
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let event = RuntimeHostNotificationEvent(
    identifier: "test-notification",
    request: RuntimeHostNotificationRequest(
      identifier: "test-notification",
      title: "Title",
      body: "Body"
    ),
    kind: .activated,
    actionIdentifier: "open",
    sequence: 1,
    timestampNs: 42
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.notifyNotificationEvent(event)

  #expect(bindings.notificationEvents.count == 1)
  #expect(bindings.notificationEvents[0].0 == sessionHandle)
  #expect(bindings.notificationEvents[0].1 == event)
}
