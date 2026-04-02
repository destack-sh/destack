package dev.destack.runtime.android.module.notification

/**
 * The Android notification priority payload.
 */
public enum class RuntimeHostNotificationPriority(
    /**
     * The stable ABI discriminant.
     */
    val rawValue: Int,
) {
    /**
     * One low-priority notification.
     */
    Low(1),

    /**
     * One normal-priority notification.
     */
    Normal(2),

    /**
     * One high-priority notification.
     */
    High(3),
}

/**
 * One Android notification calendar trigger payload.
 */
public data class RuntimeHostNotificationCalendarTrigger(
    /**
     * The trigger year.
     */
    val year: Int = 0,

    /**
     * The trigger month, 1 to 12.
     */
    val month: Int = 0,

    /**
     * The trigger day of month, 1 to 31.
     */
    val day: Int = 0,

    /**
     * The trigger hour, 0 to 23.
     */
    val hour: Int = 0,

    /**
     * The trigger minute, 0 to 59.
     */
    val minute: Int = 0,

    /**
     * The trigger second, 0 to 59.
     */
    val second: Int = 0,

    /**
     * The trigger timezone identifier.
     */
    val timeZone: String = "",

    /**
     * Whether this trigger repeats.
     */
    val repeats: Boolean = false,
)

/**
 * The Android notification trigger-kind payload.
 */
public enum class RuntimeHostNotificationTriggerKind(
    /**
     * The stable ABI discriminant.
     */
    val rawValue: Int,
) {
    /**
     * One calendar-date trigger.
     */
    CalendarDate(1),

    /**
     * One immediate trigger.
     */
    Immediate(2),

    /**
     * One time-interval trigger.
     */
    TimeInterval(3),
}

/**
 * One Android notification trigger payload.
 */
public data class RuntimeHostNotificationTrigger(
    /**
     * The trigger variant kind.
     */
    val kind: RuntimeHostNotificationTriggerKind = RuntimeHostNotificationTriggerKind.Immediate,

    /**
     * The calendar trigger payload.
     */
    val calendar: RuntimeHostNotificationCalendarTrigger =
        RuntimeHostNotificationCalendarTrigger(),

    /**
     * The time-interval trigger delay in nanoseconds.
     */
    val intervalNs: Long = 0L,
)

/**
 * One Android notification request submitted by one runtime session.
 */
public data class RuntimeHostNotificationRequest(
    /**
     * The primary notification title.
     */
    val title: String,

    /**
     * The subtitle when present.
     */
    val subtitle: String? = null,

    /**
     * The primary notification body text.
     */
    val body: String,

    /**
     * The host notification tag.
     */
    val tag: String,

    /**
     * The channel identifier when present.
     */
    val channelId: String? = null,

    /**
     * The priority class.
     */
    val priority: RuntimeHostNotificationPriority = RuntimeHostNotificationPriority.Normal,

    /**
     * The badge count when present.
     */
    val badgeCount: Int? = null,

    /**
     * The sound identifier when present.
     */
    val sound: String? = null,

    /**
     * The category identifier when present.
     */
    val categoryId: String? = null,

    /**
     * The thread identifier when present.
     */
    val threadId: String? = null,

    /**
     * The delivery trigger selector.
     */
    val trigger: RuntimeHostNotificationTrigger = RuntimeHostNotificationTrigger(),

    /**
     * The action identifier when present.
     */
    val actionId: String? = null,
) {
    /**
     * The stable runtime notification identifier.
     */
    public val identifier: String
        get() = tag
}

/**
 * One Android notification event metadata payload.
 */
public data class RuntimeHostNotificationEventMetadata(
    /**
     * The monotonic event timestamp in nanoseconds.
     */
    val timestampNs: Long = 0L,

    /**
     * The monotonic sequence number for this event stream.
     */
    val sequence: Long = 0L,

    /**
     * The host notification identifier.
     */
    val id: String,

    /**
     * The notification request payload.
     */
    val request: RuntimeHostNotificationRequest,
)

/**
 * One Android notification interacted payload.
 */
public data class RuntimeHostNotificationInteractedPayload(
    /**
     * The action identifier when present.
     */
    val actionId: String? = null,

    /**
     * The text-input response when present.
     */
    val actionResponseText: String? = null,
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
     * The notification was dismissed by the user or host.
     */
    Dismissed(2),

    /**
     * The notification was interacted with by the user.
     */
    Interacted(3),
}

/**
 * One Android notification ingress event delivered into one runtime session.
 */
public data class RuntimeHostNotificationEvent(
    /**
     * The notification interaction kind.
     */
    val kind: RuntimeHostNotificationEventKind,

    /**
     * The shared event metadata.
     */
    val metadata: RuntimeHostNotificationEventMetadata,

    /**
     * The interaction payload.
     */
    val payload: RuntimeHostNotificationInteractedPayload =
        RuntimeHostNotificationInteractedPayload(),
) {
    /**
     * The stable runtime notification identifier.
     */
    public val identifier: String
        get() = metadata.id

    /**
     * The request associated with this notification event.
     */
    public val request: RuntimeHostNotificationRequest
        get() = metadata.request

    /**
     * The host event sequence for this notification stream.
     */
    public val sequence: Long
        get() = metadata.sequence

    /**
     * The host event timestamp in nanoseconds.
     */
    public val timestampNs: Long
        get() = metadata.timestampNs

    /**
     * The optional action identifier for interactive notifications.
     */
    public val actionIdentifier: String?
        get() = payload.actionId
}
