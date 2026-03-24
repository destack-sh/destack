package dev.destack.runtime.android.module.permission

/**
 * The permission ingress surface attached to one Android runtime host.
 */
public fun interface PermissionEvents {
    /**
     * Send one normalized permission event into the attached runtime session.
     */
    public fun sendPermissionEvent(
        event: RuntimeHostPermissionEvent,
    )
}
