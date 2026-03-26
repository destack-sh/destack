package dev.destack.runtime.android.bridge.background

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.module.background.BackgroundEvents
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundConflictPolicy
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEvent
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEventKind
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundNetworkRequirement
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskOptions
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskResult
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskScheduleKind
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskSchedule
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTriggerKind

/**
 * One background bridge lane for one attached Android runtime host.
 */
internal class BackgroundBridge(
    private val sessionHandle: HostSessionHandle,
    private val bindings: BackgroundAbi,
) : BackgroundEvents {
    /**
     * Send one host background event into the runtime ingress path.
     */
    override fun sendBackgroundEvent(
        event: RuntimeHostBackgroundEvent,
    ) {
        val status = bindings.notifyBackgroundEvent(
            sessionHandle = sessionHandle,
            event = event,
        )

        require(status.code == hostStatusOk) {
            "runtime bridge could not deliver background event: code ${status.code}, error ${status.errorId}"
        }
    }

    /**
     * Read the Android background scheduler status through the attached host.
     */
    fun backgroundStatus(
        runtimeHost: RuntimeHost,
    ) = runtimeHost.backgroundRequests.backgroundStatus()

    /**
     * List registered Android background tasks through the attached host.
     */
    fun listBackgroundTasks(
        runtimeHost: RuntimeHost,
    ) = runtimeHost.backgroundRequests.listBackgroundTasks()

    /**
     * Register one Android background task through the attached host.
     */
    fun registerBackgroundTask(
        runtimeHost: RuntimeHost,
        identifier: String,
        trigger: Int,
        scheduleKind: Int,
        hasEarliestBeginUnixNs: Boolean,
        earliestBeginUnixNs: Long,
        hasRepeatIntervalNs: Boolean,
        repeatIntervalNs: Long,
        network: Int,
        requiresCharging: Boolean,
        requiresIdle: Boolean,
        conflictPolicy: Int,
    ): Int {
        val triggerKind = decodeTrigger(trigger) ?: return hostStatusInvalidArgument
        val backgroundScheduleKind = decodeScheduleKind(scheduleKind) ?: return hostStatusInvalidArgument
        val networkRequirement = decodeNetworkRequirement(network) ?: return hostStatusInvalidArgument
        val registrationConflictPolicy =
            decodeConflictPolicy(conflictPolicy) ?: return hostStatusInvalidArgument

        return runtimeHost.backgroundRequests.registerBackgroundTask(
            RuntimeHostBackgroundTaskOptions(
                identifier = identifier,
                trigger = triggerKind,
                schedule = RuntimeHostBackgroundTaskSchedule(
                    kind = backgroundScheduleKind,
                    earliestBeginUnixNs =
                        if (hasEarliestBeginUnixNs) earliestBeginUnixNs else null,
                    repeatIntervalNs = if (hasRepeatIntervalNs) repeatIntervalNs else null,
                ),
                network = networkRequirement,
                requiresCharging = requiresCharging,
                requiresIdle = requiresIdle,
                conflictPolicy = registrationConflictPolicy,
            ),
        )
    }

    /**
     * Unregister one Android background task through the attached host.
     */
    fun unregisterBackgroundTask(
        runtimeHost: RuntimeHost,
        identifier: String,
    ): Int {
        return runtimeHost.backgroundRequests.unregisterBackgroundTask(identifier)
    }

    /**
     * Trigger one Android background task through the attached host.
     */
    fun triggerBackgroundTask(
        runtimeHost: RuntimeHost,
        identifier: String,
    ) = runtimeHost.backgroundRequests.triggerBackgroundTask(identifier)

    /**
     * Complete one Android background task through the attached host.
     */
    fun completeBackgroundTask(
        runtimeHost: RuntimeHost,
        executionId: String,
        result: Int,
    ): Int {
        val taskResult = decodeResult(result) ?: return hostStatusInvalidArgument

        return runtimeHost.backgroundRequests.completeBackgroundTask(
            executionId = executionId,
            result = taskResult,
        )
    }

    /**
     * Decode one bridge trigger value.
     */
    private fun decodeTrigger(
        value: Int,
    ): RuntimeHostBackgroundTriggerKind? {
        return when (value) {
            1 -> RuntimeHostBackgroundTriggerKind.AppRefresh
            2 -> RuntimeHostBackgroundTriggerKind.Processing
            else -> null
        }
    }

    /**
     * Decode one bridge task-result value.
     */
    private fun decodeResult(
        value: Int,
    ): RuntimeHostBackgroundTaskResult? {
        return when (value) {
            1 -> RuntimeHostBackgroundTaskResult.Success
            2 -> RuntimeHostBackgroundTaskResult.Retry
            3 -> RuntimeHostBackgroundTaskResult.Failure
            else -> null
        }
    }

    /**
     * Decode one bridge schedule-kind value.
     */
    private fun decodeScheduleKind(
        value: Int,
    ): RuntimeHostBackgroundTaskScheduleKind? {
        return when (value) {
            1 -> RuntimeHostBackgroundTaskScheduleKind.Once
            2 -> RuntimeHostBackgroundTaskScheduleKind.Recurring
            else -> null
        }
    }

    /**
     * Decode one bridge network value.
     */
    private fun decodeNetworkRequirement(
        value: Int,
    ): RuntimeHostBackgroundNetworkRequirement? {
        return when (value) {
            1 -> RuntimeHostBackgroundNetworkRequirement.None
            2 -> RuntimeHostBackgroundNetworkRequirement.Connected
            3 -> RuntimeHostBackgroundNetworkRequirement.Unmetered
            else -> null
        }
    }

    /**
     * Decode one bridge conflict policy value.
     */
    private fun decodeConflictPolicy(
        value: Int,
    ): RuntimeHostBackgroundConflictPolicy? {
        return when (value) {
            1 -> RuntimeHostBackgroundConflictPolicy.Replace
            2 -> RuntimeHostBackgroundConflictPolicy.Keep
            else -> null
        }
    }
}
