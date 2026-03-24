package dev.destack.runtime.android.bridge.permission

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.bridge.RuntimeHostLibraryLoader
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle

/**
 * The default permission lane ABI resolved through the Android JNI bridge.
 */
internal object ProcessPermissionAbi : PermissionAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun notifyPermissionResult(
        sessionHandle: HostSessionHandle,
        requestId: HostRequestId?,
        permission: String,
        isGranted: Boolean,
    ): RuntimeAbiStatus {
        val values = nativeNotifyPermissionResult(
            sessionHandle.rawValue,
            requestId != null,
            requestId?.rawValue ?: 0,
            permission,
            isGranted,
        )

        return RuntimeAbiStatus(
            code = values[0].toInt(),
            errorId = values[1],
        )
    }

    private external fun nativeNotifyPermissionResult(
        sessionHandle: Long,
        hasRequestId: Boolean,
        requestId: Long,
        permission: String,
        isGranted: Boolean,
    ): LongArray
}
