import Foundation

/// The Apple runtime host for one attached Destack runtime session.
@MainActor
public final class RuntimeHost {
  /// The attached host session handle.
  public let sessionHandle: HostSessionHandle
  /// The stable embedder identifier for this embedder.
  public let embedderID: HostEmbedderID
  /// The lifecycle ingress surface for this embedder.
  public let lifecycleEvents: any LifecycleEvents
  /// The permission request surface for this embedder.
  public let permissionRequests: any PermissionRequests
  /// The permission ingress surface for this embedder.
  public let permissionEvents: any PermissionEvents
  /// The document request surface for this embedder.
  public let documentRequests: any DocumentRequests
  /// The document result surface for this embedder.
  public let documentEvents: any DocumentEvents
  /// The text-input ingress surface for this embedder.
  public let textInputEvents: any TextInputEvents
  /// The background ingress surface for this embedder.
  public let backgroundEvents: any BackgroundEvents
  /// The contact request surface for this embedder.
  public let contactRequests: any ContactRequests
  /// The calendar request surface for this embedder.
  public let calendarRequests: any CalendarRequests
  /// The intent request surface for this embedder.
  public let intentRequests: any IntentRequests
  /// The intent ingress surface for this embedder.
  public let intentEvents: any IntentEvents
  /// The location request surface for this embedder.
  public let locationRequests: any LocationRequests
  /// The location ingress surface for this embedder.
  public let locationEvents: any LocationEvents
  /// The media request surface for this embedder.
  public let mediaRequests: any MediaRequests
  /// The notification request surface for this embedder.
  public let notificationRequests: any NotificationRequests
  /// The notification ingress surface for this embedder.
  public let notificationEvents: any NotificationEvents
  /// The text-input request surface for this embedder.
  public var textInputRequests: any TextInputRequests
  /// The background request surface for this embedder.
  public var backgroundRequests: any BackgroundRequests
  /// The primary renderer surface for this embedder.
  public private(set) var rendererSurface: RendererSurface

  /// Create one Apple runtime host for one session and one primary renderer surface.
  public init(
    sessionHandle: HostSessionHandle,
    embedderID: HostEmbedderID,
    lifecycleEvents: any LifecycleEvents,
    permissionRequests: any PermissionRequests,
    permissionEvents: any PermissionEvents,
    documentRequests: any DocumentRequests,
    documentEvents: any DocumentEvents,
    textInputEvents: any TextInputEvents = NoopTextInputEvents(),
    backgroundEvents: any BackgroundEvents = NoopBackgroundEvents(),
    contactRequests: any ContactRequests,
    calendarRequests: any CalendarRequests,
    intentRequests: any IntentRequests,
    intentEvents: any IntentEvents,
    locationRequests: any LocationRequests,
    locationEvents: any LocationEvents,
    mediaRequests: any MediaRequests,
    notificationRequests: any NotificationRequests,
    notificationEvents: any NotificationEvents,
    rendererSurface: RendererSurface
  ) {
    self.sessionHandle = sessionHandle
    self.embedderID = embedderID
    self.lifecycleEvents = lifecycleEvents
    self.permissionRequests = permissionRequests
    self.permissionEvents = permissionEvents
    self.documentRequests = documentRequests
    self.documentEvents = documentEvents
    self.textInputEvents = textInputEvents
    self.backgroundEvents = backgroundEvents
    self.textInputRequests = UnsupportedTextInputRequests()
    self.backgroundRequests = UnsupportedBackgroundRequests()
    self.contactRequests = contactRequests
    self.calendarRequests = calendarRequests
    self.intentRequests = intentRequests
    self.intentEvents = intentEvents
    self.locationRequests = locationRequests
    self.locationEvents = locationEvents
    self.mediaRequests = mediaRequests
    self.notificationRequests = notificationRequests
    self.notificationEvents = notificationEvents
    self.rendererSurface = rendererSurface
  }

  /// Replace the primary renderer surface for this host embedder.
  func updateRendererSurface(
    _ rendererSurface: RendererSurface
  ) {
    self.rendererSurface = rendererSurface
  }
}
