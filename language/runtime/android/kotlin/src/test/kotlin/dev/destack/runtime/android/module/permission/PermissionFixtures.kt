package dev.destack.runtime.android

import dev.destack.runtime.android.module.permission.PermissionEvents
import dev.destack.runtime.android.module.permission.PermissionRequests
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

/**
 * One recording permission request handler for Android host tests.
 */
internal class RecordingPermissionRequestHandler : PermissionRequests {
    val requests: MutableList<RuntimeHostPermissionRequest> = mutableListOf()
    var openSettingsCalls: Int = 0
    var openSettingsStatus: Int = 0

    override fun submitPermissionRequest(
        request: RuntimeHostPermissionRequest,
    ) {
        requests += request
    }

    override fun openPermissionSettings(): Int {
        openSettingsCalls += 1

        return openSettingsStatus
    }
}

/**
 * One recording permission event sink for Android host tests.
 */
internal class RecordingPermissionEventSink : PermissionEvents {
    val events: MutableList<RuntimeHostPermissionEvent> = mutableListOf()

    override fun sendPermissionEvent(
        event: RuntimeHostPermissionEvent,
    ) {
        events += event
    }
}

/**
 * One no-op permission request handler for Android host tests.
 */
internal class NoopPermissionRequestHandler : PermissionRequests {
    override fun submitPermissionRequest(
        request: RuntimeHostPermissionRequest,
    ) {}

    override fun openPermissionSettings(): Int {
        return 1
    }
}
