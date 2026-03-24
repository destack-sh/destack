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
public enum RuntimeHostNotificationEventKind: String, Sendable, Codable {
  /// The notification was delivered to the Apple host surface.
  case delivered
  /// The notification was activated by the user.
  case activated
  /// The notification was dismissed by the user or host.
  case dismissed
}

/// One Apple notification ingress event delivered into one runtime session.
public struct RuntimeHostNotificationEvent: Sendable, Hashable, Codable {
  /// The stable runtime notification identifier.
  public let identifier: String
  /// The request associated with this notification event.
  public let request: RuntimeHostNotificationRequest
  /// The notification interaction kind.
  public let kind: RuntimeHostNotificationEventKind
  /// The optional action identifier for interactive notifications.
  public let actionIdentifier: String?

  /// Create one notification event.
  public init(
    identifier: String,
    request: RuntimeHostNotificationRequest,
    kind: RuntimeHostNotificationEventKind,
    actionIdentifier: String? = nil
  ) {
    self.identifier = identifier
    self.request = request
    self.kind = kind
    self.actionIdentifier = actionIdentifier
  }
}
