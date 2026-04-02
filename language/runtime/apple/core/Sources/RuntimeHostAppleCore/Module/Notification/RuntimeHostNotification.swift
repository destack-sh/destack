import Foundation

/// The Apple notification priority payload.
public enum RuntimeHostNotificationPriority: UInt32, Sendable, Codable {
  /// One low-priority notification.
  case low = 1
  /// One normal-priority notification.
  case normal = 2
  /// One high-priority notification.
  case high = 3
}

/// One Apple notification calendar trigger payload.
public struct RuntimeHostNotificationCalendarTrigger: Sendable, Hashable, Codable {
  /// The trigger year.
  public let year: UInt16
  /// The trigger month, 1 to 12.
  public let month: UInt8
  /// The trigger day of month, 1 to 31.
  public let day: UInt8
  /// The trigger hour, 0 to 23.
  public let hour: UInt8
  /// The trigger minute, 0 to 59.
  public let minute: UInt8
  /// The trigger second, 0 to 59.
  public let second: UInt8
  /// The trigger timezone identifier.
  public let timeZone: String
  /// Whether this trigger repeats.
  public let repeats: Bool

  /// Create one calendar trigger payload.
  public init(
    year: UInt16 = 0,
    month: UInt8 = 0,
    day: UInt8 = 0,
    hour: UInt8 = 0,
    minute: UInt8 = 0,
    second: UInt8 = 0,
    timeZone: String = "",
    repeats: Bool = false
  ) {
    self.year = year
    self.month = month
    self.day = day
    self.hour = hour
    self.minute = minute
    self.second = second
    self.timeZone = timeZone
    self.repeats = repeats
  }
}

/// The Apple notification trigger-kind payload.
public enum RuntimeHostNotificationTriggerKind: UInt32, Sendable, Codable {
  /// One calendar-date trigger.
  case calendarDate = 1
  /// One immediate trigger.
  case immediate = 2
  /// One time-interval trigger.
  case timeInterval = 3
}

/// One Apple notification trigger payload.
public struct RuntimeHostNotificationTrigger: Sendable, Hashable, Codable {
  /// The trigger variant kind.
  public let kind: RuntimeHostNotificationTriggerKind
  /// The calendar trigger payload.
  public let calendar: RuntimeHostNotificationCalendarTrigger
  /// The time-interval trigger delay in nanoseconds.
  public let intervalNs: UInt64

  /// Create one trigger payload.
  public init(
    kind: RuntimeHostNotificationTriggerKind = .immediate,
    calendar: RuntimeHostNotificationCalendarTrigger =
      RuntimeHostNotificationCalendarTrigger(),
    intervalNs: UInt64 = 0
  ) {
    self.kind = kind
    self.calendar = calendar
    self.intervalNs = intervalNs
  }
}

/// One Apple notification request submitted by one runtime session.
public struct RuntimeHostNotificationRequest: Sendable, Hashable, Codable {
  /// The primary notification title.
  public let title: String
  /// The subtitle when present.
  public let subtitle: String?
  /// The primary notification body text.
  public let body: String
  /// The host notification tag.
  public let tag: String
  /// The channel identifier when present.
  public let channelId: String?
  /// The priority class.
  public let priority: RuntimeHostNotificationPriority
  /// The badge count when present.
  public let badgeCount: UInt32?
  /// The sound identifier when present.
  public let sound: String?
  /// The category identifier when present.
  public let categoryId: String?
  /// The thread identifier when present.
  public let threadId: String?
  /// The delivery trigger selector.
  public let trigger: RuntimeHostNotificationTrigger
  /// The action identifier when present.
  public let actionId: String?

  /// The stable runtime notification identifier.
  public var identifier: String {
    tag
  }

  /// The channel identifier when present.
  public var channelID: String? {
    channelId
  }

  /// The category identifier when present.
  public var categoryID: String? {
    categoryId
  }

  /// The thread identifier when present.
  public var threadID: String? {
    threadId
  }

  /// The action identifier when present.
  public var actionID: String? {
    actionId
  }

  /// Create one notification request.
  public init(
    title: String,
    subtitle: String? = nil,
    body: String,
    tag: String,
    channelId: String? = nil,
    priority: RuntimeHostNotificationPriority = .normal,
    badgeCount: UInt32? = nil,
    sound: String? = nil,
    categoryId: String? = nil,
    threadId: String? = nil,
    trigger: RuntimeHostNotificationTrigger = RuntimeHostNotificationTrigger(),
    actionId: String? = nil
  ) {
    self.title = title
    self.subtitle = subtitle
    self.body = body
    self.tag = tag
    self.channelId = channelId
    self.priority = priority
    self.badgeCount = badgeCount
    self.sound = sound
    self.categoryId = categoryId
    self.threadId = threadId
    self.trigger = trigger
    self.actionId = actionId
  }

  /// Create one notification request with `ID`-suffixed labels.
  public init(
    title: String,
    subtitle: String?,
    body: String,
    tag: String,
    channelID: String?,
    priority: RuntimeHostNotificationPriority,
    badgeCount: UInt32?,
    sound: String?,
    categoryID: String?,
    threadID: String?,
    trigger: RuntimeHostNotificationTrigger,
    actionID: String?
  ) {
    self.init(
      title: title,
      subtitle: subtitle,
      body: body,
      tag: tag,
      channelId: channelID,
      priority: priority,
      badgeCount: badgeCount,
      sound: sound,
      categoryId: categoryID,
      threadId: threadID,
      trigger: trigger,
      actionId: actionID
    )
  }
}

/// One Apple notification event metadata payload.
public struct RuntimeHostNotificationEventMetadata: Sendable, Hashable, Codable {
  /// The monotonic event timestamp in nanoseconds.
  public let timestampNs: UInt64
  /// The monotonic sequence number for this event stream.
  public let sequence: UInt64
  /// The host notification identifier.
  public let id: String
  /// The notification request payload.
  public let request: RuntimeHostNotificationRequest

  /// Create one notification event metadata payload.
  public init(
    timestampNs: UInt64 = 0,
    sequence: UInt64 = 0,
    id: String,
    request: RuntimeHostNotificationRequest
  ) {
    self.timestampNs = timestampNs
    self.sequence = sequence
    self.id = id
    self.request = request
  }
}

/// One Apple notification interacted payload.
public struct RuntimeHostNotificationInteractedPayload: Sendable, Hashable, Codable {
  /// The action identifier when present.
  public let actionId: String?
  /// The text-input response when present.
  public let actionResponseText: String?

  /// Create one interacted payload.
  public init(
    actionId: String? = nil,
    actionResponseText: String? = nil
  ) {
    self.actionId = actionId
    self.actionResponseText = actionResponseText
  }

  /// The action identifier when present.
  public var actionID: String? {
    actionId
  }

  /// Create one interacted payload with one `ID`-suffixed label.
  public init(
    actionID: String?,
    actionResponseText: String?
  ) {
    self.init(
      actionId: actionID,
      actionResponseText: actionResponseText
    )
  }
}

/// The Apple notification interaction delivered into one runtime session.
public enum RuntimeHostNotificationEventKind: UInt32, Sendable, Codable {
  /// The notification was delivered to the Apple host surface.
  case delivered = 1
  /// The notification was dismissed by the user or host.
  case dismissed = 2
  /// The notification was interacted with by the user.
  case interacted = 3
}

/// One Apple notification ingress event delivered into one runtime session.
public struct RuntimeHostNotificationEvent: Sendable, Hashable, Codable {
  /// The notification interaction kind.
  public let kind: RuntimeHostNotificationEventKind
  /// The shared event metadata.
  public let metadata: RuntimeHostNotificationEventMetadata
  /// The interaction payload.
  public let payload: RuntimeHostNotificationInteractedPayload

  /// The stable runtime notification identifier.
  public var identifier: String {
    metadata.id
  }

  /// The request associated with this notification event.
  public var request: RuntimeHostNotificationRequest {
    metadata.request
  }

  /// The per-session ingress sequence number.
  public var sequence: UInt64 {
    metadata.sequence
  }

  /// The monotonic ingress timestamp in nanoseconds.
  public var timestampNs: UInt64 {
    metadata.timestampNs
  }

  /// The optional action identifier for interactive notifications.
  public var actionIdentifier: String? {
    payload.actionId
  }

  /// Create one notification event.
  public init(
    kind: RuntimeHostNotificationEventKind,
    metadata: RuntimeHostNotificationEventMetadata,
    payload: RuntimeHostNotificationInteractedPayload =
      RuntimeHostNotificationInteractedPayload()
  ) {
    self.kind = kind
    self.metadata = metadata
    self.payload = payload
  }
}
