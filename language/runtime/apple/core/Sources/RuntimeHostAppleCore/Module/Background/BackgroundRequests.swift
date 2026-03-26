import Foundation

/// One Apple background status response.
public struct RuntimeHostBackgroundStatusResponse: Sendable, Hashable, Codable {
  /// The bridge status code for this request.
  public let status: UInt32
  /// The reported scheduler availability when available.
  public let schedulerStatus: RuntimeHostBackgroundStatus?

  /// Create one background status response.
  public init(
    status: UInt32,
    schedulerStatus: RuntimeHostBackgroundStatus? = nil
  ) {
    self.status = status
    self.schedulerStatus = schedulerStatus
  }
}

/// One Apple background task-list response.
public struct RuntimeHostBackgroundTaskListResponse: Sendable, Hashable, Codable {
  /// The bridge status code for this request.
  public let status: UInt32
  /// The returned task descriptors.
  public let descriptors: [RuntimeHostBackgroundTaskDescriptor]

  /// Create one background task-list response.
  public init(
    status: UInt32,
    descriptors: [RuntimeHostBackgroundTaskDescriptor] = []
  ) {
    self.status = status
    self.descriptors = descriptors
  }
}

/// One Apple background trigger response.
public struct RuntimeHostBackgroundTriggerResponse: Sendable, Hashable, Codable {
  /// The bridge status code for this request.
  public let status: UInt32
  /// Whether the host triggered one task execution.
  public let isTriggered: Bool

  /// Create one background trigger response.
  public init(
    status: UInt32,
    isTriggered: Bool = false
  ) {
    self.status = status
    self.isTriggered = isTriggered
  }
}

/// The background request surface attached to one Apple runtime host.
@MainActor
public protocol BackgroundRequests {
  /// Read the Apple background scheduler status.
  func backgroundStatus() -> RuntimeHostBackgroundStatusResponse

  /// List registered Apple background tasks.
  func listBackgroundTasks() -> RuntimeHostBackgroundTaskListResponse

  /// Register one Apple background task.
  func registerBackgroundTask(
    _ options: RuntimeHostBackgroundTaskOptions
  ) -> UInt32

  /// Unregister one Apple background task.
  func unregisterBackgroundTask(
    _ identifier: String
  ) -> UInt32

  /// Trigger one Apple background task for testing.
  func triggerBackgroundTask(
    _ identifier: String
  ) -> RuntimeHostBackgroundTriggerResponse

  /// Complete one Apple background task execution.
  func completeBackgroundTask(
    executionID: String,
    result: RuntimeHostBackgroundTaskResult
  ) -> UInt32
}

/// The explicit unsupported background request surface for one Apple runtime host.
@MainActor
public final class UnsupportedBackgroundRequests: BackgroundRequests {
  /// Create one unsupported background request surface.
  public init() {}

  /// Read the Apple background scheduler status.
  public func backgroundStatus() -> RuntimeHostBackgroundStatusResponse {
    RuntimeHostBackgroundStatusResponse(status: hostStatusNotSupported)
  }

  /// List registered Apple background tasks.
  public func listBackgroundTasks() -> RuntimeHostBackgroundTaskListResponse {
    RuntimeHostBackgroundTaskListResponse(status: hostStatusNotSupported)
  }

  /// Register one Apple background task.
  public func registerBackgroundTask(
    _ options: RuntimeHostBackgroundTaskOptions
  ) -> UInt32 {
    let _ = options

    return hostStatusNotSupported
  }

  /// Unregister one Apple background task.
  public func unregisterBackgroundTask(
    _ identifier: String
  ) -> UInt32 {
    let _ = identifier

    return hostStatusNotSupported
  }

  /// Trigger one Apple background task for testing.
  public func triggerBackgroundTask(
    _ identifier: String
  ) -> RuntimeHostBackgroundTriggerResponse {
    let _ = identifier

    return RuntimeHostBackgroundTriggerResponse(status: hostStatusNotSupported)
  }

  /// Complete one Apple background task execution.
  public func completeBackgroundTask(
    executionID: String,
    result: RuntimeHostBackgroundTaskResult
  ) -> UInt32 {
    let _ = executionID
    let _ = result

    return hostStatusNotSupported
  }
}
