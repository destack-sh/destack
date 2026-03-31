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
    var requestStatus: Int = 0

    override fun request(
        request: RuntimeHostPermissionRequest,
    ): Int {
        requests += request

        return requestStatus
    }

    override fun openSettings(): Int {
        openSettingsCalls += 1

        return openSettingsStatus
    }
}

/**
 * One recording permission event sink for Android host tests.
 */
internal class RecordingPermissionEventSink : PermissionEvents {
    val events: MutableList<RuntimeHostPermissionEvent> = mutableListOf()
    val callback: PermissionEvents = this

    override fun notifyPermissionResult(
        event: RuntimeHostPermissionEvent,
    ) {
        events += event
    }
}

/**
 * One no-op permission request handler for Android host tests.
 */
internal class NoopPermissionRequestHandler : PermissionRequests {
    override fun request(
        request: RuntimeHostPermissionRequest,
    ): Int = 0

    override fun openSettings(): Int {
        return 1
    }
}
