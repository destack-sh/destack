package dev.destack.runtime.android.module.location

/**
 * The location ingress surface attached to one Android runtime host.
 */
public fun interface LocationEvents {
    /**
     * Send one normalized location sample into the attached runtime session.
     */
    public fun sendLocationSample(
        watchId: String,
        sample: RuntimeHostLocationSample,
    )
}
