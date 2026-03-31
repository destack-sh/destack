package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEventKind
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Exercise the notification bridge lane.
 */
class NotificationBridgeTest {
    /**
     * Send one notification event through the runtime ingress path.
     */
    @Test
    fun testSendNotificationEventNotifiesRuntime() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(
            sessionHandle,
            runtimeApi,
            notificationTimestampNs = { 42L },
        )
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
        bridge.notifyNotificationEvent(event)

        assertEquals(
            listOf(
                sessionHandle to event.copy(
                    sequence = 1L,
                    timestampNs = 42L,
                ),
            ),
            runtimeApi.notificationEvents,
        )
    }
}
