import RuntimeHostAppleCore
import RuntimeHostMacOS

/// Create one macOS runtime host fixture.
@MainActor
func createMacOSRuntimeHost(
  lifecycleEvents: any LifecycleEvents = MacOSRecordingLifecycleSink(),
  permissionRequests: any PermissionRequests = MacOSNoopPermissionRequestHandler(),
  permissionEvents: any PermissionEvents = MacOSNoopPermissionEventSink(),
  documentRequests: any DocumentRequests = MacOSNoopDocumentRequestHandler(),
  documentEvents: any DocumentEvents = MacOSRecordingDocumentEventSink(),
  contactRequests: any ContactRequests = MacOSNoopContactRequestHandler(),
  calendarRequests: any CalendarRequests = UnsupportedCalendarRequests(),
  intentRequests: any IntentRequests = MacOSNoopIntentRequestHandler(),
  intentEvents: any IntentEvents = MacOSRecordingIntentEventSink(),
  locationRequests: any LocationRequests = UnsupportedLocationRequests(),
  locationEvents: any LocationEvents = MacOSNoopLocationEventSink(),
  mediaRequests: any MediaRequests = MacOSNoopMediaRequestHandler(),
  notificationRequests: any NotificationRequests = MacOSNoopNotificationRequestHandler(),
  notificationEvents: any NotificationEvents = MacOSRecordingNotificationEventSink(),
  surfaceKind: RendererSurfaceKind = .metalLayer
) -> RuntimeHost {
  RuntimeHost(
    sessionHandle: HostSessionHandle(rawValue: 7),
    embedderID: HostEmbedderID(rawValue: 11),
    lifecycle: LifecycleHost(events: lifecycleEvents),
    permission: PermissionHost(
      requests: permissionRequests,
      events: permissionEvents
    ),
    document: DocumentHost(
      requests: documentRequests,
      events: documentEvents
    ),
    contact: ContactHost(requests: contactRequests),
    calendar: CalendarHost(requests: calendarRequests),
    intent: IntentHost(
      requests: intentRequests,
      events: intentEvents
    ),
    location: LocationHost(
      requests: locationRequests,
      events: locationEvents
    ),
    media: MediaHost(requests: mediaRequests),
    notification: NotificationHost(
      requests: notificationRequests,
      events: notificationEvents
    ),
    rendererSurface: RendererSurface(
      kind: surfaceKind,
      identifier: "main-surface"
    )
  )
}
