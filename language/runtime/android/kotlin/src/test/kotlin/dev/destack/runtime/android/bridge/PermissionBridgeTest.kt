package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.permission.RuntimeHostPermission
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Exercise the permission bridge lane.
 */
class PermissionBridgeTest {
    /**
     * Route one permission request from one runtime payload into the attached runtime host.
     */
    @Test
    fun testSubmitPermissionRequestRoutesIntoRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val permissionRequests = PermissionRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = permissionRequests,
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        bridge.attach(runtimeHost)
        val status = bridge.permissionRequest(
            requestId = 5,
            permission = RuntimeHostPermission.Camera.rawValue,
        )

        assertEquals(0, status)
        assertEquals(
            listOf(
                RuntimeHostPermissionRequest(
                    requestId = HostRequestId(rawValue = 5),
                    permission = RuntimeHostPermission.Camera,
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
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val permissionRequests = PermissionRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = permissionRequests,
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )

        bridge.attach(runtimeHost)
        val status = bridge.permissionOpenSettings()

        assertEquals(0, status)
        assertEquals(1, permissionRequests.openSettingsCalls)
    }

    /**
     * Send one permission event through the runtime ingress path.
     */
    @Test
    fun testSendPermissionEventNotifiesRuntime() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        val event = RuntimeHostPermissionEvent(
            requestId = HostRequestId(rawValue = 9),
            permission = RuntimeHostPermission.Camera,
            isGranted = true,
        )

        bridge.attach(runtimeHost)
        bridge.notifyPermissionResult(event)

        assertEquals(listOf(sessionHandle to event), runtimeApi.permissionEvents)
    }

}
