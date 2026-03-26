import Foundation
import RuntimeHostAppleCore

#if os(iOS) && canImport(BackgroundTasks)
  import BackgroundTasks
#endif

#if canImport(UIKit)
  import UIKit
#endif

/// One live iOS background task wrapper.
@MainActor
internal protocol IOSBackgroundRunningTask: AnyObject {
  /// The stable task identifier.
  var identifier: String { get }

  /// The host expiration callback invoked by the scheduler.
  var expirationHandler: (() -> Void)? { get set }

  /// Mark this task execution as complete.
  func setTaskCompleted(
    success: Bool
  )
}

/// One iOS scheduler backend for background tasks.
@MainActor
internal protocol BackgroundSchedulerBackend {
  /// Return the current scheduler status.
  func schedulerStatus() -> RuntimeHostBackgroundStatus

  /// Register one launch handler for one task identifier.
  func register(
    identifier: String,
    trigger: RuntimeHostBackgroundTriggerKind,
    launchHandler: @escaping (any IOSBackgroundRunningTask) -> Void
  ) -> UInt32

  /// Submit one scheduled request for one descriptor.
  func submit(
    descriptor: RuntimeHostBackgroundTaskDescriptor
  ) -> UInt32

  /// Cancel one pending request for one task identifier.
  func cancel(
    identifier: String
  )
}

#if os(iOS) && canImport(BackgroundTasks)
  /// One `BGTask` wrapper exposed through the scheduler abstraction.
  @MainActor
  private final class BackgroundRunningTaskAdapter: IOSBackgroundRunningTask {
    /// The wrapped task provided by the system scheduler.
    private let task: BGTask

    /// Create one wrapped scheduler task.
    init(
      task: BGTask
    ) {
      self.task = task
    }

    var identifier: String {
      task.identifier
    }

    var expirationHandler: (() -> Void)? {
      get {
        task.expirationHandler
      }
      set {
        task.expirationHandler = newValue
      }
    }

    /// Mark this task execution as complete.
    func setTaskCompleted(
      success: Bool
    ) {
      task.setTaskCompleted(success: success)
    }
  }

  /// One `BGTaskScheduler` backend.
  @MainActor
  internal final class BGTaskSchedulerBackend: BackgroundSchedulerBackend {
    /// The shared platform scheduler.
    private let scheduler = BGTaskScheduler.shared

    /// Return the current scheduler status.
    func schedulerStatus() -> RuntimeHostBackgroundStatus {
      #if canImport(UIKit)
        switch UIApplication.shared.backgroundRefreshStatus {
        case .available:
          return .available
        case .restricted:
          return .restricted
        case .denied:
          return .unavailable
        @unknown default:
          return .unavailable
        }
      #else
        return .available
      #endif
    }

    /// Register one launch handler for one task identifier.
    func register(
      identifier: String,
      trigger: RuntimeHostBackgroundTriggerKind,
      launchHandler: @escaping (any IOSBackgroundRunningTask) -> Void
    ) -> UInt32 {
      // register the task identifier with the platform scheduler
      let succeeded = scheduler.register(
        forTaskWithIdentifier: identifier,
        using: nil
      ) { task in
        launchHandler(
          BackgroundRunningTaskAdapter(task: task)
        )
      }

      return succeeded ? hostStatusOk : hostStatusFailed
    }

    /// Submit one scheduled request for one descriptor.
    func submit(
      descriptor: RuntimeHostBackgroundTaskDescriptor
    ) -> UInt32 {
      do {
        // build and submit the platform request
        let request = try schedulerRequest(descriptor)

        try scheduler.submit(request)

        return hostStatusOk
      } catch {
        return hostStatusFailed
      }
    }

    /// Cancel one pending request for one task identifier.
    func cancel(
      identifier: String
    ) {
      // cancel the pending platform request
      scheduler.cancel(taskRequestWithIdentifier: identifier)
    }

    /// Build one scheduler request for the descriptor trigger class.
    private func schedulerRequest(
      _ descriptor: RuntimeHostBackgroundTaskDescriptor
    ) throws -> BGTaskRequest {
      let request: BGTaskRequest

      // build the request class that matches the runtime trigger
      switch descriptor.trigger {
      case .appRefresh:
        let appRefreshRequest = BGAppRefreshTaskRequest(identifier: descriptor.identifier)
        request = appRefreshRequest
      case .processing:
        let processingRequest = BGProcessingTaskRequest(identifier: descriptor.identifier)
        processingRequest.requiresNetworkConnectivity =
          descriptor.network != .none
        processingRequest.requiresExternalPower = descriptor.requiresCharging
        request = processingRequest
      }

      // apply the earliest begin date when one was requested
      if let earliestBeginUnixNs = descriptor.schedule.earliestBeginUnixNs {
        let seconds = TimeInterval(earliestBeginUnixNs) / 1_000_000_000
        request.earliestBeginDate = Date(timeIntervalSince1970: seconds)
      }

      return request
    }
  }
#else
  /// One unavailable scheduler backend used when BackgroundTasks is not present.
  @MainActor
  internal final class BGTaskSchedulerBackend: BackgroundSchedulerBackend {
    /// Return the current scheduler status.
    func schedulerStatus() -> RuntimeHostBackgroundStatus {
      .unavailable
    }

    /// Register one launch handler for one task identifier.
    func register(
      identifier: String,
      trigger: RuntimeHostBackgroundTriggerKind,
      launchHandler: @escaping (any IOSBackgroundRunningTask) -> Void
    ) -> UInt32 {
      let _ = identifier
      let _ = trigger
      let _ = launchHandler

      return hostStatusNotSupported
    }

    /// Submit one scheduled request for one descriptor.
    func submit(
      descriptor: RuntimeHostBackgroundTaskDescriptor
    ) -> UInt32 {
      let _ = descriptor

      return hostStatusNotSupported
    }

    /// Cancel one pending request for one task identifier.
    func cancel(
      identifier: String
    ) {
      let _ = identifier
    }
  }
#endif
