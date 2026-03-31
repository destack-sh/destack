import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@MainActor
@Test
func testRuntimeBridgeRoutesBackgroundRequestsIntoRuntimeHost() throws {
  let bindings = RuntimeIngressSpy()
  let sessionHandle = makeTestSessionHandle()
  let bridge = RuntimeBridge(
    sessionHandle: sessionHandle,
    runtimeApi: bindings
  )
  let backgroundRequests = BackgroundRequestRecorder()
  backgroundRequests.listResponse = RuntimeHostBackgroundListResponse(
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

  let backgroundStatus = bridge.backgroundStatus(runtimeHost: runtimeHost)
  let listResponse = bridge.backgroundList(runtimeHost: runtimeHost)
  let registerStatus = bridge.backgroundRegisterTask(runtimeHost: runtimeHost, options)
  let unregisterStatus = bridge.backgroundUnregister(
    runtimeHost: runtimeHost,
    RuntimeHostBackgroundUnregisterRequest(identifier: "sync")
  )
  let triggerResponse = bridge.backgroundTriggerTest(
    runtimeHost: runtimeHost,
    RuntimeHostBackgroundTriggerTestRequest(identifier: "sync")
  )
  let completeStatus = bridge.backgroundComplete(
    runtimeHost: runtimeHost,
    RuntimeHostBackgroundCompleteRequest(
      executionID: "execution-1",
      result: .success
    )
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
  bridge.notifyBackgroundEvent(event)

  #expect(bindings.backgroundEvents.count == 1)
  #expect(bindings.backgroundEvents[0].0 == sessionHandle)
  #expect(bindings.backgroundEvents[0].1 == event)
}
