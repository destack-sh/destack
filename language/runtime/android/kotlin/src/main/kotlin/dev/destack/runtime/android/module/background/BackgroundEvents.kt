package dev.destack.runtime.android.module.background

/**
 * The background ingress surface attached to one Android runtime host.
 */
public fun interface BackgroundEvents {
    /**
     * Send one host background event into the attached runtime session.
     */
    public fun sendBackgroundEvent(
        event: RuntimeHostBackgroundEvent,
    )
}
