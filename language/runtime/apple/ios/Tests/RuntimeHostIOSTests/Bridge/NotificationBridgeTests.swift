import Foundation
import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testProcessRuntimeAbiRoundtripsNotificationRequest() throws {
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
    bindings: bindings,
    notificationTimestampNs: { 42 }
  )
  let permissionRequests = PermissionRequestRecorder()
  let documentRequests = DocumentRequestRecorder()
  let notificationRequests = NotificationRequestRecorder()
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: permissionRequests,
    documentRequests: documentRequests,
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder(),
    notificationRequests: notificationRequests
  )
  let request = RuntimeHostNotificationRequest(
    identifier: "notification-1",
    title: "Title",
    body: "Body"
  )

  try bridge.attach(runtimeHost: runtimeHost)

  defer {
    bridge.detach()
  }

  let submitStatus = bindings.submitTestNotificationPost(
    sessionHandle: sessionHandle,
    request: request
  )

  #expect(submitStatus == hostStatusOk)
  #expect(notificationRequests.postedRequests == [request])
}

@MainActor
@Test
func testRuntimeBridgeSendsNotificationEventsIntoRuntimeIngress() throws {
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
  let event = RuntimeHostNotificationEvent(
    identifier: "test-notification",
    request: RuntimeHostNotificationRequest(
      identifier: "test-notification",
      title: "Title",
      body: "Body"
    ),
    kind: .activated,
    actionIdentifier: "open"
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.sendNotificationEvent(event)

  #expect(bindings.notificationEvents.count == 1)
  #expect(bindings.notificationEvents[0].0 == sessionHandle)
  #expect(bindings.notificationEvents[0].1 == event)
}
