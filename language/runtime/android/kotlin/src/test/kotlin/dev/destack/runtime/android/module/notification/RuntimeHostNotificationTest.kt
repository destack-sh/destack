package dev.destack.runtime.android

import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEventKind
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create one notification request and one notification ingress event.
 */
class RuntimeHostNotificationTest {
    @Test
    fun testCreateNotificationRequestAndEvent() {
        val request = RuntimeHostNotificationRequest(
            identifier = "welcome",
            title = "Hello",
            body = "Destack is ready",
        )
        val event = RuntimeHostNotificationEvent(
            identifier = "welcome",
            request = request,
            kind = RuntimeHostNotificationEventKind.Activated,
            actionIdentifier = "open",
        )

        assertEquals("welcome", request.identifier)
        assertEquals("Hello", request.title)
        assertEquals("welcome", event.identifier)
        assertEquals(RuntimeHostNotificationEventKind.Activated, event.kind)
        assertEquals("open", event.actionIdentifier)
    }
}
