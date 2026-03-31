package dev.destack.runtime.android.module.background

/**
 * The Android background scheduler availability state.
 */
public enum class RuntimeHostBackgroundStatus(
    /**
     * The raw bridge value for this enum case.
     */
    val rawValue: Int,
) {
    /**
     * The scheduler is unavailable on this host.
     */
    Unavailable(1),

    /**
     * The scheduler is available but restricted.
     */
    Restricted(2),

    /**
     * The scheduler is available.
     */
    Available(3),
}

/**
 * The Android background trigger class.
 */
public enum class RuntimeHostBackgroundTriggerKind(
    /**
     * The raw bridge value for this enum case.
     */
    val rawValue: Int,
) {
    /**
     * One opportunistic refresh task.
     */
    AppRefresh(1),

    /**
     * One longer processing task.
     */
    Processing(2),
}

/**
 * The Android background completion class.
 */
public enum class RuntimeHostBackgroundTaskResult(
    /**
     * The raw bridge value for this enum case.
     */
    val rawValue: Int,
) {
    /**
     * The task completed successfully.
     */
    Success(1),

    /**
     * The task should be retried.
     */
    Retry(2),

    /**
     * The task failed permanently.
     */
    Failure(3),
}

/**
 * The Android background network requirement.
 */
public enum class RuntimeHostBackgroundNetworkRequirement(
    /**
     * The raw bridge value for this enum case.
     */
    val rawValue: Int,
) {
    /**
     * No network route is required.
     */
    None(1),

    /**
     * One network route is required.
     */
    Connected(2),

    /**
     * One unmetered network route is required.
     */
    Unmetered(3),
}

/**
 * The Android background conflict policy.
 */
public enum class RuntimeHostBackgroundConflictPolicy(
    /**
     * The raw bridge value for this enum case.
     */
    val rawValue: Int,
) {
    /**
     * Replace one existing registration.
     */
    Replace(1),

    /**
     * Keep one existing registration.
     */
    Keep(2),
}

/**
 * The Android background schedule class.
 */
public enum class RuntimeHostBackgroundTaskScheduleKind(
    /**
     * The raw bridge value for this enum case.
     */
    val rawValue: Int,
) {
    /**
     * One single future execution.
     */
    Once(1),

    /**
     * One recurring execution stream.
     */
    Recurring(2),
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
 * One Android background status response.
 */
public data class RuntimeHostBackgroundStatusResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The scheduler status when available.
     */
    val schedulerStatus: RuntimeHostBackgroundStatus? = null,
)

/**
 * One Android background task-list response.
 */
public data class RuntimeHostBackgroundListResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned task descriptors.
     */
    val descriptors: List<RuntimeHostBackgroundTaskDescriptor> = emptyList(),
)

/**
 * One Android background unregister request.
 */
public data class RuntimeHostBackgroundUnregisterRequest(
    /**
     * The stable task identifier.
     */
    val identifier: String,
)

/**
 * One Android background trigger-test request.
 */
public data class RuntimeHostBackgroundTriggerTestRequest(
    /**
     * The stable task identifier.
     */
    val identifier: String,
)

/**
 * One Android background trigger-test response.
 */
public data class RuntimeHostBackgroundTriggerTestResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * Whether the host triggered one task execution.
     */
    val isTriggered: Boolean = false,
)

/**
 * One Android background complete request.
 */
public data class RuntimeHostBackgroundCompleteRequest(
    /**
     * The stable execution identifier.
     */
    val executionId: String,

    /**
     * The completion result.
     */
    val result: RuntimeHostBackgroundTaskResult,
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
public enum class RuntimeHostBackgroundEventKind(
    /**
     * The raw bridge value for this enum case.
     */
    val rawValue: Int,
) {
    /**
     * One task became ready to execute.
     */
    TaskReady(1),

    /**
     * One task execution expired.
     */
    TaskExpired(2),
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
