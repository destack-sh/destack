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
    runtimeApi: bindings
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let event = RuntimeHostNotificationEvent(
    kind: .interacted,
    metadata: RuntimeHostNotificationEventMetadata(
      timestampNs: 42,
      sequence: 1,
      id: "test-notification",
      request: RuntimeHostNotificationRequest(
        title: "Title",
        body: "Body",
        tag: "test-notification"
      )
    ),
    payload: RuntimeHostNotificationInteractedPayload(
      actionId: "open"
    )
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.notifyNotificationEvent(event)

  #expect(bindings.notificationEvents.count == 1)
  #expect(bindings.notificationEvents[0].0 == sessionHandle)
  #expect(bindings.notificationEvents[0].1 == event)
}
