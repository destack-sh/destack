package dev.destack.runtime.android.module.permission

import dev.destack.runtime.android.core.HostRequestId

/**
 * One Android permission request submitted by one runtime session.
 */
public data class RuntimeHostPermissionRequest(
    /**
     * The stable request identifier for this interactive host flow.
     */
    val requestId: HostRequestId,

    /**
     * The normalized permission name requested by the runtime.
     */
    val permission: String,
)

/**
 * One Android permission result event delivered into one runtime session.
 */
public data class RuntimeHostPermissionEvent(
    /**
     * The stable request identifier for this interactive host flow.
     */
    val requestId: HostRequestId,

    /**
     * The normalized permission name associated with this result.
     */
    val permission: String,

    /**
     * Whether the Android host granted the permission.
     */
    val isGranted: Boolean,
)
