package dev.destack.runtime.android.module.intent

/**
 * The intent ingress surface attached to one Android runtime host.
 */
public fun interface IntentEvents {
    /**
     * Send one normalized intent event into the attached runtime session.
     */
    public fun sendIntentEvent(
        event: RuntimeHostIntentEvent,
    )
}
