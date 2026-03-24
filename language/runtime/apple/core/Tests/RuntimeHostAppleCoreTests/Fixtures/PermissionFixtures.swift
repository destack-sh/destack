import RuntimeHostAppleCore

/// One recording permission request handler for Apple core tests.
@MainActor
final class RecordingPermissionRequestHandler: PermissionRequests {
    var requests: [RuntimeHostPermissionRequest] = []
    var openSettingsCalls: Int = 0
    var openSettingsStatus: UInt32 = hostStatusOk

    func submitPermissionRequest(_ request: RuntimeHostPermissionRequest) {
        requests.append(request)
    }

    func openPermissionSettings() -> UInt32 {
        openSettingsCalls += 1

        return openSettingsStatus
    }
}

/// One recording permission event sink for Apple core tests.
@MainActor
final class RecordingPermissionEventSink: PermissionEvents {
    var events: [RuntimeHostPermissionEvent] = []

    func sendPermissionEvent(_ event: RuntimeHostPermissionEvent) {
        events.append(event)
    }
}
