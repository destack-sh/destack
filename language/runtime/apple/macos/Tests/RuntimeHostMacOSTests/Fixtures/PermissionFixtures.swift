import RuntimeHostAppleCore

/// One no-op permission request handler for macOS tests.
@MainActor
final class MacOSNoopPermissionRequestHandler: PermissionRequests {
    func submitPermissionRequest(_ request: RuntimeHostPermissionRequest) {}

    func openPermissionSettings() -> UInt32 {
        hostStatusNotSupported
    }
}

/// One recording permission request handler for macOS tests.
@MainActor
final class MacOSRecordingPermissionRequestHandler: PermissionRequests {
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

/// One no-op permission event sink for macOS tests.
@MainActor
final class MacOSNoopPermissionEventSink: PermissionEvents {
    func sendPermissionEvent(_ event: RuntimeHostPermissionEvent) {}
}

/// One recording permission event sink for macOS tests.
@MainActor
final class MacOSRecordingPermissionEventSink: PermissionEvents {
    var events: [RuntimeHostPermissionEvent] = []

    func sendPermissionEvent(_ event: RuntimeHostPermissionEvent) {
        events.append(event)
    }
}
