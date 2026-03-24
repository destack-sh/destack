package dev.destack.runtime.android.module.permission

/**
 * The permission request surface attached to one Android runtime host.
 */
public interface PermissionRequests {
    /**
     * Submit one permission request to the Android host.
     */
    public fun submitPermissionRequest(
        request: RuntimeHostPermissionRequest,
    )

    /**
     * Open the native permission settings surface through the Android host.
     */
    public fun openPermissionSettings(): Int
}
