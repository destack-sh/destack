package dev.destack.runtime.android.module.notification

/**
 * One Android notification request submitted by one runtime session.
 */
public data class RuntimeHostNotificationRequest(
    /**
     * The stable runtime notification identifier.
     */
    val identifier: String,

    /**
     * The primary notification title.
     */
    val title: String,

    /**
     * The primary notification body text.
     */
    val body: String,
)

/**
 * The Android notification interaction delivered into one runtime session.
 */
public enum class RuntimeHostNotificationEventKind(
    /**
     * The stable ABI discriminant.
     */
    val rawValue: Int,
) {
    /**
     * The notification was delivered to the Android host surface.
     */
    Delivered(1),

    /**
     * The notification was activated by the user.
     */
    Activated(2),

    /**
     * The notification was dismissed by the user or host.
     */
    Dismissed(3),
}

/**
 * One Android notification ingress event delivered into one runtime session.
 */
public data class RuntimeHostNotificationEvent(
    /**
     * The stable runtime notification identifier.
     */
    val identifier: String,

    /**
     * The request associated with this notification event.
     */
    val request: RuntimeHostNotificationRequest,

    /**
     * The notification interaction kind.
     */
    val kind: RuntimeHostNotificationEventKind,

    /**
     * The optional action identifier for interactive notifications.
     */
    val actionIdentifier: String? = null,

    /**
     * The host event sequence for this notification stream.
     */
    val sequence: Long = 0,

    /**
     * The host event timestamp in nanoseconds.
     */
    val timestampNs: Long = 0,
)
