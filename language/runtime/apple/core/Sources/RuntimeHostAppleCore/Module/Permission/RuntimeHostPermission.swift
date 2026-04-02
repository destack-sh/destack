import Foundation

/// One Apple permission selector.
public enum RuntimeHostPermission: Int32, Sendable, Codable {
  /// Location.
  case location = 1
  /// LocationBackground.
  case locationBackground = 2
  /// Camera.
  case camera = 3
  /// Microphone.
  case microphone = 4
  /// Bluetooth.
  case bluetooth = 5
  /// Notifications.
  case notifications = 6
  /// ContactsRead.
  case contactsRead = 7
  /// ContactsWrite.
  case contactsWrite = 8
  /// MediaRead.
  case mediaRead = 9
  /// MediaWrite.
  case mediaWrite = 10
  /// Motion.
  case motion = 11
  /// ClipboardRead.
  case clipboardRead = 12
  /// CalendarRead.
  case calendarRead = 13
  /// CalendarWrite.
  case calendarWrite = 14

  /// Return the canonical host permission token for this selector.
  public var hostName: String {
    switch self {
    case .location: "location"
    case .locationBackground: "locationBackground"
    case .camera: "camera"
    case .microphone: "microphone"
    case .bluetooth: "bluetooth"
    case .notifications: "notifications"
    case .contactsRead: "contactsRead"
    case .contactsWrite: "contactsWrite"
    case .mediaRead: "mediaRead"
    case .mediaWrite: "mediaWrite"
    case .motion: "motion"
    case .clipboardRead: "clipboardRead"
    case .calendarRead: "calendarRead"
    case .calendarWrite: "calendarWrite"
    }
  }

  /// Create one permission selector from one canonical host token.
  public init?(hostName: String) {
    switch hostName {
    case "location": self = .location
    case "locationBackground": self = .locationBackground
    case "camera": self = .camera
    case "microphone": self = .microphone
    case "bluetooth": self = .bluetooth
    case "notifications", "notification": self = .notifications
    case "contactsRead": self = .contactsRead
    case "contactsWrite": self = .contactsWrite
    case "mediaRead": self = .mediaRead
    case "mediaWrite": self = .mediaWrite
    case "motion": self = .motion
    case "clipboardRead": self = .clipboardRead
    case "calendarRead": self = .calendarRead
    case "calendarWrite": self = .calendarWrite
    default: return nil
    }
  }
}

/// One Apple permission request submitted by one runtime session.
public struct RuntimeHostPermissionRequest: Sendable, Hashable, Codable {
  /// The stable request identifier for this interactive host flow.
  public let requestID: HostRequestID
  /// The permission requested by the runtime.
  public let permission: RuntimeHostPermission

  /// Create one permission request.
  public init(
    requestID: HostRequestID,
    permission: RuntimeHostPermission
  ) {
    self.requestID = requestID
    self.permission = permission
  }

  /// Create one permission request from one canonical host token.
  public init(
    requestID: HostRequestID,
    permission: String
  ) {
    self.init(
      requestID: requestID,
      permission: requireRuntimeHostPermission(hostName: permission)
    )
  }
}

/// One Apple permission result event delivered into one runtime session.
public struct RuntimeHostPermissionEvent: Sendable, Hashable, Codable {
  /// The stable request identifier for this interactive host flow.
  public let requestID: HostRequestID
  /// The permission associated with this result.
  public let permission: RuntimeHostPermission
  /// Whether the Apple host granted the permission.
  public let isGranted: Bool

  /// Create one permission event.
  public init(
    requestID: HostRequestID,
    permission: RuntimeHostPermission,
    isGranted: Bool
  ) {
    self.requestID = requestID
    self.permission = permission
    self.isGranted = isGranted
  }

  /// Create one permission event from one canonical host token.
  public init(
    requestID: HostRequestID,
    permission: String,
    isGranted: Bool
  ) {
    self.init(
      requestID: requestID,
      permission: requireRuntimeHostPermission(hostName: permission),
      isGranted: isGranted
    )
  }
}

/// Compare one permission selector against one canonical host token.
public func == (left: RuntimeHostPermission, right: String) -> Bool {
  left.hostName == right
}

/// Compare one canonical host token against one permission selector.
public func == (left: String, right: RuntimeHostPermission) -> Bool {
  left == right.hostName
}

/// Resolve one permission selector from one canonical host token.
private func requireRuntimeHostPermission(hostName: String) -> RuntimeHostPermission {
  guard let permission = RuntimeHostPermission(hostName: hostName) else {
    preconditionFailure("invalid RuntimeHostPermission host token: \(hostName)")
  }

  return permission
}
