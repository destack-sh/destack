package dev.destack.runtime.android.bridge.permission

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle

/**
 * The low-level permission lane ABI for one Android runtime bridge.
 */
public interface PermissionAbi {
    /**
     * Deliver one permission result into one runtime session.
     */
    public fun notifyPermissionResult(
        sessionHandle: HostSessionHandle,
        requestId: HostRequestId?,
        permission: String,
        isGranted: Boolean,
    ): RuntimeAbiStatus
}
