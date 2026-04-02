package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEvent
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEventKind
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEventMetadata
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundConflictPolicy
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundCompleteRequest
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundNetworkRequirement
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskDescriptor
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskOptions
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskResult
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskScheduleKind
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTaskSchedule
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTriggerKind
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundTriggerTestRequest
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundUnregisterRequest
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundListResponse

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Exercise the background bridge lane.
 */
class BackgroundBridgeTest {
    /**
     * Route background requests and ingress events through the attached runtime host.
     */
    @Test
    fun testBackgroundRequestsAndIngressRouteThroughRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val backgroundRequests = BackgroundRequestRecorder().also {
            it.listResponse = RuntimeHostBackgroundListResponse(
                status = 0,
                descriptors = listOf(
                    RuntimeHostBackgroundTaskDescriptor(
                        identifier = "sync",
                        trigger = RuntimeHostBackgroundTriggerKind.Processing,
                        schedule = RuntimeHostBackgroundTaskSchedule(
                            kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                            earliestBeginUnixNs = 0L,
                            repeatIntervalNs = 60_000_000_000L,
                        ),
                        network = RuntimeHostBackgroundNetworkRequirement.Connected,
                        requiresCharging = false,
                        requiresIdle = false,
                        conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
                    ),
                ),
            )
        }
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            backgroundRequests = backgroundRequests,
            intentRequests = IntentRequestRecorder(),
        )
        val event = RuntimeHostBackgroundEvent(
            kind = RuntimeHostBackgroundEventKind.TaskReady,
            metadata = RuntimeHostBackgroundEventMetadata(
                timestampNs = 42L,
                sequence = 7L,
                identifier = "sync",
                executionId = "execution-1",
                deadlineUnixNs = 99L,
            ),
        )

        bridge.attach(runtimeHost)

        val statusResponse = bridge.backgroundStatus(runtimeHost)
        val listResponse = bridge.backgroundList(runtimeHost)
        val registerStatus = bridge.backgroundRegisterTask(
            runtimeHost = runtimeHost,
            request = RuntimeHostBackgroundTaskOptions(
                identifier = "sync",
                trigger = RuntimeHostBackgroundTriggerKind.Processing,
                schedule = RuntimeHostBackgroundTaskSchedule(
                    kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                    earliestBeginUnixNs = 0L,
                    repeatIntervalNs = 60_000_000_000L,
                ),
                network = RuntimeHostBackgroundNetworkRequirement.Connected,
                requiresCharging = false,
                requiresIdle = false,
                conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
            ),
        )
        val unregisterStatus = bridge.backgroundUnregister(
            runtimeHost = runtimeHost,
            request = RuntimeHostBackgroundUnregisterRequest(identifier = "sync"),
        )
        val triggerResponse = bridge.backgroundTriggerTest(
            runtimeHost = runtimeHost,
            request = RuntimeHostBackgroundTriggerTestRequest(identifier = "sync"),
        )
        val completeStatus = bridge.backgroundComplete(
            runtimeHost = runtimeHost,
            request = RuntimeHostBackgroundCompleteRequest(
                executionId = "execution-1",
                result = RuntimeHostBackgroundTaskResult.Success,
            ),
        )

        bridge.notifyBackgroundEvent(event)

        assertEquals(listOf("sync"), listResponse.descriptors.map { descriptor -> descriptor.identifier })
        assertEquals(
            listOf(
                RuntimeHostBackgroundTaskOptions(
                    identifier = "sync",
                    trigger = RuntimeHostBackgroundTriggerKind.Processing,
                    schedule = RuntimeHostBackgroundTaskSchedule(
                        kind = RuntimeHostBackgroundTaskScheduleKind.Recurring,
                        earliestBeginUnixNs = 0L,
                        repeatIntervalNs = 60_000_000_000L,
                    ),
                    network = RuntimeHostBackgroundNetworkRequirement.Connected,
                    requiresCharging = false,
                    requiresIdle = false,
                    conflictPolicy = RuntimeHostBackgroundConflictPolicy.Replace,
                ),
            ),
            backgroundRequests.registerCalls,
        )
        assertEquals(listOf("sync"), backgroundRequests.unregisterCalls)
        assertEquals(listOf("sync"), backgroundRequests.triggerCalls)
        assertEquals(
            listOf("execution-1" to RuntimeHostBackgroundTaskResult.Success),
            backgroundRequests.completeCalls,
        )
        assertEquals(0, statusResponse.status)
        assertEquals(0, registerStatus)
        assertEquals(0, unregisterStatus)
        assertTrue(triggerResponse.isTriggered)
        assertEquals(0, completeStatus)
        assertEquals(listOf(sessionHandle to event), runtimeApi.backgroundEvents)
    }
}
