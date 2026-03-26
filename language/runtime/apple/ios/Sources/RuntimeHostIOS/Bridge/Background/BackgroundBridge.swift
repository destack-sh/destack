import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let backgroundStatusCallback: BackgroundStatusCallback = {
  sessionHandle,
  outputStatus in
  handleBackgroundStatus(
    sessionHandle: sessionHandle,
    outputStatus: outputStatus
  )
}

let backgroundListCallback: BackgroundListCallback = {
  sessionHandle,
  outputDescriptors in
  handleBackgroundList(
    sessionHandle: sessionHandle,
    outputDescriptors: outputDescriptors
  )
}

let backgroundRegisterCallback: BackgroundRegisterCallback = {
  sessionHandle,
  options in
  handleBackgroundRegister(
    sessionHandle: sessionHandle,
    options: options
  )
}

let backgroundUnregisterCallback: BackgroundUnregisterCallback = {
  sessionHandle,
  identifier in
  handleBackgroundUnregister(
    sessionHandle: sessionHandle,
    bridgeIdentifier: identifier
  )
}

let backgroundTriggerTestCallback: BackgroundTriggerTestCallback = {
  sessionHandle,
  identifier,
  isTriggered in
  handleBackgroundTriggerTest(
    sessionHandle: sessionHandle,
    bridgeIdentifier: identifier,
    isTriggered: isTriggered
  )
}

let backgroundCompleteCallback: BackgroundCompleteCallback = {
  sessionHandle,
  executionID,
  result in
  handleBackgroundComplete(
    sessionHandle: sessionHandle,
    bridgeExecutionID: executionID,
    result: result
  )
}

/// One temporary native allocation arena for one background bridge callback.
private final class BackgroundBridgeArena {
  /// The raw deallocation actions recorded for this callback.
  private var deallocations: [() -> Void] = []

  /// Remove every recorded allocation before one new callback payload is encoded.
  func reset() {
    for deallocate in deallocations.reversed() {
      deallocate()
    }

    deallocations.removeAll(keepingCapacity: true)
  }

  /// Release every recorded native allocation.
  deinit {
    for deallocate in deallocations.reversed() {
      deallocate()
    }
  }

  /// Allocate one copied UTF-8 buffer for one Swift string.
  func makeStringRef(
    _ value: String
  ) -> DestackRustStringRef {
    let bytes = Array(value.utf8)
    if bytes.isEmpty {
      return DestackRustStringRef(data: nil, len: 0)
    }

    let storage = UnsafeMutablePointer<UInt8>.allocate(capacity: bytes.count)
    storage.initialize(from: bytes, count: bytes.count)
    deallocations.append {
      storage.deinitialize(count: bytes.count)
      storage.deallocate()
    }

    return DestackRustStringRef(
      data: UnsafePointer(storage),
      len: UInt32(bytes.count)
    )
  }

  /// Allocate one copied native array and fill it with one builder closure.
  func makeArray<Element>(
    count: Int,
    fill: (UnsafeMutableBufferPointer<Element>) -> Void
  ) -> UnsafeMutablePointer<Element>? {
    if count == 0 {
      return nil
    }

    let storage = UnsafeMutablePointer<Element>.allocate(capacity: count)
    let buffer = UnsafeMutableBufferPointer(start: storage, count: count)
    fill(buffer)
    deallocations.append {
      storage.deinitialize(count: count)
      storage.deallocate()
    }

    return storage
  }
}

/// Return the thread-local background bridge arena for one callback thread.
private func currentBackgroundBridgeArena() -> BackgroundBridgeArena {
  let dictionary = Thread.current.threadDictionary
  let key = "dev.destack.runtime.apple.background-bridge-arena"

  if let arena = dictionary[key] as? BackgroundBridgeArena {
    arena.reset()

    return arena
  }

  let arena = BackgroundBridgeArena()
  dictionary[key] = arena

  return arena
}

/// One background bridge lane for one attached iOS runtime host.
@MainActor
final class BackgroundBridge: BackgroundEvents {
  /// The runtime session routed through this background bridge lane.
  let sessionHandle: HostSessionHandle
  private let bindings: any BackgroundAbi

  /// Create one background bridge lane.
  init(
    sessionHandle: HostSessionHandle,
    bindings: any BackgroundAbi
  ) {
    self.sessionHandle = sessionHandle
    self.bindings = bindings
  }

  /// Send one host background event into the runtime ingress path.
  func sendBackgroundEvent(
    _ event: RuntimeHostBackgroundEvent
  ) {
    let status = bindings.notifyBackgroundEvent(
      sessionHandle: sessionHandle,
      event: event
    )

    precondition(
      status.code == hostStatusOk,
      "runtime bridge could not deliver background event: code \(status.code), error \(status.errorID)"
    )
  }

  /// Read the Apple background scheduler status through the attached host.
  func backgroundStatus(
    runtimeHost: RuntimeHost
  ) -> RuntimeHostBackgroundStatusResponse {
    runtimeHost.backgroundRequests.backgroundStatus()
  }

  /// List registered Apple background tasks through the attached host.
  func listBackgroundTasks(
    runtimeHost: RuntimeHost
  ) -> RuntimeHostBackgroundTaskListResponse {
    runtimeHost.backgroundRequests.listBackgroundTasks()
  }

  /// Register one Apple background task through the attached host.
  func registerBackgroundTask(
    runtimeHost: RuntimeHost,
    _ options: RuntimeHostBackgroundTaskOptions
  ) -> UInt32 {
    runtimeHost.backgroundRequests.registerBackgroundTask(options)
  }

  /// Unregister one Apple background task through the attached host.
  func unregisterBackgroundTask(
    runtimeHost: RuntimeHost,
    _ identifier: String
  ) -> UInt32 {
    runtimeHost.backgroundRequests.unregisterBackgroundTask(identifier)
  }

  /// Trigger one Apple background task through the attached host.
  func triggerBackgroundTask(
    runtimeHost: RuntimeHost,
    _ identifier: String
  ) -> RuntimeHostBackgroundTriggerResponse {
    runtimeHost.backgroundRequests.triggerBackgroundTask(identifier)
  }

  /// Complete one Apple background task through the attached host.
  func completeBackgroundTask(
    runtimeHost: RuntimeHost,
    executionID: String,
    result: RuntimeHostBackgroundTaskResult
  ) -> UInt32 {
    runtimeHost.backgroundRequests.completeBackgroundTask(
      executionID: executionID,
      result: result
    )
  }
}

/// Resolve one registered background bridge for one runtime session.
func guardBackgroundBridge(
  sessionHandle: UInt64
) -> BackgroundBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.backgroundBridge
}

/// Handle one runtime callback asking for background scheduler status.
private func handleBackgroundStatus(
  sessionHandle: UInt64,
  outputStatus: UnsafeMutablePointer<UInt32>?
) -> UInt32 {
  guard let outputStatus else {
    return hostStatusInvalidArgument
  }
  guard let bridge = guardBackgroundBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let response = runOnMainThread {
    bridge.backgroundStatus(runtimeHost: runtimeHost)
  }
  guard response.status == hostStatusOk, let schedulerStatus = response.schedulerStatus else {
    return response.status
  }

  outputStatus.pointee = encodeBackgroundStatus(schedulerStatus)

  return hostStatusOk
}

/// Handle one runtime callback asking for registered background tasks.
private func handleBackgroundList(
  sessionHandle: UInt64,
  outputDescriptors: UnsafeMutablePointer<DestackRustBackgroundTaskDescriptorArray>?
) -> UInt32 {
  guard let outputDescriptors else {
    return hostStatusInvalidArgument
  }
  guard let bridge = guardBackgroundBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let response = runOnMainThread {
    bridge.listBackgroundTasks(runtimeHost: runtimeHost)
  }
  guard response.status == hostStatusOk else {
    return response.status
  }

  outputDescriptors.pointee = encodeBackgroundTaskDescriptors(response.descriptors)

  return hostStatusOk
}

/// Handle one runtime callback asking to register one background task.
private func handleBackgroundRegister(
  sessionHandle: UInt64,
  options nativeOptions: DestackRustBackgroundTaskOptions
) -> UInt32 {
  guard let bridge = guardBackgroundBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let options: RuntimeHostBackgroundTaskOptions
  do {
    options = try decodeBackgroundTaskOptions(nativeOptions)
  } catch {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.registerBackgroundTask(
      runtimeHost: runtimeHost,
      options
    )
  }
}

/// Handle one runtime callback asking to unregister one background task.
private func handleBackgroundUnregister(
  sessionHandle: UInt64,
  bridgeIdentifier: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardBackgroundBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let identifier: String
  do {
    identifier = try tryDecodeNativeString(bridgeIdentifier)
  } catch {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.unregisterBackgroundTask(
      runtimeHost: runtimeHost,
      identifier
    )
  }
}

/// Handle one runtime callback asking to trigger one background task.
private func handleBackgroundTriggerTest(
  sessionHandle: UInt64,
  bridgeIdentifier: DestackRustStringRef,
  isTriggered: UnsafeMutablePointer<Bool>?
) -> UInt32 {
  guard let isTriggered else {
    return hostStatusInvalidArgument
  }
  guard let bridge = guardBackgroundBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let identifier: String
  do {
    identifier = try tryDecodeNativeString(bridgeIdentifier)
  } catch {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.triggerBackgroundTask(
      runtimeHost: runtimeHost,
      identifier
    )
  }
  guard response.status == hostStatusOk else {
    return response.status
  }

  isTriggered.pointee = response.isTriggered

  return hostStatusOk
}

/// Handle one runtime callback asking to complete one background task execution.
private func handleBackgroundComplete(
  sessionHandle: UInt64,
  bridgeExecutionID: DestackRustStringRef,
  result: UInt32
) -> UInt32 {
  guard let bridge = guardBackgroundBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  let executionID: String
  do {
    executionID = try tryDecodeNativeString(bridgeExecutionID)
  } catch {
    return hostStatusInvalidArgument
  }
  guard let result = decodeBackgroundTaskResult(result) else {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.completeBackgroundTask(
      runtimeHost: runtimeHost,
      executionID: executionID,
      result: result
    )
  }
}

/// Decode one bridge background task-options payload.
private func decodeBackgroundTaskOptions(
  _ options: DestackRustBackgroundTaskOptions
) throws -> RuntimeHostBackgroundTaskOptions {
  guard let trigger = decodeBackgroundTrigger(options.trigger) else {
    throw BridgeStringError.invalidStringSlice
  }
  guard let scheduleKind = decodeBackgroundScheduleKind(options.schedule.kind) else {
    throw BridgeStringError.invalidStringSlice
  }

  return RuntimeHostBackgroundTaskOptions(
    identifier: try tryDecodeNativeString(options.identifier),
    trigger: trigger,
    schedule: RuntimeHostBackgroundTaskSchedule(
      kind: scheduleKind,
      earliestBeginUnixNs: options.schedule.has_earliest_begin_unix_ns
        ? options.schedule.earliest_begin_unix_ns
        : nil,
      repeatIntervalNs: options.schedule.has_repeat_interval_ns
        ? options.schedule.repeat_interval_ns
        : nil
    ),
    network: try decodeBackgroundNetworkRequirement(options.network),
    requiresCharging: options.requires_charging,
    requiresIdle: options.requires_idle,
    conflictPolicy: try decodeBackgroundConflictPolicy(options.conflict_policy)
  )
}

/// Encode one Swift background status as one bridge value.
private func encodeBackgroundStatus(
  _ status: RuntimeHostBackgroundStatus
) -> UInt32 {
  switch status {
  case .unavailable:
    return 1
  case .restricted:
    return 2
  case .available:
    return 3
  }
}

/// Encode one Swift background task list as one bridge array payload.
private func encodeBackgroundTaskDescriptors(
  _ descriptors: [RuntimeHostBackgroundTaskDescriptor]
) -> DestackRustBackgroundTaskDescriptorArray {
  let arena = currentBackgroundBridgeArena()
  let data = arena.makeArray(count: descriptors.count) { buffer in
    for (index, descriptor) in descriptors.enumerated() {
      buffer[index] = DestackRustBackgroundTaskDescriptor(
        identifier: arena.makeStringRef(descriptor.identifier),
        trigger: descriptor.trigger.rawValue,
        schedule: DestackRustBackgroundTaskSchedule(
          kind: descriptor.schedule.kind.rawValue,
          has_earliest_begin_unix_ns: descriptor.schedule.earliestBeginUnixNs != nil,
          earliest_begin_unix_ns: descriptor.schedule.earliestBeginUnixNs ?? 0,
          has_repeat_interval_ns: descriptor.schedule.repeatIntervalNs != nil,
          repeat_interval_ns: descriptor.schedule.repeatIntervalNs ?? 0
        ),
        network: descriptor.network.rawValue,
        requires_charging: descriptor.requiresCharging,
        requires_idle: descriptor.requiresIdle,
        conflict_policy: descriptor.conflictPolicy.rawValue
      )
    }
  }

  return DestackRustBackgroundTaskDescriptorArray(
    data: data,
    len: UInt32(descriptors.count),
    capacity: UInt32(descriptors.count)
  )
}

/// Decode one Swift background schedule kind from one bridge value.
private func decodeBackgroundScheduleKind(
  _ value: UInt32
) -> RuntimeHostBackgroundTaskScheduleKind? {
  RuntimeHostBackgroundTaskScheduleKind(rawValue: value)
}

/// Decode one bridge background trigger as one Swift value.
private func decodeBackgroundTrigger(
  _ trigger: UInt32
) -> RuntimeHostBackgroundTriggerKind? {
  switch trigger {
  case 1:
    return .appRefresh
  case 2:
    return .processing
  default:
    return nil
  }
}

/// Decode one bridge background network requirement as one Swift value.
private func decodeBackgroundNetworkRequirement(
  _ network: UInt32
) throws -> RuntimeHostBackgroundNetworkRequirement {
  switch network {
  case 1:
    return .none
  case 2:
    return .connected
  case 3:
    return .unmetered
  default:
    throw BridgeStringError.invalidStringSlice
  }
}

/// Decode one bridge background conflict policy as one Swift value.
private func decodeBackgroundConflictPolicy(
  _ policy: UInt32
) throws -> RuntimeHostBackgroundConflictPolicy {
  switch policy {
  case 1:
    return .replace
  case 2:
    return .keep
  default:
    throw BridgeStringError.invalidStringSlice
  }
}

/// Decode one bridge background task result as one Swift value.
private func decodeBackgroundTaskResult(
  _ result: UInt32
) -> RuntimeHostBackgroundTaskResult? {
  switch result {
  case 1:
    return .success
  case 2:
    return .retry
  case 3:
    return .failure
  default:
    return nil
  }
}
