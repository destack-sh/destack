import RuntimeHostAppleCore

/// One no-op permission request handler for macOS tests.
@MainActor
final class MacOSNoopPermissionRequestHandler: PermissionRequests {
  func request(_ request: RuntimeHostPermissionRequest) -> UInt32 { hostStatusOk }

  func openSettings() -> UInt32 {
    hostStatusNotSupported
  }
}

/// One recording permission request handler for macOS tests.
@MainActor
final class MacOSRecordingPermissionRequestHandler: PermissionRequests {
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

/// One no-op permission event sink for macOS tests.
@MainActor
final class MacOSNoopPermissionEventSink: PermissionEvents {
  func notifyPermissionResult(_ event: RuntimeHostPermissionEvent) {}
}

/// One recording permission event sink for macOS tests.
@MainActor
final class MacOSRecordingPermissionEventSink: PermissionEvents {
  var events: [RuntimeHostPermissionEvent] = []

  func notifyPermissionResult(_ event: RuntimeHostPermissionEvent) {
    events.append(event)
  }
}
