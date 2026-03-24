package dev.destack.runtime.android.bridge.notification

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent

/**
 * The low-level notification ABI for one Android runtime bridge.
 */
public interface NotificationAbi {
    /**
     * Deliver one notification event into one runtime session.
     */
    public fun notifyNotificationEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostNotificationEvent,
        sequence: Long,
        timestampNs: Long,
    ): RuntimeAbiStatus
}
