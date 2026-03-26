package dev.destack.runtime.android.bridge.background

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEvent

/**
 * The low-level background ingress ABI for one Android runtime bridge.
 */
public interface BackgroundAbi {
    /**
     * Deliver one host background event into one runtime session.
     */
    public fun notifyBackgroundEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostBackgroundEvent,
    ): RuntimeAbiStatus
}
