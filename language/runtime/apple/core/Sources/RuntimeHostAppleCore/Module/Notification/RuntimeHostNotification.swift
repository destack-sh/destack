import Foundation

/// One Apple notification request submitted by one runtime session.
public struct RuntimeHostNotificationRequest: Sendable, Hashable, Codable {
  /// The stable runtime notification identifier.
  public let identifier: String
  /// The primary notification title.
  public let title: String
  /// The primary notification body text.
  public let body: String

  /// Create one notification request.
  public init(
    identifier: String,
    title: String,
    body: String
  ) {
    self.identifier = identifier
    self.title = title
    self.body = body
  }
}

/// The Apple notification interaction delivered into one runtime session.
public enum RuntimeHostNotificationEventKind: UInt32, Sendable, Codable {
  /// The notification was delivered to the Apple host surface.
  case delivered = 1
  /// The notification was activated by the user.
  case activated = 2
  /// The notification was dismissed by the user or host.
  case dismissed = 3
}

/// One Apple notification ingress event delivered into one runtime session.
public struct RuntimeHostNotificationEvent: Sendable, Hashable, Codable {
  /// The stable runtime notification identifier.
  public let identifier: String
  /// The request associated with this notification event.
  public let request: RuntimeHostNotificationRequest
  /// The notification interaction kind.
  public let kind: RuntimeHostNotificationEventKind
  /// The per-session ingress sequence number.
  public let sequence: UInt64
  /// The monotonic ingress timestamp in nanoseconds.
  public let timestampNs: UInt64
  /// The optional action identifier for interactive notifications.
  public let actionIdentifier: String?

  /// Create one notification event.
  public init(
    identifier: String,
    request: RuntimeHostNotificationRequest,
    kind: RuntimeHostNotificationEventKind,
    actionIdentifier: String? = nil,
    sequence: UInt64 = 0,
    timestampNs: UInt64 = 0
  ) {
    self.identifier = identifier
    self.request = request
    self.kind = kind
    self.sequence = sequence
    self.timestampNs = timestampNs
    self.actionIdentifier = actionIdentifier
  }
}
