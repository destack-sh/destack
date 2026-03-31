import Foundation

/// The lifecycle host surface attached to one Apple runtime host.
@MainActor
public final class LifecycleHost: LifecycleEvents {
  private let events: any LifecycleEvents

  /// Create one lifecycle host surface.
  public init(
    events: any LifecycleEvents
  ) {
    self.events = events
  }

  /// Send one lifecycle event into the attached runtime session.
  public func sendLifecycleEvent(_ event: RuntimeHostLifecycleEvent) {
    events.sendLifecycleEvent(event)
  }
}

/// The background host surface attached to one Apple runtime host.
@MainActor
public final class BackgroundHost: BackgroundRequests, BackgroundEvents {
  private var requests: any BackgroundRequests
  private let events: any BackgroundEvents

  /// Create one background host surface.
  public init(
    requests: any BackgroundRequests,
    events: any BackgroundEvents
  ) {
    self.requests = requests
    self.events = events
  }

  /// Replace the request surface for this host.
  func updateRequests(
    _ requests: any BackgroundRequests
  ) {
    self.requests = requests
  }

  /// Read the Apple background scheduler status.
  public func status() -> RuntimeHostBackgroundStatusResponse {
    requests.status()
  }

  /// List registered Apple background tasks.
  public func list() -> RuntimeHostBackgroundListResponse {
    requests.list()
  }

  /// Register one Apple background task.
  public func registerTask(
    _ request: RuntimeHostBackgroundTaskOptions
  ) -> UInt32 {
    requests.registerTask(request)
  }

  /// Unregister one Apple background task.
  public func unregister(
    _ request: RuntimeHostBackgroundUnregisterRequest
  ) -> UInt32 {
    requests.unregister(request)
  }

  /// Trigger one Apple background task for testing.
  public func triggerTest(
    _ request: RuntimeHostBackgroundTriggerTestRequest
  ) -> RuntimeHostBackgroundTriggerTestResponse {
    requests.triggerTest(request)
  }

  /// Complete one Apple background task execution.
  public func complete(
    _ request: RuntimeHostBackgroundCompleteRequest
  ) -> UInt32 {
    requests.complete(request)
  }

  /// Send one host background event into the attached runtime session.
  public func notifyBackgroundEvent(_ event: RuntimeHostBackgroundEvent) {
    events.notifyBackgroundEvent(event)
  }
}

/// The text host surface attached to one Apple runtime host.
@MainActor
public final class TextHost: TextRequests, TextEvents {
  private var requests: any TextRequests
  private let events: any TextEvents

  /// Create one text-input host surface.
  public init(
    requests: any TextRequests,
    events: any TextEvents
  ) {
    self.requests = requests
    self.events = events
  }

  /// Replace the request surface for this host.
  func updateRequests(
    _ requests: any TextRequests
  ) {
    self.requests = requests
  }

  /// Open one text session through one attached host.
  public func open(
    _ request: RuntimeHostTextInputOpenRequest
  ) -> UInt32 {
    requests.open(request)
  }

  /// Close one text session through one attached host.
  public func close(
    _ request: RuntimeHostTextInputCloseRequest
  ) -> UInt32 {
    requests.close(request)
  }

  /// Update one text geometry payload through one attached host.
  public func setGeometry(
    _ request: RuntimeHostTextInputGeometryRequest
  ) -> UInt32 {
    requests.setGeometry(request)
  }

  /// Update one text state payload through one attached host.
  public func setState(
    _ request: RuntimeHostTextInputStateRequest
  ) -> UInt32 {
    requests.setState(request)
  }

  /// Deliver one text-session state event into one runtime session.
  public func notifyTextInputState(_ event: RuntimeHostTextInputEvent) {
    events.notifyTextInputState(event)
  }
}
