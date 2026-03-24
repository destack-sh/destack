import RuntimeHostAppleCore

/// Create one Apple core runtime host fixture.
@MainActor
func createRuntimeHost(
    lifecycleEvents: any LifecycleEvents = RecordingLifecycleSink(),
    permissionRequests: any PermissionRequests = RecordingPermissionRequestHandler(),
    permissionEvents: any PermissionEvents = RecordingPermissionEventSink(),
    documentRequests: any DocumentRequests = RecordingDocumentRequestHandler(),
    documentEvents: any DocumentEvents = RecordingDocumentEventSink(),
    contactRequests: any ContactRequests = UnsupportedContactRequests(),
    calendarRequests: any CalendarRequests = UnsupportedCalendarRequests(),
    intentRequests: any IntentRequests = RecordingIntentRequestHandler(),
    intentEvents: any IntentEvents = RecordingIntentEventSink(),
    locationRequests: any LocationRequests = RecordingLocationRequestHandler(),
    locationEvents: any LocationEvents = RecordingLocationEventSink(),
    mediaRequests: any MediaRequests = RecordingMediaRequestHandler(),
    notificationRequests: any NotificationRequests = RecordingNotificationRequestHandler(),
    notificationEvents: any NotificationEvents = RecordingNotificationEventSink(),
    surfaceKind: RendererSurfaceKind = .metalLayer,
    rendererSurfaceIdentifier: String = "main-surface"
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
            identifier: rendererSurfaceIdentifier
        )
    )
}
