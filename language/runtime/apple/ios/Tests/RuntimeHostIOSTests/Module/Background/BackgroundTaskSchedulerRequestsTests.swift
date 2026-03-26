import RuntimeHostAppleCore
import Testing

@testable import RuntimeHostIOS

@Test
func testInitialRegularRunUsesEarliestEligibleTimeForRecurringTasks() {
  let schedule = RuntimeHostBackgroundTaskSchedule(
    kind: .recurring,
    earliestBeginUnixNs: 5_000_000_000,
    repeatIntervalNs: 60_000_000_000
  )

  let initialRunUnixNs = BackgroundTaskSchedulerRequests.initialRegularRunUnixNs(
    schedule,
    wallClockNowNs: 10_000_000_000
  )

  #expect(initialRunUnixNs == 10_000_000_000)
}

@Test
func testResolvedRecordForSchedulingRollsRecurringCadenceForwardWithoutDrift() {
  let record = BackgroundTaskRecord(
    descriptor: RuntimeHostBackgroundTaskDescriptor(
      identifier: "sync",
      trigger: .processing,
      schedule: RuntimeHostBackgroundTaskSchedule(
        kind: .recurring,
        earliestBeginUnixNs: 5_000_000_000,
        repeatIntervalNs: 60_000_000_000
      ),
      network: .connected,
      requiresCharging: false,
      requiresIdle: false,
      conflictPolicy: .replace
    ),
    nextRegularRunUnixNs: 5_000_000_000
  )

  let resolvedRecord = BackgroundTaskSchedulerRequests.resolvedRecordForScheduling(
    record,
    wallClockNowNs: 10_000_000_000
  )

  #expect(resolvedRecord.nextRegularRunUnixNs == 65_000_000_000)
}

@Test
func testAdvanceRecordAfterRegularLaunchMovesToNextFutureSlot() {
  let record = BackgroundTaskRecord(
    descriptor: RuntimeHostBackgroundTaskDescriptor(
      identifier: "sync",
      trigger: .processing,
      schedule: RuntimeHostBackgroundTaskSchedule(
        kind: .recurring,
        earliestBeginUnixNs: 10_000_000_000,
        repeatIntervalNs: 60_000_000_000
      ),
      network: .connected,
      requiresCharging: false,
      requiresIdle: false,
      conflictPolicy: .replace
    ),
    nextRegularRunUnixNs: 10_000_000_000
  )

  let advancedRecord = BackgroundTaskSchedulerRequests.advanceRecordAfterRegularLaunch(
    record,
    wallClockNowNs: 10_000_000_000
  )

  #expect(advancedRecord.nextRegularRunUnixNs == 70_000_000_000)
}

@Test
func testRetriedDescriptorUsesEarlierRetryDelayWithoutRewritingRegularCadence() {
  let record = BackgroundTaskRecord(
    descriptor: RuntimeHostBackgroundTaskDescriptor(
      identifier: "sync",
      trigger: .processing,
      schedule: RuntimeHostBackgroundTaskSchedule(
        kind: .recurring,
        earliestBeginUnixNs: 120_000_000_000,
        repeatIntervalNs: 60_000_000_000
      ),
      network: .connected,
      requiresCharging: false,
      requiresIdle: false,
      conflictPolicy: .replace
    ),
    nextRegularRunUnixNs: 120_000_000_000
  )

  let scheduledDescriptor = BackgroundTaskSchedulerRequests.retriedDescriptor(
    record,
    wallClockNowNs: 10_000_000_000
  )

  #expect(scheduledDescriptor.schedule.earliestBeginUnixNs == 20_000_000_000)
}
