package dev.destack.runtime.android.bridge.location

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample

/**
 * The low-level location ingress ABI for one Android runtime bridge.
 */
public interface LocationAbi {
    /**
     * Deliver one location sample into one runtime session.
     */
    public fun notifyLocationSample(
        sessionHandle: HostSessionHandle,
        watchId: String,
        sample: RuntimeHostLocationSample,
    ): RuntimeAbiStatus
}
