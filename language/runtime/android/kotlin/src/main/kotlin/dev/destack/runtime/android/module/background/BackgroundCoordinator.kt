package dev.destack.runtime.android.module.background

import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk

private const val backgroundExecutionDeadlineNs: Long = 30_000_000_000L
private const val backgroundRetryDelayNs: Long = 10_000_000_000L

/**
 * One live Android background execution.
 */
internal data class BackgroundExecution(
    /**
     * The stable task identifier.
     */
    val identifier: String,

    /**
     * The stable execution identifier.
     */
    val executionId: String,

    /**
     * The execution deadline in Unix nanoseconds.
     */
    val deadlineUnixNs: Long,

    /**
     * The optional completion callback bound to the host scheduler.
     */
    val finish: ((RuntimeHostBackgroundTaskResult) -> Unit)? = null,
)

/**
 * The Android process-level background coordinator.
 */
internal class BackgroundCoordinator(
    private val scheduler: BackgroundSchedulerBackend,
    private val registrations: BackgroundRegistrationStore,
    private val monotonicNowNs: () -> Long,
    private val wallClockNowNs: () -> Long,
) {
    /**
     * The coordinator lock.
     */
    private val lock = Any()

    /**
     * The active execution store.
     */
    private val executions = BackgroundExecutionStore(
        monotonicNowNs = monotonicNowNs,
        wallClockNowNs = wallClockNowNs,
    )

    /**
     * The currently attached event sink.
     */
    private var events: BackgroundEvents? = null

    /**
     * The queued events waiting for one sink attachment.
     */
    private val queuedEvents: MutableList<RuntimeHostBackgroundEvent> = mutableListOf()

    /**
     * Attach one live background event sink and flush queued events.
     */
    fun attachEvents(
        events: BackgroundEvents,
    ) {
        // install the live sink and take ownership of queued events
        val queuedEvents = synchronized(lock) {
            this.events = events

            val queuedEvents = queuedEvents.toList()
            this.queuedEvents.clear()

            queuedEvents
        }

        // flush queued events outside the lock
        for (event in queuedEvents) {
            events.notifyBackgroundEvent(event)
        }
    }

    /**
     * Detach the current background event sink.
     */
    fun detachEvents() {
        // clear the live sink and keep future events queued
        synchronized(lock) {
            events = null
        }
    }

    /**
     * Return the scheduler status for this process.
     */
    fun status(): RuntimeHostBackgroundStatusResponse {
        // expose unsupported status when no scheduler backend exists
        val schedulerStatus = scheduler.schedulerStatus()
            ?: return RuntimeHostBackgroundStatusResponse(status = hostStatusNotSupported)

        return RuntimeHostBackgroundStatusResponse(
            status = hostStatusOk,
            schedulerStatus = schedulerStatus,
        )
    }

    /**
     * Return every registered background task descriptor.
     */
    fun list(): RuntimeHostBackgroundListResponse {
        // snapshot the descriptors under the coordinator lock
        val descriptors = synchronized(lock) {
            registrations.listDescriptors()
        }

        return RuntimeHostBackgroundListResponse(
            status = hostStatusOk,
            descriptors = descriptors,
        )
    }

    /**
     * Register one background task and schedule its next execution.
     */
    fun registerTask(
        request: RuntimeHostBackgroundTaskOptions,
    ): Int {
        // reject invalid registrations before mutating host state
        val validationStatus = validateOptions(request)
        if (validationStatus != hostStatusOk) {
            return validationStatus
        }

        // persist the registration and capture whether the scheduler should update it
        val nowUnixNs = wallClockNowNs()
        val storedRegistration = synchronized(lock) {
            registrations.putRegistration(
                options = request,
                nextRegularRunUnixNs = initialRegularRunUnixNs(request.schedule, nowUnixNs),
            )
        }

        if (!storedRegistration.shouldSchedule) {
            return hostStatusOk
        }

        // schedule the next eligible execution
        return scheduleRecord(storedRegistration.record)
    }

    /**
     * Unregister one background task and remove any active scheduler state.
     */
    fun unregister(
        request: RuntimeHostBackgroundUnregisterRequest,
    ): Int {
        val identifier = request.identifier

        // reject blank task identifiers
        if (identifier.isBlank()) {
            return hostStatusInvalidArgument
        }

        val removedRegistration: BackgroundTaskRecord
        val finishCallbacks: List<((RuntimeHostBackgroundTaskResult) -> Unit)?>

        // remove the registration and capture active executions
        synchronized(lock) {
            val record = registrations.removeRegistration(identifier)
                ?: return hostStatusNotFound

            val removedExecutions = executions.removeAllForTask(identifier)

            removedRegistration = record
            finishCallbacks = removedExecutions.map { execution -> execution.finish }
        }

        // finish any in-flight executions outside the lock
        for (finish in finishCallbacks) {
            finish?.invoke(RuntimeHostBackgroundTaskResult.Failure)
        }

        // cancel the platform scheduler entry
        return scheduler.cancel(removedRegistration.descriptor.identifier)
    }

    /**
     * Trigger one synthetic background execution for testing.
     */
    fun triggerTest(
        request: RuntimeHostBackgroundTriggerTestRequest,
    ): RuntimeHostBackgroundTriggerTestResponse {
        val identifier = request.identifier

        // reject blank task identifiers
        if (identifier.isBlank()) {
            return RuntimeHostBackgroundTriggerTestResponse(status = hostStatusInvalidArgument)
        }

        // create one synthetic execution for the registered task
        val execution = synchronized(lock) {
            val record = registrations.resolveRecord(identifier)
                ?: return RuntimeHostBackgroundTriggerTestResponse(status = hostStatusNotFound)

            executions.create(
                identifier = record.descriptor.identifier,
                finish = null,
            )
        }

        // publish the ready event immediately
        publishReadyEvent(execution)

        return RuntimeHostBackgroundTriggerTestResponse(
            status = hostStatusOk,
            isTriggered = true,
        )
    }

    /**
     * Complete one active background execution and schedule the next run.
     */
    fun complete(
        request: RuntimeHostBackgroundCompleteRequest,
    ): Int {
        val executionId = request.executionId
        val result = request.result

        // remove the active execution first
        val execution = synchronized(lock) {
            executions.removeExecution(executionId)
        } ?: return hostStatusNotFound

        // WorkManager already owns retries for scheduler-backed executions
        val shouldScheduleRetryLocally = shouldScheduleRetryLocally(
            result = result,
            hasPlatformCompletion = execution.finish != null,
        )

        // finish the scheduler callback outside the lock
        execution.finish?.invoke(result)

        // resolve the next registration action for this completion
        val record = synchronized(lock) {
            when {
                result == RuntimeHostBackgroundTaskResult.Retry -> {
                    registrations.resolveRecord(execution.identifier)
                }
                else -> {
                    val record = registrations.resolveRecord(execution.identifier)
                        ?: return@synchronized null

                    if (isRecurringSchedule(record.descriptor.schedule)) {
                        record
                    } else {
                        registrations.removeRegistration(execution.identifier)
                        null
                    }
                }
            }
        }

        // schedule the next run for retries and recurring registrations
        if (record != null) {
            when (result) {
                RuntimeHostBackgroundTaskResult.Retry -> {
                    if (shouldScheduleRetryLocally) {
                        scheduleRetry(record)
                    }
                }
                RuntimeHostBackgroundTaskResult.Success,
                RuntimeHostBackgroundTaskResult.Failure,
                -> scheduleRecord(record)
            }
        }

        // normalize all runtime completion classes to a successful host completion
        return when (result) {
            RuntimeHostBackgroundTaskResult.Success,
            RuntimeHostBackgroundTaskResult.Retry,
            RuntimeHostBackgroundTaskResult.Failure,
            -> hostStatusOk
        }
    }

    /**
     * Start one scheduler-delivered Android background execution.
     */
    fun startExecution(
        identifier: String,
        finish: (RuntimeHostBackgroundTaskResult) -> Unit,
    ): String? {
        // create one live execution backed by the platform completion callback
        val execution = synchronized(lock) {
            val record = registrations.resolveRecord(identifier)
                ?: return null

            val updatedRecord = advanceRecordAfterRegularLaunch(
                record = record,
                wallClockNowNs = wallClockNowNs(),
            )

            if (updatedRecord !== record) {
                registrations.putRecord(updatedRecord)
            }

            executions.create(
                identifier = updatedRecord.descriptor.identifier,
                finish = finish,
            )
        }

        // publish the ready event after the execution exists
        publishReadyEvent(execution)

        return execution.executionId
    }

    /**
     * Stop one active scheduler execution and emit expiration when needed.
     */
    fun stopExecution(
        executionId: String,
    ) {
        // remove the active execution by execution identifier
        val execution = synchronized(lock) {
            executions.removeExecution(executionId)
        } ?: return

        // publish expiration before the next schedule is computed
        publishExpiredEvent(execution)

        // finish the platform worker before the next schedule is computed
        execution.finish?.invoke(RuntimeHostBackgroundTaskResult.Failure)

        // resolve the registration for the next run
        val record = synchronized(lock) {
            registrations.resolveRecord(execution.identifier)
        } ?: return

        // schedule the next eligible run
        scheduleRecord(record)
    }

    /**
     * Schedule one record for its next eligible execution time.
     */
    private fun scheduleRecord(
        record: BackgroundTaskRecord,
    ): Int {
        // compute the next regular run time for the declared schedule class
        val nowUnixNs = wallClockNowNs()
        val resolvedRecord = resolveRecordForScheduling(
            record = record,
            wallClockNowNs = nowUnixNs,
        )

        synchronized(lock) {
            if (resolvedRecord !== record) {
                registrations.putRecord(resolvedRecord)
            }
        }

        val runAtUnixNs = scheduledRunAtUnixNs(
            record = resolvedRecord,
            wallClockNowNs = nowUnixNs,
        )

        return scheduler.schedule(resolvedRecord, runAtUnixNs)
    }

    /**
     * Schedule one record for one retry attempt.
     */
    private fun scheduleRetry(
        record: BackgroundTaskRecord,
    ): Int {
        val nowUnixNs = wallClockNowNs()
        val retryRunAtUnixNs = nowUnixNs + backgroundRetryDelayNs
        val runAtUnixNs = if (isRecurringSchedule(record.descriptor.schedule)) {
            val regularRunAtUnixNs = scheduledRunAtUnixNs(
                record = record,
                wallClockNowNs = nowUnixNs,
            )

            minOf(retryRunAtUnixNs, regularRunAtUnixNs)
        } else {
            retryRunAtUnixNs
        }

        return scheduler.schedule(record, runAtUnixNs)
    }

    /**
     * Publish one task-ready event into the runtime ingress surface.
     */
    private fun publishReadyEvent(
        execution: BackgroundExecution,
    ) {
        publishEvent(
            synchronized(lock) {
                executions.event(
                    kind = RuntimeHostBackgroundEventKind.TaskReady,
                    execution = execution,
                )
            },
        )
    }

    /**
     * Publish one task-expired event into the runtime ingress surface.
     */
    private fun publishExpiredEvent(
        execution: BackgroundExecution,
    ) {
        publishEvent(
            synchronized(lock) {
                executions.event(
                    kind = RuntimeHostBackgroundEventKind.TaskExpired,
                    execution = execution,
                )
            },
        )
    }

    /**
     * Deliver one event immediately or queue it until one sink is attached.
     */
    private fun publishEvent(
        event: RuntimeHostBackgroundEvent,
    ) {
        // either take the live sink or queue the event for the next attachment
        val events = synchronized(lock) {
            val events = events
            if (events == null) {
                queuedEvents += event
            }

            events
        }

        // deliver immediately when one sink is attached
        events?.notifyBackgroundEvent(event)
    }

    /**
     * Validate one registration payload against the Android scheduler model.
     */
    private fun validateOptions(
        options: RuntimeHostBackgroundTaskOptions,
    ): Int {
        return validateBackgroundTaskOptions(
            options = options,
            isSchedulerAvailable = scheduler.schedulerStatus() != null,
        )
    }
}

/**
 * Validate one Android background task registration payload.
 */
internal fun validateBackgroundTaskOptions(
    options: RuntimeHostBackgroundTaskOptions,
    isSchedulerAvailable: Boolean,
): Int {
    // reject blank task identifiers
    if (options.identifier.isBlank()) {
        return hostStatusInvalidArgument
    }

    // require valid schedule payloads for the declared schedule class
    when (options.schedule.kind) {
        RuntimeHostBackgroundTaskScheduleKind.Once -> {
            if (options.schedule.repeatIntervalNs != null) {
                return hostStatusInvalidArgument
            }
        }
        RuntimeHostBackgroundTaskScheduleKind.Recurring -> {
            val repeatIntervalNs = options.schedule.repeatIntervalNs
                ?: return hostStatusInvalidArgument

            if (repeatIntervalNs <= 0L) {
                return hostStatusInvalidArgument
            }
        }
    }

    // reject app refresh constraints that do not fit the lightweight work class
    if (options.trigger == RuntimeHostBackgroundTriggerKind.AppRefresh) {
        if (options.network == RuntimeHostBackgroundNetworkRequirement.Unmetered) {
            return hostStatusNotSupported
        }

        if (options.requiresCharging) {
            return hostStatusNotSupported
        }

        if (options.requiresIdle) {
            return hostStatusNotSupported
        }
    }

    // reject requests when no scheduler backend is available
    if (!isSchedulerAvailable) {
        return hostStatusNotSupported
    }

    return hostStatusOk
}

/**
 * Return whether one schedule uses recurring execution.
 */
internal fun isRecurringSchedule(
    schedule: RuntimeHostBackgroundTaskSchedule,
): Boolean {
    return schedule.kind == RuntimeHostBackgroundTaskScheduleKind.Recurring
}

/**
 * Return whether the coordinator must schedule one retry itself.
 */
internal fun shouldScheduleRetryLocally(
    result: RuntimeHostBackgroundTaskResult,
    hasPlatformCompletion: Boolean,
): Boolean {
    return result == RuntimeHostBackgroundTaskResult.Retry && !hasPlatformCompletion
}

/**
 * Return the first regular run target for one declared schedule.
 */
internal fun initialRegularRunUnixNs(
    schedule: RuntimeHostBackgroundTaskSchedule,
    wallClockNowNs: Long,
): Long? {
    val earliestUnixNs = schedule.earliestBeginUnixNs ?: wallClockNowNs

    return when (schedule.kind) {
        RuntimeHostBackgroundTaskScheduleKind.Once -> null
        RuntimeHostBackgroundTaskScheduleKind.Recurring -> maxOf(earliestUnixNs, wallClockNowNs)
    }
}

/**
 * Return one record whose recurring schedule is ready for submission.
 */
internal fun resolveRecordForScheduling(
    record: BackgroundTaskRecord,
    wallClockNowNs: Long,
): BackgroundTaskRecord {
    if (!isRecurringSchedule(record.descriptor.schedule)) {
        return record
    }

    val repeatIntervalNs = record.descriptor.schedule.repeatIntervalNs ?: return record
    val currentRunAtUnixNs =
        record.nextRegularRunUnixNs
            ?: initialRegularRunUnixNs(record.descriptor.schedule, wallClockNowNs)
            ?: return record
    var resolvedRunAtUnixNs = currentRunAtUnixNs

    while (resolvedRunAtUnixNs < wallClockNowNs) {
        resolvedRunAtUnixNs += repeatIntervalNs
    }

    if (resolvedRunAtUnixNs == record.nextRegularRunUnixNs) {
        return record
    }

    return record.copy(
        nextRegularRunUnixNs = resolvedRunAtUnixNs,
    )
}

/**
 * Return one record advanced past the recurring execution that just started.
 */
internal fun advanceRecordAfterRegularLaunch(
    record: BackgroundTaskRecord,
    wallClockNowNs: Long,
): BackgroundTaskRecord {
    if (!isRecurringSchedule(record.descriptor.schedule)) {
        return record
    }

    val repeatIntervalNs = record.descriptor.schedule.repeatIntervalNs ?: return record
    val currentRunAtUnixNs =
        record.nextRegularRunUnixNs
            ?: initialRegularRunUnixNs(record.descriptor.schedule, wallClockNowNs)
            ?: return record
    var nextRunAtUnixNs = currentRunAtUnixNs + repeatIntervalNs

    while (nextRunAtUnixNs <= wallClockNowNs) {
        nextRunAtUnixNs += repeatIntervalNs
    }

    return record.copy(
        nextRegularRunUnixNs = nextRunAtUnixNs,
    )
}

/**
 * Return the next regular run time for one stored record.
 */
internal fun scheduledRunAtUnixNs(
    record: BackgroundTaskRecord,
    wallClockNowNs: Long,
): Long {
    return when (record.descriptor.schedule.kind) {
        RuntimeHostBackgroundTaskScheduleKind.Once -> {
            val earliestUnixNs = record.descriptor.schedule.earliestBeginUnixNs ?: wallClockNowNs

            maxOf(earliestUnixNs, wallClockNowNs)
        }
        RuntimeHostBackgroundTaskScheduleKind.Recurring -> {
            record.nextRegularRunUnixNs
                ?: initialRegularRunUnixNs(record.descriptor.schedule, wallClockNowNs)
                ?: wallClockNowNs
        }
    }
}

/**
 * The live execution state store for one Android background coordinator.
 */
private class BackgroundExecutionStore(
    private val monotonicNowNs: () -> Long,
    private val wallClockNowNs: () -> Long,
) {
    /**
     * The next event sequence number.
     */
    private var nextSequence: Long = 1

    /**
     * The active executions by stable execution identifier.
     */
    private val activeExecutions: MutableMap<String, BackgroundExecution> = mutableMapOf()

    /**
     * Create one active execution and retain it until completion.
     */
    fun create(
        identifier: String,
        finish: ((RuntimeHostBackgroundTaskResult) -> Unit)?,
    ): BackgroundExecution {
        // build one active execution snapshot
        val execution = BackgroundExecution(
            identifier = identifier,
            executionId = buildExecutionId(identifier),
            deadlineUnixNs = wallClockNowNs() + backgroundExecutionDeadlineNs,
            finish = finish,
        )

        // retain the execution until completion or expiration
        activeExecutions[execution.executionId] = execution

        return execution
    }

    /**
     * Remove one active execution by execution identifier.
     */
    fun removeExecution(
        executionId: String,
    ): BackgroundExecution? {
        return activeExecutions.remove(executionId)
    }

    /**
     * Remove every active execution for one task identifier.
     */
    fun removeAllForTask(
        identifier: String,
    ): List<BackgroundExecution> {
        // collect the active execution identifiers first
        val executionIds = activeExecutions
            .values
            .filter { execution -> execution.identifier == identifier }
            .map { execution -> execution.executionId }

        // remove the executions in a stable second pass
        return executionIds.mapNotNull { executionId ->
            activeExecutions.remove(executionId)
        }
    }

    /**
     * Build one event payload for the given execution.
     */
    fun event(
        kind: RuntimeHostBackgroundEventKind,
        execution: BackgroundExecution,
    ): RuntimeHostBackgroundEvent {
        return RuntimeHostBackgroundEvent(
            kind = kind,
            metadata = RuntimeHostBackgroundEventMetadata(
                timestampNs = monotonicNowNs(),
                sequence = nextSequence(),
                identifier = execution.identifier,
                executionId = execution.executionId,
                deadlineUnixNs = execution.deadlineUnixNs,
            ),
        )
    }

    /**
     * Return the next monotonic event sequence number.
     */
    private fun nextSequence(): Long {
        // reserve the current sequence number
        val sequence = nextSequence

        // advance the next sequence number
        nextSequence += 1

        return sequence
    }

    /**
     * Build one stable execution identifier for one task launch.
     */
    private fun buildExecutionId(
        identifier: String,
    ): String {
        return "android-$identifier-${monotonicNowNs()}"
    }
}
