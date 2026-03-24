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
        lifecycleEvents: lifecycleEvents,
        permissionRequests: permissionRequests,
        permissionEvents: permissionEvents,
        documentRequests: documentRequests,
        documentEvents: documentEvents,
        contactRequests: contactRequests,
        calendarRequests: calendarRequests,
        intentRequests: intentRequests,
        intentEvents: intentEvents,
        locationRequests: locationRequests,
        locationEvents: locationEvents,
        mediaRequests: mediaRequests,
        notificationRequests: notificationRequests,
        notificationEvents: notificationEvents,
        rendererSurface: RendererSurface(
            kind: surfaceKind,
            identifier: "main-surface"
        )
    )
}
