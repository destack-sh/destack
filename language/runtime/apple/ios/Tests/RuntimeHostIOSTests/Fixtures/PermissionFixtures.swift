import RuntimeHostAppleCore

/// One no-op permission request handler for iOS tests.
@MainActor
final class IOSNoopPermissionRequestHandler: PermissionRequests {
  func request(_ request: RuntimeHostPermissionRequest) -> UInt32 { hostStatusOk }

  func openSettings() -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording permission request handler for iOS tests.
@MainActor
final class IOSRecordingPermissionRequestHandler: PermissionRequests {
  var requests: [RuntimeHostPermissionRequest] = []
  var openSettingsCalls: Int = 0
  var openSettingsStatus: UInt32 = hostStatusOk

  func request(_ request: RuntimeHostPermissionRequest) -> UInt32 {
    requests.append(request)

    return hostStatusOk
  }

  func openSettings() -> UInt32 {
    openSettingsCalls += 1

    return openSettingsStatus
  }
}

/// One no-op permission event sink for iOS tests.
@MainActor
final class IOSNoopPermissionEventSink: PermissionEvents {
  func notifyPermissionResult(_ event: RuntimeHostPermissionEvent) {}
}

/// One recording permission event sink for iOS tests.
@MainActor
final class IOSRecordingPermissionEventSink: PermissionEvents {
  var events: [RuntimeHostPermissionEvent] = []

  func notifyPermissionResult(_ event: RuntimeHostPermissionEvent) {
    events.append(event)
  }
}
