package dev.destack.runtime.android.bridge.permission

import dev.destack.runtime.android.bridge.RuntimeAbi
import dev.destack.runtime.android.bridge.core.MainThreadBridge
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.permission.PermissionEvents
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

/**
 * One permission bridge lane for one attached Android runtime host.
 */
internal class PermissionBridge(
    private val sessionHandle: HostSessionHandle,
    private val bindings: RuntimeAbi,
    private val mainThreadBridge: MainThreadBridge,
) : PermissionEvents {
    /**
     * Send one permission result into the runtime ingress path.
     */
    override fun sendPermissionEvent(
        event: RuntimeHostPermissionEvent,
    ) {
        val status = bindings.notifyPermissionResult(
            sessionHandle = sessionHandle,
            requestId = event.requestId,
            permission = event.permission,
            isGranted = event.isGranted,
        )

        require(status.code == hostStatusOk) {
            "runtime bridge could not deliver permission event: code ${status.code}, error ${status.errorId}"
        }
    }

    /**
     * Submit one permission request decoded from one runtime callback payload.
     */
    fun submitRequest(
        runtimeHost: RuntimeHost,
        requestId: Long,
        permissions: Array<String>,
    ): Int {
        // the cleaned native host currently supports one permission request at a time
        val permission = permissions.singleOrNull()
            ?: return hostStatusNotSupported

        val request = RuntimeHostPermissionRequest(
            requestId = HostRequestId(rawValue = requestId),
            permission = permission,
        )

        mainThreadBridge.run {
            runtimeHost.permissionRequests.submitPermissionRequest(request)
        }

        return hostStatusOk
    }

    /**
     * Open the native permission settings surface for one runtime callback.
     */
    fun openSettings(
        runtimeHost: RuntimeHost,
    ): Int {
        return mainThreadBridge.run {
            runtimeHost.permissionRequests.openPermissionSettings()
        }
    }
}
