import Foundation

/// The Apple runtime host for one attached Destack runtime session.
@MainActor
public final class RuntimeHost {
  /// The attached host session handle.
  public let sessionHandle: HostSessionHandle
  /// The stable embedder identifier for this embedder.
  public let embedderID: HostEmbedderID
  /// The lifecycle host surface for this embedder.
  public let lifecycle: LifecycleHost
  /// The permission host surface for this embedder.
  public let permission: PermissionHost
  /// The document host surface for this embedder.
  public let document: DocumentHost
  /// The text host surface for this embedder.
  public let text: TextHost
  /// The background host surface for this embedder.
  public let background: BackgroundHost
  /// The contact host surface for this embedder.
  public let contact: ContactHost
  /// The calendar host surface for this embedder.
  public let calendar: CalendarHost
  /// The intent host surface for this embedder.
  public let intent: IntentHost
  /// The location host surface for this embedder.
  public let location: LocationHost
  /// The media host surface for this embedder.
  public let media: MediaHost
  /// The notification host surface for this embedder.
  public let notification: NotificationHost
  /// The primary renderer surface for this embedder.
  public private(set) var rendererSurface: RendererSurface

  /// Create one Apple runtime host for one session and one primary renderer surface.
  public init(
    sessionHandle: HostSessionHandle,
    embedderID: HostEmbedderID,
    lifecycle: LifecycleHost,
    permission: PermissionHost,
    document: DocumentHost,
    text: TextHost = TextHost(
      requests: UnsupportedTextRequests(),
      events: NoopTextEvents()
    ),
    background: BackgroundHost = BackgroundHost(
      requests: UnsupportedBackgroundRequests(),
      events: NoopBackgroundEvents()
    ),
    contact: ContactHost,
    calendar: CalendarHost,
    intent: IntentHost,
    location: LocationHost,
    media: MediaHost,
    notification: NotificationHost,
    rendererSurface: RendererSurface
  ) {
    self.sessionHandle = sessionHandle
    self.embedderID = embedderID
    self.lifecycle = lifecycle
    self.permission = permission
    self.document = document
    self.text = text
    self.background = background
    self.contact = contact
    self.calendar = calendar
    self.intent = intent
    self.location = location
    self.media = media
    self.notification = notification
    self.rendererSurface = rendererSurface
  }

  /// Replace the primary renderer surface for this host embedder.
  func updateRendererSurface(
    _ rendererSurface: RendererSurface
  ) {
    self.rendererSurface = rendererSurface
  }

  /// Replace the text request surface for this host.
  public func updateTextRequests(
    _ requests: any TextRequests
  ) {
    text.updateRequests(requests)
  }

  /// Replace the background request surface for this host.
  public func updateBackgroundRequests(
    _ requests: any BackgroundRequests
  ) {
    background.updateRequests(requests)
  }
}
