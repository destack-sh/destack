import Foundation

/// The Apple background scheduler availability state.
public enum RuntimeHostBackgroundStatus: UInt32, Sendable, Hashable, Codable {
  /// The scheduler is unavailable on this host.
  case unavailable = 1
  /// The scheduler is available but restricted.
  case restricted = 2
  /// The scheduler is available.
  case available = 3
}

/// The Apple background trigger class.
public enum RuntimeHostBackgroundTriggerKind: UInt32, Sendable, Hashable, Codable {
  /// One opportunistic refresh task.
  case appRefresh = 1
  /// One longer processing task.
  case processing = 2
}

/// The Apple background network requirement.
public enum RuntimeHostBackgroundNetworkRequirement: UInt32, Sendable, Hashable, Codable {
  /// No network route is required.
  case none = 1
  /// One network route is required.
  case connected = 2
  /// One unmetered network route is required.
  case unmetered = 3
}

/// The Apple background conflict policy.
public enum RuntimeHostBackgroundConflictPolicy: UInt32, Sendable, Hashable, Codable {
  /// Replace one existing registration.
  case replace = 1
  /// Keep one existing registration.
  case keep = 2
}

/// The Apple background schedule class.
public enum RuntimeHostBackgroundTaskScheduleKind: UInt32, Sendable, Hashable, Codable {
  /// One single future execution.
  case once = 1
  /// One recurring execution stream.
  case recurring = 2
}

/// The Apple background completion class.
public enum RuntimeHostBackgroundTaskResult: UInt32, Sendable, Hashable, Codable {
  /// The task completed successfully.
  case success = 1
  /// The task should be retried.
  case retry = 2
  /// The task failed permanently.
  case failure = 3
}

/// One Apple background task schedule payload.
public struct RuntimeHostBackgroundTaskSchedule: Sendable, Hashable, Codable {
  /// The schedule class.
  public let kind: RuntimeHostBackgroundTaskScheduleKind
  /// The earliest execution target in UTC nanoseconds when provided.
  public let earliestBeginUnixNs: UInt64?
  /// The repeat interval in nanoseconds for recurring schedules.
  public let repeatIntervalNs: UInt64?

  /// Create one background task schedule payload.
  public init(
    kind: RuntimeHostBackgroundTaskScheduleKind,
    earliestBeginUnixNs: UInt64?,
    repeatIntervalNs: UInt64? = nil
  ) {
    self.kind = kind
    self.earliestBeginUnixNs = earliestBeginUnixNs
    self.repeatIntervalNs = repeatIntervalNs
  }

  /// Return whether this schedule uses recurring execution.
  public var isRecurring: Bool {
    kind == .recurring
  }
}

/// One Apple background task registration payload.
public struct RuntimeHostBackgroundTaskOptions: Sendable, Hashable, Codable {
  /// The stable task identifier.
  public let identifier: String
  /// The trigger class.
  public let trigger: RuntimeHostBackgroundTriggerKind
  /// The requested schedule payload.
  public let schedule: RuntimeHostBackgroundTaskSchedule
  /// The requested network requirement.
  public let network: RuntimeHostBackgroundNetworkRequirement
  /// Whether charging power is required.
  public let requiresCharging: Bool
  /// Whether idle mode is required.
  public let requiresIdle: Bool
  /// The registration conflict policy.
  public let conflictPolicy: RuntimeHostBackgroundConflictPolicy

  /// Create one background task options payload.
  public init(
    identifier: String,
    trigger: RuntimeHostBackgroundTriggerKind,
    schedule: RuntimeHostBackgroundTaskSchedule,
    network: RuntimeHostBackgroundNetworkRequirement,
    requiresCharging: Bool,
    requiresIdle: Bool,
    conflictPolicy: RuntimeHostBackgroundConflictPolicy
  ) {
    self.identifier = identifier
    self.trigger = trigger
    self.schedule = schedule
    self.network = network
    self.requiresCharging = requiresCharging
    self.requiresIdle = requiresIdle
    self.conflictPolicy = conflictPolicy
  }
}

/// One Apple background status response.
public struct RuntimeHostBackgroundStatusResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The scheduler status when available.
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

/// One Apple background task descriptor payload.
public struct RuntimeHostBackgroundTaskDescriptor: Sendable, Hashable, Codable {
  /// The stable task identifier.
  public let identifier: String
  /// The trigger class.
  public let trigger: RuntimeHostBackgroundTriggerKind
  /// The effective schedule payload.
  public let schedule: RuntimeHostBackgroundTaskSchedule
  /// The effective network requirement.
  public let network: RuntimeHostBackgroundNetworkRequirement
  /// Whether charging power is required.
  public let requiresCharging: Bool
  /// Whether idle mode is required.
  public let requiresIdle: Bool
  /// The registration conflict policy.
  public let conflictPolicy: RuntimeHostBackgroundConflictPolicy

  /// Create one background task descriptor.
  public init(
    identifier: String,
    trigger: RuntimeHostBackgroundTriggerKind,
    schedule: RuntimeHostBackgroundTaskSchedule,
    network: RuntimeHostBackgroundNetworkRequirement,
    requiresCharging: Bool,
    requiresIdle: Bool,
    conflictPolicy: RuntimeHostBackgroundConflictPolicy
  ) {
    self.identifier = identifier
    self.trigger = trigger
    self.schedule = schedule
    self.network = network
    self.requiresCharging = requiresCharging
    self.requiresIdle = requiresIdle
    self.conflictPolicy = conflictPolicy
  }
}

/// One Apple background task-list response.
public struct RuntimeHostBackgroundListResponse: Sendable, Hashable, Codable {
  /// The host status code.
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

/// One Apple background event metadata payload.
public struct RuntimeHostBackgroundEventMetadata: Sendable, Hashable, Codable {
  /// The monotonic event timestamp in nanoseconds.
  public let timestampNs: UInt64
  /// The monotonic event sequence number.
  public let sequence: UInt64
  /// The stable task identifier.
  public let identifier: String
  /// The stable execution identifier.
  public let executionID: String
  /// The execution deadline in UTC nanoseconds.
  public let deadlineUnixNs: UInt64

  /// Create one background event metadata payload.
  public init(
    timestampNs: UInt64,
    sequence: UInt64,
    identifier: String,
    executionID: String,
    deadlineUnixNs: UInt64
  ) {
    self.timestampNs = timestampNs
    self.sequence = sequence
    self.identifier = identifier
    self.executionID = executionID
    self.deadlineUnixNs = deadlineUnixNs
  }
}

/// The Apple background event kind.
public enum RuntimeHostBackgroundEventKind: UInt32, Sendable, Codable {
  /// One task became ready to execute.
  case taskReady = 1
  /// One task execution expired.
  case taskExpired = 2
}

/// One Apple background ingress event delivered into one runtime session.
public struct RuntimeHostBackgroundEvent: Sendable, Hashable, Codable {
  /// The event kind.
  public let kind: RuntimeHostBackgroundEventKind
  /// The shared event metadata.
  public let metadata: RuntimeHostBackgroundEventMetadata

  /// Create one background event.
  public init(
    kind: RuntimeHostBackgroundEventKind,
    metadata: RuntimeHostBackgroundEventMetadata
  ) {
    self.kind = kind
    self.metadata = metadata
  }
}

/// One background unregister request.
public struct RuntimeHostBackgroundUnregisterRequest: Sendable, Hashable, Codable {
  /// The stable task identifier.
  public let identifier: String

  /// Create one background unregister request.
  public init(identifier: String) {
    self.identifier = identifier
  }
}

/// One background trigger-test request.
public struct RuntimeHostBackgroundTriggerTestRequest: Sendable, Hashable, Codable {
  /// The stable task identifier.
  public let identifier: String

  /// Create one background trigger-test request.
  public init(identifier: String) {
    self.identifier = identifier
  }
}

/// One Apple background trigger-test response.
public struct RuntimeHostBackgroundTriggerTestResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// Whether the host triggered one task execution.
  public let isTriggered: Bool

  /// Create one background trigger-test response.
  public init(
    status: UInt32,
    isTriggered: Bool = false
  ) {
    self.status = status
    self.isTriggered = isTriggered
  }
}

/// One background complete request.
public struct RuntimeHostBackgroundCompleteRequest: Sendable, Hashable, Codable {
  /// The stable execution identifier.
  public let executionID: String
  /// The completion result.
  public let result: RuntimeHostBackgroundTaskResult

  /// Create one background complete request.
  public init(
    executionID: String,
    result: RuntimeHostBackgroundTaskResult
  ) {
    self.executionID = executionID
    self.result = result
  }
}
