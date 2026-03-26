package dev.destack.runtime.android.module.background

import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Preserve the Android background trigger and completion semantics.
 */
class BackgroundSemanticsTest {
    @Test
    fun testAppRefreshRejectsHeavyProcessingConstraints() {
        val options = RuntimeHostBackgroundTaskOptions(
            identifier = "sync",
            trigger = RuntimeHostBackgroundTriggerKind.AppRefresh,
            schedule = RuntimeHostBackgroundTaskSchedule(
                kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                earliestBeginUnixNs = null,
                repeatIntervalNs = 60_000_000_000,
            ),
            network = RuntimeHostBackgroundNetworkRequirement.Unmetered,
            requiresCharging = false,
            requiresIdle = false,
            conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
        )

        val status = validateBackgroundTaskOptions(
            options = options,
            isSchedulerAvailable = true,
        )

        assertEquals(hostStatusNotSupported, status)
    }

    @Test
    fun testProcessingAllowsHeavyConstraints() {
        val options = RuntimeHostBackgroundTaskOptions(
            identifier = "sync",
            trigger = RuntimeHostBackgroundTriggerKind.Processing,
            schedule = RuntimeHostBackgroundTaskSchedule(
                kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                earliestBeginUnixNs = null,
                repeatIntervalNs = 60_000_000_000,
            ),
            network = RuntimeHostBackgroundNetworkRequirement.Unmetered,
            requiresCharging = true,
            requiresIdle = true,
            conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
        )

        val status = validateBackgroundTaskOptions(
            options = options,
            isSchedulerAvailable = true,
        )

        assertEquals(hostStatusOk, status)
    }

    @Test
    fun testAppRefreshUsesExpeditedWork() {
        assertTrue(usesExpeditedWork(RuntimeHostBackgroundTriggerKind.AppRefresh))
        assertFalse(usesExpeditedWork(RuntimeHostBackgroundTriggerKind.Processing))
    }

    @Test
    fun testRetrySchedulesLocallyOnlyForSyntheticExecutions() {
        assertTrue(
            shouldScheduleRetryLocally(
                result = RuntimeHostBackgroundTaskResult.Retry,
                hasPlatformCompletion = false,
            ),
        )
        assertFalse(
            shouldScheduleRetryLocally(
                result = RuntimeHostBackgroundTaskResult.Retry,
                hasPlatformCompletion = true,
            ),
        )
        assertFalse(
            shouldScheduleRetryLocally(
                result = RuntimeHostBackgroundTaskResult.Success,
                hasPlatformCompletion = false,
            ),
        )
    }

    @Test
    fun testInitialRegularRunUsesEarliestEligibleTimeForRecurringTasks() {
        val runAtUnixNs = initialRegularRunUnixNs(
            schedule =
                RuntimeHostBackgroundTaskSchedule(
                    kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                    earliestBeginUnixNs = 5_000_000_000,
                    repeatIntervalNs = 60_000_000_000,
                ),
            wallClockNowNs = 10_000_000_000,
        )

        assertEquals(10_000_000_000, runAtUnixNs)
    }

    @Test
    fun testResolveRecordForSchedulingRollsRecurringCadenceForwardWithoutDrift() {
        val record = BackgroundTaskRecord(
            descriptor =
                RuntimeHostBackgroundTaskDescriptor(
                    identifier = "sync",
                    trigger = RuntimeHostBackgroundTriggerKind.Processing,
                    schedule =
                        RuntimeHostBackgroundTaskSchedule(
                            kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                            earliestBeginUnixNs = 5_000_000_000,
                            repeatIntervalNs = 60_000_000_000,
                        ),
                    network = RuntimeHostBackgroundNetworkRequirement.Connected,
                    requiresCharging = false,
                    requiresIdle = false,
                    conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
                ),
            nextRegularRunUnixNs = 5_000_000_000,
        )
        val resolvedRecord = resolveRecordForScheduling(
            record = record,
            wallClockNowNs = 10_000_000_000,
        )

        assertEquals(65_000_000_000, resolvedRecord.nextRegularRunUnixNs)
    }

    @Test
    fun testAdvanceRecordAfterRegularLaunchMovesToNextFutureSlot() {
        val record = BackgroundTaskRecord(
            descriptor =
                RuntimeHostBackgroundTaskDescriptor(
                    identifier = "sync",
                    trigger = RuntimeHostBackgroundTriggerKind.Processing,
                    schedule =
                        RuntimeHostBackgroundTaskSchedule(
                            kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                            earliestBeginUnixNs = 10_000_000_000,
                            repeatIntervalNs = 60_000_000_000,
                        ),
                    network = RuntimeHostBackgroundNetworkRequirement.Connected,
                    requiresCharging = false,
                    requiresIdle = false,
                    conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
                ),
            nextRegularRunUnixNs = 10_000_000_000,
        )
        val advancedRecord = advanceRecordAfterRegularLaunch(
            record = record,
            wallClockNowNs = 10_000_000_000,
        )

        assertEquals(70_000_000_000, advancedRecord.nextRegularRunUnixNs)
    }

    @Test
    fun testScheduledRunUsesImmediateTimeForOneShotTasks() {
        val record = BackgroundTaskRecord(
            descriptor =
                RuntimeHostBackgroundTaskDescriptor(
                    identifier = "sync",
                    trigger = RuntimeHostBackgroundTriggerKind.Processing,
                    schedule =
                        RuntimeHostBackgroundTaskSchedule(
                            kind = RuntimeHostBackgroundTaskScheduleKind.Once,
                            earliestBeginUnixNs = null,
                        ),
                    network = RuntimeHostBackgroundNetworkRequirement.Connected,
                    requiresCharging = false,
                    requiresIdle = false,
                    conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
                ),
            nextRegularRunUnixNs = null,
        )
        val runAtUnixNs = scheduledRunAtUnixNs(
            record = record,
            wallClockNowNs = 10_000_000_000,
        )

        assertEquals(10_000_000_000, runAtUnixNs)
    }
}
