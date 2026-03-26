import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testRuntimeBridgeRoutesBackgroundRequestsIntoRuntimeHost() throws {
  let bindings = RuntimeAbiSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    bindings: bindings
  )
  let backgroundRequests = BackgroundRequestRecorder()
  backgroundRequests.listResponse = RuntimeHostBackgroundTaskListResponse(
    status: hostStatusOk,
    descriptors: [
      RuntimeHostBackgroundTaskDescriptor(
        identifier: "sync",
        trigger: .processing,
        schedule: RuntimeHostBackgroundTaskSchedule(
          kind: .recurring,
          earliestBeginUnixNs: 0,
          repeatIntervalNs: 60_000_000_000
        ),
        network: .connected,
        requiresCharging: false,
        requiresIdle: false,
        conflictPolicy: .replace
      )
    ]
  )
  let runtimeHost = createBridgeRuntimeHost(
    sessionHandle: sessionHandle,
    permissionRequests: PermissionRequestRecorder(),
    documentRequests: DocumentRequestRecorder(),
    backgroundRequests: backgroundRequests,
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let options = RuntimeHostBackgroundTaskOptions(
    identifier: "sync",
    trigger: .processing,
    schedule: RuntimeHostBackgroundTaskSchedule(
      kind: .recurring,
      earliestBeginUnixNs: 0,
      repeatIntervalNs: 60_000_000_000
    ),
    network: .connected,
    requiresCharging: false,
    requiresIdle: false,
    conflictPolicy: .replace
  )

  try bridge.attach(runtimeHost: runtimeHost)

  let backgroundStatus = bridge.backgroundBridge.backgroundStatus(runtimeHost: runtimeHost)
  let listResponse = bridge.backgroundBridge.listBackgroundTasks(runtimeHost: runtimeHost)
  let registerStatus = bridge.backgroundBridge.registerBackgroundTask(
    runtimeHost: runtimeHost,
    options
  )
  let unregisterStatus = bridge.backgroundBridge.unregisterBackgroundTask(
    runtimeHost: runtimeHost,
    "sync"
  )
  let triggerResponse = bridge.backgroundBridge.triggerBackgroundTask(
    runtimeHost: runtimeHost,
    "sync"
  )
  let completeStatus = bridge.backgroundBridge.completeBackgroundTask(
    runtimeHost: runtimeHost,
    executionID: "execution-1",
    result: .success
  )

  #expect(backgroundStatus.status == hostStatusOk)
  #expect(backgroundStatus.schedulerStatus == .available)
  #expect(listResponse.status == hostStatusOk)
  #expect(registerStatus == hostStatusOk)
  #expect(unregisterStatus == hostStatusOk)
  #expect(triggerResponse.status == hostStatusOk)
  #expect(completeStatus == hostStatusOk)
  #expect(triggerResponse.isTriggered)
  #expect(listResponse.descriptors == backgroundRequests.listResponse.descriptors)
  #expect(backgroundRequests.registerCalls == [options])
  #expect(backgroundRequests.unregisterCalls == ["sync"])
  #expect(backgroundRequests.triggerCalls == ["sync"])
  #expect(backgroundRequests.completeCalls.count == 1)
  #expect(backgroundRequests.completeCalls[0].0 == "execution-1")
  #expect(backgroundRequests.completeCalls[0].1 == .success)
}

@MainActor
@Test
func testRuntimeBridgeSendsBackgroundEventsIntoRuntimeIngress() throws {
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
    backgroundRequests: BackgroundRequestRecorder(),
    intentRequests: IntentRequestRecorder(),
    mediaRequests: MediaRequestRecorder()
  )
  let event = RuntimeHostBackgroundEvent(
    kind: .taskReady,
    metadata: RuntimeHostBackgroundEventMetadata(
      timestampNs: 42,
      sequence: 7,
      identifier: "sync",
      executionID: "execution-1",
      deadlineUnixNs: 99
    )
  )

  try bridge.attach(runtimeHost: runtimeHost)
  bridge.sendBackgroundEvent(event)

  #expect(bindings.backgroundEvents.count == 1)
  #expect(bindings.backgroundEvents[0].0 == sessionHandle)
  #expect(bindings.backgroundEvents[0].1 == event)
}
