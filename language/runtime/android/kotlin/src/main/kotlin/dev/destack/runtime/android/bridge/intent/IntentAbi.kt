package dev.destack.runtime.android.bridge.intent

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent

/**
 * The low-level intent ingress ABI for one Android runtime bridge.
 */
public interface IntentAbi {
    /**
     * Deliver one intent event into one runtime session.
     */
    public fun notifyIntentEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostIntentEvent,
    ): RuntimeAbiStatus
}
