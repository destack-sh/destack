import RuntimeHostAppleCore

/// One recording permission request handler for Apple core tests.
@MainActor
final class RecordingPermissionRequestHandler: PermissionRequests {
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

/// One recording permission event sink for Apple core tests.
@MainActor
final class RecordingPermissionEventSink: PermissionEvents {
  var events: [RuntimeHostPermissionEvent] = []

  func notifyPermissionResult(_ event: RuntimeHostPermissionEvent) {
    events.append(event)
  }
}
