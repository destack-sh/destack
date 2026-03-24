package dev.destack.runtime.android.module.lifecycle

/**
 * The lifecycle ingress surface attached to one Android runtime host.
 */
public fun interface LifecycleEvents {
    /**
     * Send one normalized lifecycle event into the attached runtime session.
     */
    public fun sendLifecycleEvent(
        event: RuntimeHostLifecycleEvent,
    )
}
