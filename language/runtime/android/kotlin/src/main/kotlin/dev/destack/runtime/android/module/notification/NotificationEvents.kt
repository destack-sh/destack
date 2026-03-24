package dev.destack.runtime.android.module.notification

/**
 * The notification ingress surface attached to one Android runtime host.
 */
public fun interface NotificationEvents {
    /**
     * Send one normalized notification event into the attached runtime session.
     */
    public fun sendNotificationEvent(
        event: RuntimeHostNotificationEvent,
    )
}
