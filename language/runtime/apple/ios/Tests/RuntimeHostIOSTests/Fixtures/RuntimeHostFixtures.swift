import RuntimeHostAppleCore
import RuntimeHostIOS

/// Create one iOS runtime host fixture.
@MainActor
func createIOSRuntimeHost(
  lifecycleEvents: any LifecycleEvents = IOSRecordingLifecycleSink(),
  permissionRequests: any PermissionRequests = IOSNoopPermissionRequestHandler(),
  permissionEvents: any PermissionEvents = IOSNoopPermissionEventSink(),
  documentRequests: any DocumentRequests = IOSNoopDocumentRequestHandler(),
  documentEvents: any DocumentEvents = IOSRecordingDocumentEventSink(),
  contactRequests: any ContactRequests = IOSNoopContactRequestHandler(),
  calendarRequests: any CalendarRequests = UnsupportedCalendarRequests(),
  intentRequests: any IntentRequests = IOSNoopIntentRequestHandler(),
  intentEvents: any IntentEvents = IOSRecordingIntentEventSink(),
  locationRequests: any LocationRequests = IOSRecordingLocationRequestHandler(),
  locationEvents: any LocationEvents = IOSNoopLocationEventSink(),
  mediaRequests: any MediaRequests = IOSNoopMediaRequestHandler(),
  notificationRequests: any NotificationRequests = IOSNoopNotificationRequestHandler(),
  notificationEvents: any NotificationEvents = IOSRecordingNotificationEventSink(),
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
