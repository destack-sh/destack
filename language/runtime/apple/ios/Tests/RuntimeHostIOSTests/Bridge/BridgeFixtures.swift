import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

@testable import RuntimeHostIOS

/// One runtime ingress spy for iOS bridge tests.
final class RuntimeIngressSpy: RuntimeIngress {
  var attachStatus: UInt32 = hostStatusOk
  var attachedSessionHandles: [HostSessionHandle] = []
  var detachedSessionHandles: [HostSessionHandle] = []
  var backgroundEvents: [(HostSessionHandle, RuntimeHostBackgroundEvent)] = []
  var documentResults: [(HostSessionHandle, HostRequestID, [RuntimeHostDocumentDescriptor])] = []
  var intentEvents: [(HostSessionHandle, RuntimeHostIntentEvent)] = []
  var locationSamples: [(HostSessionHandle, String, RuntimeHostLocationSample)] = []
  var notificationEvents: [(HostSessionHandle, RuntimeHostNotificationEvent)] = []
  var permissionEvents: [(HostSessionHandle, HostRequestID, String, Bool)] = []
  var textInputEvents: [(HostSessionHandle, RuntimeHostTextInputEvent)] = []

  var documentCallback: DocumentPickCallback?
  var permissionRequestCallback: PermissionRequestCallback?
  var permissionOpenSettingsCallback: PermissionOpenSettingsCallback?
  var backgroundStatusCallback: BackgroundStatusCallback?
  var backgroundListCallback: BackgroundListCallback?
  var backgroundRegisterCallback: BackgroundRegisterTaskCallback?
  var backgroundUnregisterCallback: BackgroundUnregisterCallback?
  var backgroundTriggerTestCallback: BackgroundTriggerTestCallback?
  var backgroundCompleteCallback: BackgroundCompleteCallback?
  var calendarListCallback: CalendarListCallback?
  var calendarEventListCallback: CalendarEventListCallback?
  var calendarEventReadCallback: CalendarEventReadCallback?
  var calendarEventCreateCallback: CalendarEventCreateCallback?
  var calendarEventUpdateCallback: CalendarEventUpdateCallback?
  var calendarEventDeleteCallback: CalendarEventDeleteCallback?
  var contactListCallback: ContactListCallback?
  var contactSearchCallback: ContactSearchCallback?
  var contactReadCallback: ContactReadCallback?
  var contactCreateCallback: ContactCreateCallback?
  var contactUpdateCallback: ContactUpdateCallback?
  var contactDeleteCallback: ContactDeleteContactCallback?
  var intentCanOpenURLCallback: IntentCanOpenUrlCallback?
  var intentOpenURLCallback: IntentOpenUrlCallback?
  var intentOpenPathCallback: IntentOpenPathCallback?
  var intentShareTextCallback: IntentShareTextCallback?
  var intentSharePathsCallback: IntentSharePathsCallback?
  var locationServicesEnabledCallback: LocationServicesEnabledCallback?
  var locationLastKnownCallback: LocationLastKnownCallback?
  var locationWatchOpenCallback: LocationWatchOpenCallback?
  var locationWatchCloseCallback: LocationWatchCloseCallback?
  var mediaListCallback: MediaListCallback?
  var mediaReadCallback: MediaReadCallback?
  var mediaImportPathCallback: MediaImportPathCallback?
  var mediaDeleteCallback: MediaDeleteCallback?
  var notificationPostCallback: NotificationPostCallback?
  var notificationCancelCallback: NotificationCancelCallback?
  var notificationCancelAllCallback: NotificationCancelAllCallback?

  /// Deliver one background event into one runtime session.
  func notifyBackgroundEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostBackgroundEvent
  ) -> RuntimeIngressStatus {
    backgroundEvents.append((sessionHandle, event))

    return RuntimeIngressStatus(code: hostStatusOk, errorID: 0)
  }

  /// Deliver one document result into one runtime session.
  func notifyDocumentResult(
    sessionHandle: HostSessionHandle,
    requestID: HostRequestID,
    documents: [RuntimeHostDocumentDescriptor]
  ) -> RuntimeIngressStatus {
    documentResults.append((sessionHandle, requestID, documents))

    return RuntimeIngressStatus(code: hostStatusOk, errorID: 0)
  }

  /// Deliver one intent event into one runtime session.
  func notifyIntentEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostIntentEvent
  ) -> RuntimeIngressStatus {
    intentEvents.append((sessionHandle, event))

    return RuntimeIngressStatus(code: hostStatusOk, errorID: 0)
  }

  /// Deliver one location sample into one runtime session.
  func notifyLocationSample(
    sessionHandle: HostSessionHandle,
    watchID: String,
    sample: RuntimeHostLocationSample
  ) -> RuntimeIngressStatus {
    locationSamples.append((sessionHandle, watchID, sample))

    return RuntimeIngressStatus(code: hostStatusOk, errorID: 0)
  }

  /// Deliver one notification event into one runtime session.
  func notifyNotificationEvent(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostNotificationEvent
  ) -> RuntimeIngressStatus {
    notificationEvents.append((sessionHandle, event))

    return RuntimeIngressStatus(code: hostStatusOk, errorID: 0)
  }

  /// Deliver one permission result into one runtime session.
  func notifyPermissionResult(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostPermissionEvent
  ) -> RuntimeIngressStatus {
    permissionEvents.append((sessionHandle, event.requestID, event.permission, event.isGranted))

    return RuntimeIngressStatus(code: hostStatusOk, errorID: 0)
  }

  /// Deliver one text-session state event into one runtime session.
  func notifyTextInputState(
    sessionHandle: HostSessionHandle,
    event: RuntimeHostTextInputEvent
  ) -> RuntimeIngressStatus {
    textInputEvents.append((sessionHandle, event))

    return RuntimeIngressStatus(code: hostStatusOk, errorID: 0)
  }

  /// Attach one bridge instance to one runtime session.
  func attachBridge(
    sessionHandle: HostSessionHandle,
    bridge: RuntimeBridge
  ) -> HostAbiStatus {
    _ = bridge

    attachedSessionHandles.append(sessionHandle)

    documentCallback = RuntimeHostIOS.documentPickCallback
    permissionRequestCallback = RuntimeHostIOS.permissionRequestCallback
    permissionOpenSettingsCallback = RuntimeHostIOS.permissionOpenSettingsCallback
    backgroundStatusCallback = RuntimeHostIOS.backgroundStatusCallback
    backgroundListCallback = RuntimeHostIOS.backgroundListCallback
    backgroundRegisterCallback = RuntimeHostIOS.backgroundRegisterTaskCallback
    backgroundUnregisterCallback = RuntimeHostIOS.backgroundUnregisterCallback
    backgroundTriggerTestCallback = RuntimeHostIOS.backgroundTriggerTestCallback
    backgroundCompleteCallback = RuntimeHostIOS.backgroundCompleteCallback
    calendarListCallback = RuntimeHostIOS.calendarListCallback
    calendarEventListCallback = RuntimeHostIOS.calendarEventListCallback
    calendarEventReadCallback = RuntimeHostIOS.calendarEventReadCallback
    calendarEventCreateCallback = RuntimeHostIOS.calendarEventCreateCallback
    calendarEventUpdateCallback = RuntimeHostIOS.calendarEventUpdateCallback
    calendarEventDeleteCallback = RuntimeHostIOS.calendarEventDeleteCallback
    contactListCallback = RuntimeHostIOS.contactListCallback
    contactSearchCallback = RuntimeHostIOS.contactSearchCallback
    contactReadCallback = RuntimeHostIOS.contactReadCallback
    contactCreateCallback = RuntimeHostIOS.contactCreateCallback
    contactUpdateCallback = RuntimeHostIOS.contactUpdateCallback
    contactDeleteCallback = RuntimeHostIOS.contactDeleteContactCallback
    intentCanOpenURLCallback = RuntimeHostIOS.intentCanOpenUrlCallback
    intentOpenURLCallback = RuntimeHostIOS.intentOpenUrlCallback
    intentOpenPathCallback = RuntimeHostIOS.intentOpenPathCallback
    intentShareTextCallback = RuntimeHostIOS.intentShareTextCallback
    intentSharePathsCallback = RuntimeHostIOS.intentSharePathsCallback
    locationServicesEnabledCallback = RuntimeHostIOS.locationServicesEnabledCallback
    locationLastKnownCallback = RuntimeHostIOS.locationLastKnownCallback
    locationWatchOpenCallback = RuntimeHostIOS.locationWatchOpenCallback
    locationWatchCloseCallback = RuntimeHostIOS.locationWatchCloseCallback
    mediaListCallback = RuntimeHostIOS.mediaListCallback
    mediaReadCallback = RuntimeHostIOS.mediaReadCallback
    mediaImportPathCallback = RuntimeHostIOS.mediaImportPathCallback
    mediaDeleteCallback = RuntimeHostIOS.mediaDeleteCallback
    notificationPostCallback = RuntimeHostIOS.notificationPostCallback
    notificationCancelCallback = RuntimeHostIOS.notificationCancelCallback
    notificationCancelAllCallback = RuntimeHostIOS.notificationCancelAllCallback

    return attachStatus
  }

  /// Detach one bridge instance from one runtime session.
  func detachBridge(
    sessionHandle: HostSessionHandle
  ) {
    detachedSessionHandles.append(sessionHandle)
  }
}

func withBridgeDocumentRequest<T>(
  _ request: RuntimeHostDocumentRequest,
  body: (DestackRustDocumentRequest) -> T
) -> T {
  withNativeStringSlice(request.contentTypes) { mimeTypes in
    withNativeStringSlice([]) { extensions in
      body(
        DestackRustDocumentRequest(
          request_id: request.requestID.rawValue,
          mime_types: mimeTypes,
          extensions: extensions,
          allows_multiple_selection: request.allowsMultipleSelection,
          allows_directory_selection: false,
          copies_to_sandbox: false
        )
      )
    }
  }
}

func withBridgePermissionRequest<T>(
  _ request: RuntimeHostPermissionRequest,
  body: (DestackRustPermissionRequest) -> T
) -> T {
  withNativeStringRef(request.permission) { permission in
    body(
      DestackRustPermissionRequest(
        request_id: request.requestID.rawValue,
        permission: permission
      )
    )
  }
}
