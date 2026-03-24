package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEventKind
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest

import org.junit.Assert.assertEquals
import org.junit.Assume.assumeTrue
import org.junit.Test

/**
 * Exercise the notification bridge lane.
 */
class NotificationBridgeTest {
    /**
     * Roundtrip one live notification request through the process runtime ABI.
     */
    @Test
    fun testProcessRuntimeAbiRoundtripsNotificationRequest() {
        assumeTrue("missing DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY", hasRuntimeBridgeLibrary())

        val sessionHandle = RuntimeAbiTest.openTestSession()

        try {
            val bridge = RuntimeBridge(
                sessionHandle = sessionHandle,
                bindings = ProcessRuntimeAbi,
                notificationTimestampNs = { 42L },
            )
            val permissionRequests = PermissionRequestRecorder()
            val documentRequests = DocumentRequestRecorder()
            val notificationRequests = NotificationRequestRecorder()
            val runtimeHost = createBridgeRuntimeHost(
                sessionHandle = sessionHandle,
                permissionRequests = permissionRequests,
                documentRequests = documentRequests,
                intentRequests = IntentRequestRecorder(),
                notificationRequests = notificationRequests,
            )
            val request = RuntimeHostNotificationRequest(
                identifier = "notification-1",
                title = "Title",
                body = "Body",
            )

            try {
                bridge.attach(runtimeHost)
                val submitStatus = RuntimeAbiTest.submitTestNotificationPost(
                    sessionHandle = sessionHandle,
                    identifier = request.identifier,
                    title = request.title,
                    body = request.body,
                )

                assertEquals(0, submitStatus)
                assertEquals(listOf(request), notificationRequests.postedRequests)
            } finally {
                bridge.detach()
            }
        } finally {
            RuntimeAbiTest.closeTestSession(sessionHandle)
        }
    }

    /**
     * Send one notification event through the runtime ingress path.
     */
    @Test
    fun testSendNotificationEventNotifiesRuntime() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
        )
        val event = RuntimeHostNotificationEvent(
            identifier = "test-notification",
            kind = RuntimeHostNotificationEventKind.Activated,
            request = RuntimeHostNotificationRequest(
                identifier = "test-notification",
                title = "Title",
                body = "Body",
            ),
            actionIdentifier = "open",
        )

        bridge.attach(runtimeHost)
        bridge.sendNotificationEvent(event)

        assertEquals(listOf(sessionHandle to event), bindings.notificationEvents)
    }
}
