import Foundation
import RuntimeHostAppleCore

private let backgroundRetryDelayNs: UInt64 = 10_000_000_000

/// The iOS app-level background request surface backed by `BGTaskScheduler`.
@MainActor
public final class BackgroundTaskSchedulerRequests: BackgroundRequests {
  /// The live ingress surface for emitted background events.
  private let events: any BackgroundEvents

  /// The platform scheduler backend.
  private let scheduler: any BackgroundSchedulerBackend

  /// The registration store for durable descriptors and handlers.
  private let registrations: BackgroundRegistrationStore

  /// The active execution store.
  private let executions: BackgroundExecutionStore

  /// The wall clock used for the next scheduler submission time.
  private let wallClockNowNs: () -> UInt64

  /// Create one live iOS background request surface.
  public convenience init(
    events: any BackgroundEvents
  ) {
    self.init(
      events: events,
      scheduler: BGTaskSchedulerBackend(),
      defaults: .standard,
      monotonicNowNs: {
        DispatchTime.now().uptimeNanoseconds
      },
      wallClockNowNs: {
        UInt64(Date().timeIntervalSince1970 * 1_000_000_000)
      }
    )
  }

  /// Create one background request surface with injected scheduler state.
  fileprivate init(
    events: any BackgroundEvents,
    scheduler: any BackgroundSchedulerBackend,
    defaults: UserDefaults,
    monotonicNowNs: @escaping () -> UInt64,
    wallClockNowNs: @escaping () -> UInt64
  ) {
    // capture the injected host dependencies
    self.events = events
    self.scheduler = scheduler
    self.registrations = BackgroundRegistrationStore(defaults: defaults)
    self.executions = BackgroundExecutionStore(
      monotonicNowNs: monotonicNowNs,
      wallClockNowNs: wallClockNowNs
    )
    self.wallClockNowNs = wallClockNowNs

    // restore registration handlers for durable tasks before the app becomes active
    for descriptor in registrations.listDescriptors() {
      _ = ensureLaunchHandler(for: descriptor)
    }
  }

  /// Return one descriptor adjusted for the next eligible scheduler submission time.
  nonisolated static func scheduledDescriptor(
    _ record: BackgroundTaskRecord,
    wallClockNowNs: UInt64
  ) -> RuntimeHostBackgroundTaskDescriptor {
    let record = resolvedRecordForScheduling(
      record,
      wallClockNowNs: wallClockNowNs
    )
    let descriptor = record.descriptor
    let runAtUnixNs: UInt64 =
      switch descriptor.schedule.kind {
      case .once:
        max(descriptor.schedule.earliestBeginUnixNs ?? wallClockNowNs, wallClockNowNs)
      case .recurring:
        record.nextRegularRunUnixNs ?? wallClockNowNs
      }

    return RuntimeHostBackgroundTaskDescriptor(
      identifier: descriptor.identifier,
      trigger: descriptor.trigger,
      schedule: RuntimeHostBackgroundTaskSchedule(
        kind: descriptor.schedule.kind,
        earliestBeginUnixNs: runAtUnixNs,
        repeatIntervalNs: descriptor.schedule.repeatIntervalNs
      ),
      network: descriptor.network,
      requiresCharging: descriptor.requiresCharging,
      requiresIdle: descriptor.requiresIdle,
      conflictPolicy: descriptor.conflictPolicy
    )
  }

  /// Return one descriptor adjusted for one earlier retry attempt.
  nonisolated static func retriedDescriptor(
    _ record: BackgroundTaskRecord,
    wallClockNowNs: UInt64
  ) -> RuntimeHostBackgroundTaskDescriptor {
    let retryUnixNs = wallClockNowNs &+ backgroundRetryDelayNs
    let earliestBeginUnixNs: UInt64
    if record.descriptor.schedule.isRecurring {
      let regularDescriptor = scheduledDescriptor(
        record,
        wallClockNowNs: wallClockNowNs
      )

      earliestBeginUnixNs = min(
        retryUnixNs,
        regularDescriptor.schedule.earliestBeginUnixNs ?? retryUnixNs
      )
    } else {
      earliestBeginUnixNs = retryUnixNs
    }
    let descriptor = record.descriptor

    return RuntimeHostBackgroundTaskDescriptor(
      identifier: descriptor.identifier,
      trigger: descriptor.trigger,
      schedule: RuntimeHostBackgroundTaskSchedule(
        kind: descriptor.schedule.kind,
        earliestBeginUnixNs: earliestBeginUnixNs,
        repeatIntervalNs: descriptor.schedule.repeatIntervalNs
      ),
      network: descriptor.network,
      requiresCharging: descriptor.requiresCharging,
      requiresIdle: descriptor.requiresIdle,
      conflictPolicy: descriptor.conflictPolicy
    )
  }

  /// Return the first regular run target for one declared schedule.
  nonisolated static func initialRegularRunUnixNs(
    _ schedule: RuntimeHostBackgroundTaskSchedule,
    wallClockNowNs: UInt64
  ) -> UInt64? {
    switch schedule.kind {
    case .once:
      nil
    case .recurring:
      max(schedule.earliestBeginUnixNs ?? wallClockNowNs, wallClockNowNs)
    }
  }

  /// Return one record whose recurring schedule is ready for submission.
  nonisolated static func resolvedRecordForScheduling(
    _ record: BackgroundTaskRecord,
    wallClockNowNs: UInt64
  ) -> BackgroundTaskRecord {
    guard record.descriptor.schedule.isRecurring else {
      return record
    }

    guard let repeatIntervalNs = record.descriptor.schedule.repeatIntervalNs else {
      return record
    }

    guard
      var resolvedRunAtUnixNs = record.nextRegularRunUnixNs
        ?? initialRegularRunUnixNs(record.descriptor.schedule, wallClockNowNs: wallClockNowNs)
    else {
      return record
    }

    while resolvedRunAtUnixNs < wallClockNowNs {
      resolvedRunAtUnixNs &+= repeatIntervalNs
    }

    guard resolvedRunAtUnixNs != record.nextRegularRunUnixNs else {
      return record
    }

    return BackgroundTaskRecord(
      descriptor: record.descriptor,
      nextRegularRunUnixNs: resolvedRunAtUnixNs
    )
  }

  /// Return one record advanced past the recurring execution that just started.
  nonisolated static func advanceRecordAfterRegularLaunch(
    _ record: BackgroundTaskRecord,
    wallClockNowNs: UInt64
  ) -> BackgroundTaskRecord {
    guard record.descriptor.schedule.isRecurring else {
      return record
    }

    guard let repeatIntervalNs = record.descriptor.schedule.repeatIntervalNs else {
      return record
    }

    guard
      var nextRunAtUnixNs = record.nextRegularRunUnixNs
        ?? initialRegularRunUnixNs(record.descriptor.schedule, wallClockNowNs: wallClockNowNs)
    else {
      return record
    }

    nextRunAtUnixNs &+= repeatIntervalNs

    while nextRunAtUnixNs <= wallClockNowNs {
      nextRunAtUnixNs &+= repeatIntervalNs
    }

    return BackgroundTaskRecord(
      descriptor: record.descriptor,
      nextRegularRunUnixNs: nextRunAtUnixNs
    )
  }

  /// Read the Apple background scheduler status.
  public func backgroundStatus() -> RuntimeHostBackgroundStatusResponse {
    RuntimeHostBackgroundStatusResponse(
      status: hostStatusOk,
      schedulerStatus: scheduler.schedulerStatus()
    )
  }

  /// List registered Apple background tasks.
  public func listBackgroundTasks() -> RuntimeHostBackgroundTaskListResponse {
    RuntimeHostBackgroundTaskListResponse(
      status: hostStatusOk,
      descriptors: registrations.listDescriptors()
    )
  }

  /// Register one Apple background task.
  public func registerBackgroundTask(
    _ options: RuntimeHostBackgroundTaskOptions
  ) -> UInt32 {
    // reject invalid registration payloads before mutating host state
    let validationStatus = validateOptions(options)
    guard validationStatus == hostStatusOk else {
      return validationStatus
    }

    // build the stable descriptor view
    let descriptor = RuntimeHostBackgroundTaskDescriptor(
      identifier: options.identifier,
      trigger: options.trigger,
      schedule: options.schedule,
      network: options.network,
      requiresCharging: options.requiresCharging,
      requiresIdle: options.requiresIdle,
      conflictPolicy: options.conflictPolicy
    )

    // ensure the launch handler exists before storing the registration
    let registrationStatus = ensureLaunchHandler(for: descriptor)
    guard registrationStatus == hostStatusOk else {
      return registrationStatus
    }

    // persist the registration and schedule the next run when needed
    let storedRegistration = registrations.putRegistration(
      descriptor,
      nextRegularRunUnixNs: Self.initialRegularRunUnixNs(
        descriptor.schedule,
        wallClockNowNs: wallClockNowNs()
      )
    )
    if !storedRegistration.shouldSchedule {
      return hostStatusOk
    }

    return schedule(storedRegistration.record)
  }

  /// Unregister one Apple background task.
  public func unregisterBackgroundTask(
    _ identifier: String
  ) -> UInt32 {
    // reject blank task identifiers
    guard !identifier.isEmpty else {
      return hostStatusInvalidArgument
    }

    // remove the durable registration
    guard registrations.removeRegistration(identifier) != nil else {
      return hostStatusNotFound
    }

    // cancel any pending scheduler request
    scheduler.cancel(identifier: identifier)

    // finish active executions for this task
    let activeExecutions = executions.removeExecutions(identifier: identifier)
    for execution in activeExecutions {
      execution.task?.setTaskCompleted(success: false)
    }

    return hostStatusOk
  }

  /// Trigger one Apple background task for testing.
  public func triggerBackgroundTask(
    _ identifier: String
  ) -> RuntimeHostBackgroundTriggerResponse {
    // reject blank task identifiers
    guard !identifier.isEmpty else {
      return RuntimeHostBackgroundTriggerResponse(status: hostStatusInvalidArgument)
    }

    // require one registered descriptor
    guard registrations.resolveDescriptor(identifier) != nil else {
      return RuntimeHostBackgroundTriggerResponse(status: hostStatusNotFound)
    }

    // create one synthetic execution
    let execution = executions.createExecution(
      identifier: identifier,
      task: nil
    )

    // publish the ready event immediately
    publishEvent(
      kind: .taskReady,
      execution: execution
    )

    return RuntimeHostBackgroundTriggerResponse(
      status: hostStatusOk,
      isTriggered: true
    )
  }

  /// Complete one Apple background task execution.
  public func completeBackgroundTask(
    executionID: String,
    result: RuntimeHostBackgroundTaskResult
  ) -> UInt32 {
    // remove the active execution first
    guard let execution = executions.removeExecution(executionID) else {
      return hostStatusNotFound
    }

    // complete the scheduler task with the runtime result
    execution.task?.setTaskCompleted(success: result == .success)

    // resolve the next registration action for this completion
    let record: BackgroundTaskRecord?
    switch result {
    // keep the registration so the host can retry this execution
    case .retry:
      record = registrations.resolveRecord(execution.identifier)

    // otherwise remove one-shot registrations and keep recurring ones
    case .success, .failure:
      if let currentRecord = registrations.resolveRecord(execution.identifier) {
        if currentRecord.descriptor.schedule.isRecurring {
          record = currentRecord
        } else {
          _ = registrations.removeRegistration(execution.identifier)
          record = nil
        }
      } else {
        record = nil
      }
    }

    // schedule the next run when one registration remains active
    if let record {
      switch result {
      case .success, .failure:
        _ = schedule(record)
      case .retry:
        _ = scheduleRetry(record)
      }
    }

    return hostStatusOk
  }

  /// Install one launch handler for the descriptor identifier when needed.
  private func ensureLaunchHandler(
    for descriptor: RuntimeHostBackgroundTaskDescriptor
  ) -> UInt32 {
    // reuse the existing handler registration when present
    if registrations.isHandlerRegistered(descriptor.identifier) {
      return hostStatusOk
    }

    // register the launch handler with the platform scheduler
    let status = scheduler.register(
      identifier: descriptor.identifier,
      trigger: descriptor.trigger
    ) { [weak self] task in
      self?.handleLaunch(task: task)
    }
    guard status == hostStatusOk else {
      return status
    }

    // remember that the handler registration succeeded
    registrations.markHandlerRegistered(descriptor.identifier)

    return hostStatusOk
  }

  /// Handle one scheduler-delivered task launch.
  private func handleLaunch(
    task: any IOSBackgroundRunningTask
  ) {
    if let record = registrations.resolveRecord(task.identifier) {
      let advancedRecord = Self.advanceRecordAfterRegularLaunch(
        record,
        wallClockNowNs: wallClockNowNs()
      )

      if advancedRecord.nextRegularRunUnixNs != record.nextRegularRunUnixNs {
        registrations.putRecord(advancedRecord)
      }
    }

    // create one active execution for the launched task
    let execution = executions.createExecution(
      identifier: task.identifier,
      task: task
    )

    // route platform expiration back into the execution store
    task.expirationHandler = { [weak self] in
      Task { @MainActor in
        self?.expireExecution(executionID: execution.executionID)
      }
    }

    // publish the ready event after the execution exists
    publishEvent(
      kind: .taskReady,
      execution: execution
    )
  }

  /// Mark one active execution as expired and schedule the next run.
  private func expireExecution(
    executionID: String
  ) {
    // remove the active execution first
    guard let execution = executions.removeExecution(executionID) else {
      return
    }

    // complete the scheduler task and publish expiration
    execution.task?.setTaskCompleted(success: false)
    publishEvent(
      kind: .taskExpired,
      execution: execution
    )

    // schedule the next run when the task remains registered
    if let record = registrations.resolveRecord(execution.identifier) {
      _ = schedule(record)
    }
  }

  /// Publish one background event into the runtime ingress surface.
  private func publishEvent(
    kind: RuntimeHostBackgroundEventKind,
    execution: BackgroundExecution
  ) {
    // build the runtime event payload
    let event = executions.event(
      kind: kind,
      execution: execution
    )

    // deliver the event into the ingress surface
    events.sendBackgroundEvent(event)
  }

  /// Schedule one descriptor through the system scheduler.
  private func schedule(
    _ record: BackgroundTaskRecord
  ) -> UInt32 {
    // resolve and persist the next regular submission time
    let record = Self.resolvedRecordForScheduling(
      record,
      wallClockNowNs: wallClockNowNs()
    )
    registrations.putRecord(record)
    let descriptor = Self.scheduledDescriptor(
      record,
      wallClockNowNs: wallClockNowNs()
    )

    return scheduler.submit(descriptor: descriptor)
  }

  /// Schedule one descriptor for one earlier retry attempt.
  private func scheduleRetry(
    _ record: BackgroundTaskRecord
  ) -> UInt32 {
    let descriptor = Self.retriedDescriptor(
      record,
      wallClockNowNs: wallClockNowNs()
    )

    return scheduler.submit(descriptor: descriptor)
  }

  /// Validate one task registration payload against the iOS background model.
  private func validateOptions(
    _ options: RuntimeHostBackgroundTaskOptions
  ) -> UInt32 {
    // reject blank task identifiers
    if options.identifier.isEmpty {
      return hostStatusInvalidArgument
    }

    // require valid schedule payloads for the declared schedule class
    switch options.schedule.kind {
    case .once:
      if options.schedule.repeatIntervalNs != nil {
        return hostStatusInvalidArgument
      }
    case .recurring:
      guard let repeatIntervalNs = options.schedule.repeatIntervalNs else {
        return hostStatusInvalidArgument
      }

      if repeatIntervalNs == 0 {
        return hostStatusInvalidArgument
      }
    }

    // reject app-refresh constraints that the platform cannot enforce
    if options.trigger == .appRefresh {
      if options.network != .none {
        return hostStatusNotSupported
      }

      if options.requiresCharging {
        return hostStatusNotSupported
      }

      if options.requiresIdle {
        return hostStatusNotSupported
      }
    }

    // reject the unmetered network requirement because the Apple scheduler does not expose it
    if options.network == .unmetered {
      return hostStatusNotSupported
    }

    // reject the idle requirement because the Apple scheduler does not expose it
    if options.requiresIdle {
      return hostStatusNotSupported
    }

    // reject requests when the scheduler is unavailable
    if scheduler.schedulerStatus() == .unavailable {
      return hostStatusNotSupported
    }

    return hostStatusOk
  }
}
