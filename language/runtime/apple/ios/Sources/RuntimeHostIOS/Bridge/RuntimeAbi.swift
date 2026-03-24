import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One runtime status returned by one runtime ABI ingress call.
struct RuntimeAbiStatus {
  /// The status code, where zero means success.
  let code: UInt32
  /// The recorded runtime error identifier when one failure occurred.
  let errorID: UInt64
}

/// One low-level runtime ABI surface for one iOS host bridge.
protocol RuntimeAbi: CalendarAbi, ContactAbi, DocumentAbi, IntentAbi, LocationAbi, MediaAbi, NotificationAbi, PermissionAbi {
  /// Attach one bridge instance to one runtime session.
  func attachBridge(
    sessionHandle: HostSessionHandle,
    bridge: RuntimeBridge
  ) -> HostAbiStatus

  /// Detach one bridge instance from one runtime session.
  func detachBridge(
    sessionHandle: HostSessionHandle
  )
}

/// One default runtime ABI surface loaded through the iOS bridge C shim.
final class ProcessRuntimeAbi: RuntimeAbi, @unchecked Sendable {
  /// Attach one bridge instance to one runtime session.
  func attachBridge(
    sessionHandle: HostSessionHandle,
    bridge: RuntimeBridge
  ) -> HostAbiStatus {
    destack_runtime_host_ios_register_runtime_bridge_bindings(
      sessionHandle.rawValue,
      IosRuntimeBridgeBindings(
        document: IosHostDocumentCallbacks(
          pick: documentCallback
        ),
        permission: IosHostPermissionCallbacks(
          open_settings: permissionOpenSettingsCallback,
          request: permissionRequestCallback
        ),
        contact: IosHostContactCallbacks(
          list: contactListCallback,
          search: contactSearchCallback,
          read: contactReadCallback,
          create: contactCreateCallback,
          update: contactUpdateCallback,
          delete_contact: contactDeleteCallback
        ),
        calendar: IosHostCalendarCallbacks(
          list: calendarListCallback,
          event_list: calendarEventListCallback,
          event_read: calendarEventReadCallback,
          event_create: calendarEventCreateCallback,
          event_update: calendarEventUpdateCallback,
          event_delete: calendarEventDeleteCallback
        ),
        intent: IosHostIntentCallbacks(
          can_open_url: intentCanOpenURLCallback,
          open_url: intentOpenURLCallback,
          open_path: intentOpenPathCallback,
          share_text: intentShareTextCallback,
          share_paths: intentSharePathsCallback
        ),
        location: IosHostLocationCallbacks(
          services_enabled: locationServicesEnabledCallback,
          last_known: locationLastKnownCallback,
          watch_open: locationWatchOpenCallback,
          watch_close: locationWatchCloseCallback
        ),
        media: IosHostMediaCallbacks(
          list: mediaListCallback,
          read: mediaReadCallback,
          import_path: mediaImportPathCallback,
          delete_media: mediaDeleteCallback
        ),
        notification: IosHostNotificationCallbacks(
          cancel: notificationCancelCallback,
          cancel_all: notificationCancelAllCallback,
          post: notificationPostCallback
        )
      )
    )
  }

  /// Detach one bridge instance from one runtime session.
  func detachBridge(
    sessionHandle: HostSessionHandle
  ) {
    destack_runtime_host_ios_unregister_runtime_bridge_bindings(sessionHandle.rawValue)
  }

  /// Deliver one location sample into one runtime session.
  func notifyLocationSample(
    sessionHandle: HostSessionHandle,
    watchID: String,
    sample: RuntimeHostLocationSample
  ) -> RuntimeAbiStatus {
    withNativeStringRef(watchID) { watchIDRef in
      let status = destack_runtime_host_ios_notify_location_sample(
        sessionHandle.rawValue,
        watchIDRef,
        DestackRustLocationSample(
          latitude_degrees: sample.latitudeDegrees,
          longitude_degrees: sample.longitudeDegrees,
          altitude_meters: sample.altitudeMeters,
          horizontal_accuracy_meters: sample.horizontalAccuracyMeters,
          vertical_accuracy_meters: sample.verticalAccuracyMeters,
          speed_meters_per_second: sample.speedMetersPerSecond,
          heading_degrees: sample.headingDegrees,
          timestamp_unix_ns: sample.timestampUnixNs
        )
      )

      return RuntimeAbiStatus(
        code: status.code,
        errorID: status.error_id
      )
    }
  }
}
