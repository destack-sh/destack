package dev.destack.runtime.android.bridge.text

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.input.text.RuntimeHostTextInputState

/**
 * The low-level text-input ingress ABI for one Android runtime bridge.
 */
public interface TextAbi {
    /**
     * Deliver one host text-input state event into one runtime session.
     */
    public fun notifyTextInputState(
        sessionHandle: HostSessionHandle,
        sessionId: Long,
        state: RuntimeHostTextInputState,
    ): RuntimeAbiStatus
}
