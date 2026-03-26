package dev.destack.runtime.android.module.background

/**
 * The Android background scheduler availability state.
 */
public enum class RuntimeHostBackgroundStatus {
    /**
     * The scheduler is unavailable on this host.
     */
    Unavailable,

    /**
     * The scheduler is available but restricted.
     */
    Restricted,

    /**
     * The scheduler is available.
     */
    Available,
}

/**
 * The Android background trigger class.
 */
public enum class RuntimeHostBackgroundTriggerKind {
    /**
     * One opportunistic refresh task.
     */
    AppRefresh,

    /**
     * One longer processing task.
     */
    Processing,
}

/**
 * The Android background completion class.
 */
public enum class RuntimeHostBackgroundTaskResult {
    /**
     * The task completed successfully.
     */
    Success,

    /**
     * The task should be retried.
     */
    Retry,

    /**
     * The task failed permanently.
     */
    Failure,
}

/**
 * The Android background network requirement.
 */
public enum class RuntimeHostBackgroundNetworkRequirement {
    /**
     * No network route is required.
     */
    None,

    /**
     * One network route is required.
     */
    Connected,

    /**
     * One unmetered network route is required.
     */
    Unmetered,
}

/**
 * The Android background conflict policy.
 */
public enum class RuntimeHostBackgroundConflictPolicy {
    /**
     * Replace one existing registration.
     */
    Replace,

    /**
     * Keep one existing registration.
     */
    Keep,
}

/**
 * The Android background schedule class.
 */
public enum class RuntimeHostBackgroundTaskScheduleKind {
    /**
     * One single future execution.
     */
    Once,

    /**
     * One recurring execution stream.
     */
    Recurring,
}

/**
 * One Android background task schedule payload.
 */
public data class RuntimeHostBackgroundTaskSchedule(
    /**
     * The schedule class.
     */
    val kind: RuntimeHostBackgroundTaskScheduleKind,

    /**
     * The earliest execution target in UTC nanoseconds when provided.
     */
    val earliestBeginUnixNs: Long?,

    /**
     * The repeat interval in nanoseconds for recurring schedules.
     */
    val repeatIntervalNs: Long? = null,
)

/**
 * One Android background task registration payload.
 */
public data class RuntimeHostBackgroundTaskOptions(
    /**
     * The stable task identifier.
     */
    val identifier: String,

    /**
     * The trigger class.
     */
    val trigger: RuntimeHostBackgroundTriggerKind,

    /**
     * The requested schedule payload.
     */
    val schedule: RuntimeHostBackgroundTaskSchedule,

    /**
     * The requested network requirement.
     */
    val network: RuntimeHostBackgroundNetworkRequirement,

    /**
     * Whether charging power is required.
     */
    val requiresCharging: Boolean,

    /**
     * Whether device idle mode is required.
     */
    val requiresIdle: Boolean,

    /**
     * The conflict policy for one existing registration.
     */
    val conflictPolicy: RuntimeHostBackgroundConflictPolicy,
)

/**
 * One Android background task descriptor payload.
 */
public data class RuntimeHostBackgroundTaskDescriptor(
    /**
     * The stable task identifier.
     */
    val identifier: String,

    /**
     * The trigger class.
     */
    val trigger: RuntimeHostBackgroundTriggerKind,

    /**
     * The effective schedule payload.
     */
    val schedule: RuntimeHostBackgroundTaskSchedule,

    /**
     * The effective network requirement.
     */
    val network: RuntimeHostBackgroundNetworkRequirement,

    /**
     * Whether charging power is required.
     */
    val requiresCharging: Boolean,

    /**
     * Whether device idle mode is required.
     */
    val requiresIdle: Boolean,

    /**
     * The conflict policy for one existing registration.
     */
    val conflictPolicy: RuntimeHostBackgroundConflictPolicy,
) {
    /**
     * Return the flattened schedule kind for the Android bridge.
     */
    val scheduleKind: RuntimeHostBackgroundTaskScheduleKind
        get() = schedule.kind

    /**
     * Return whether this descriptor uses one recurring schedule.
     */
    val isRecurring: Boolean
        get() = schedule.kind == RuntimeHostBackgroundTaskScheduleKind.Recurring

    /**
     * Return whether one earliest execution target is present for the Android bridge.
     */
    val hasEarliestBeginUnixNs: Boolean
        get() = schedule.earliestBeginUnixNs != null

    /**
     * Return the flattened earliest execution target for the Android bridge.
     */
    val earliestBeginUnixNs: Long
        get() = schedule.earliestBeginUnixNs ?: 0L

    /**
     * Return whether one repeat interval is present for the Android bridge.
     */
    val hasRepeatIntervalNs: Boolean
        get() = schedule.repeatIntervalNs != null

    /**
     * Return the flattened repeat interval for the Android bridge.
     */
    val repeatIntervalNs: Long
        get() = schedule.repeatIntervalNs ?: 0L
}

/**
 * One Android background event metadata payload.
 */
public data class RuntimeHostBackgroundEventMetadata(
    /**
     * The monotonic event timestamp in nanoseconds.
     */
    val timestampNs: Long,

    /**
     * The monotonic event sequence number.
     */
    val sequence: Long,

    /**
     * The stable task identifier.
     */
    val identifier: String,

    /**
     * The stable execution identifier.
     */
    val executionId: String,

    /**
     * The execution deadline in UTC nanoseconds.
     */
    val deadlineUnixNs: Long,
)

/**
 * The Android background event kind.
 */
public enum class RuntimeHostBackgroundEventKind {
    /**
     * One task became ready to execute.
     */
    TaskReady,

    /**
     * One task execution expired.
     */
    TaskExpired,
}

/**
 * One Android background ingress event delivered into one runtime session.
 */
public data class RuntimeHostBackgroundEvent(
    /**
     * The event kind.
     */
    val kind: RuntimeHostBackgroundEventKind,

    /**
     * The shared event metadata.
     */
    val metadata: RuntimeHostBackgroundEventMetadata,
)
