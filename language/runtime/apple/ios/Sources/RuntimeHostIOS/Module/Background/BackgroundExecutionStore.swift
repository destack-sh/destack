import Foundation
import RuntimeHostAppleCore

private let backgroundExecutionDeadlineNs: UInt64 = 30_000_000_000

/// One active iOS background execution.
@MainActor
internal final class BackgroundExecution {
  /// The stable task identifier.
  let identifier: String

  /// The stable execution identifier.
  let executionID: String

  /// The execution deadline in Unix nanoseconds.
  let deadlineUnixNs: UInt64

  /// The optional scheduler task bound to this execution.
  let task: (any IOSBackgroundRunningTask)?

  /// Create one live background execution.
  init(
    identifier: String,
    executionID: String,
    deadlineUnixNs: UInt64,
    task: (any IOSBackgroundRunningTask)?
  ) {
    self.identifier = identifier
    self.executionID = executionID
    self.deadlineUnixNs = deadlineUnixNs
    self.task = task
  }
}

/// The execution store for one iOS background host.
@MainActor
internal final class BackgroundExecutionStore {
  /// The monotonic clock used for event timestamps.
  private let monotonicNowNs: () -> UInt64

  /// The wall clock used for execution deadlines.
  private let wallClockNowNs: () -> UInt64

  /// The next event sequence number.
  private var nextSequence: UInt64 = 1

  /// The active executions keyed by execution identifier.
  private var activeExecutions: [String: BackgroundExecution] = [:]

  /// Create one background execution store.
  init(
    monotonicNowNs: @escaping () -> UInt64,
    wallClockNowNs: @escaping () -> UInt64
  ) {
    self.monotonicNowNs = monotonicNowNs
    self.wallClockNowNs = wallClockNowNs
  }

  /// Create one active execution for the given task identifier.
  func createExecution(
    identifier: String,
    task: (any IOSBackgroundRunningTask)?
  ) -> BackgroundExecution {
    // build one active execution snapshot
    let execution = BackgroundExecution(
      identifier: identifier,
      executionID: "ios-\(identifier)-\(monotonicNowNs())",
      deadlineUnixNs: wallClockNowNs() + backgroundExecutionDeadlineNs,
      task: task
    )

    // retain the execution until completion or expiration
    activeExecutions[execution.executionID] = execution

    return execution
  }

  /// Remove one execution by execution identifier.
  func removeExecution(
    _ executionID: String
  ) -> BackgroundExecution? {
    activeExecutions.removeValue(forKey: executionID)
  }

  /// Remove every execution for one task identifier.
  func removeExecutions(
    identifier: String
  ) -> [BackgroundExecution] {
    // collect the active execution identifiers first
    let executionIDs = activeExecutions.values
      .filter { execution in execution.identifier == identifier }
      .map { execution in execution.executionID }

    // remove the executions in a stable second pass
    return executionIDs.compactMap { executionID in
      activeExecutions.removeValue(forKey: executionID)
    }
  }

  /// Build one runtime background event for the given execution.
  func event(
    kind: RuntimeHostBackgroundEventKind,
    execution: BackgroundExecution
  ) -> RuntimeHostBackgroundEvent {
    RuntimeHostBackgroundEvent(
      kind: kind,
      metadata: RuntimeHostBackgroundEventMetadata(
        timestampNs: monotonicNowNs(),
        sequence: nextEventSequence(),
        identifier: execution.identifier,
        executionID: execution.executionID,
        deadlineUnixNs: execution.deadlineUnixNs
      )
    )
  }

  /// Return the next emitted event sequence number.
  private func nextEventSequence() -> UInt64 {
    // reserve the current sequence number
    let sequence = nextSequence

    // advance the next sequence number
    nextSequence &+= 1

    return sequence
  }
}
