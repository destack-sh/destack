package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

import org.junit.Assert.assertEquals
import org.junit.Assume.assumeTrue
import org.junit.Test

/**
 * Exercise the permission bridge lane.
 */
class PermissionBridgeTest {
    /**
     * Roundtrip one live permission request and one completion through the process runtime ABI.
     */
    @Test
    fun testProcessRuntimeAbiRoundtripsPermissionRequest() {
        assumeTrue("missing DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY", hasRuntimeBridgeLibrary())

        val sessionHandle = RuntimeAbiTest.openTestSession()

        try {
            val bridge = RuntimeBridge(
                sessionHandle = sessionHandle,
                bindings = ProcessRuntimeAbi,
            )
            val permissionRequests = PermissionRequestRecorder()
            val documentRequests = DocumentRequestRecorder()
            val runtimeHost = createBridgeRuntimeHost(
                sessionHandle = sessionHandle,
                permissionRequests = permissionRequests,
                documentRequests = documentRequests,
                intentRequests = IntentRequestRecorder(),
            )
            val expectedRequest = RuntimeHostPermissionRequest(
                requestId = HostRequestId(rawValue = 5),
                permission = "camera",
            )

            try {
                bridge.attach(runtimeHost)

                val submitStatus = RuntimeAbiTest.submitTestPermissionRequest(
                    sessionHandle,
                    requestId = 5,
                    permissions = arrayOf("camera"),
                )

                assertEquals(0, submitStatus)
                assertEquals(listOf(expectedRequest), permissionRequests.requests)
            } finally {
                bridge.detach()
            }
        } finally {
            RuntimeAbiTest.closeTestSession(sessionHandle)
        }
    }

    /**
     * Route one permission request from one runtime payload into the attached runtime host.
     */
    @Test
    fun testSubmitPermissionRequestRoutesIntoRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val permissionRequests = PermissionRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = permissionRequests,
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        bridge.attach(runtimeHost)
        val status = bridge.submitPermissionRequest(
            requestId = 5,
            permissions = arrayOf("camera"),
        )

        assertEquals(0, status)
        assertEquals(
            listOf(
                RuntimeHostPermissionRequest(
                    requestId = HostRequestId(rawValue = 5),
                    permission = "camera",
                ),
            ),
            permissionRequests.requests,
        )
    }

    /**
     * Open the permission settings surface through the attached runtime host.
     */
    @Test
    fun testOpenPermissionSettingsRoutesIntoRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val permissionRequests = PermissionRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = permissionRequests,
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )

        bridge.attach(runtimeHost)
        val status = bridge.openPermissionSettings()

        assertEquals(0, status)
        assertEquals(1, permissionRequests.openSettingsCalls)
    }

    /**
     * Send one permission event through the runtime ingress path.
     */
    @Test
    fun testSendPermissionEventNotifiesRuntime() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        val event = RuntimeHostPermissionEvent(
            requestId = HostRequestId(rawValue = 9),
            permission = "camera",
            isGranted = true,
        )

        bridge.attach(runtimeHost)
        bridge.sendPermissionEvent(event)

        assertEquals(listOf(sessionHandle to event), bindings.permissionEvents)
    }

}
