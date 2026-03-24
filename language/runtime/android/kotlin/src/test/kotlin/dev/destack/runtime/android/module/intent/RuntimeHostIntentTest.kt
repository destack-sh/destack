package dev.destack.runtime.android

import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
import dev.destack.runtime.android.module.intent.RuntimeHostIntentPayload

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create one intent ingress event.
 */
class RuntimeHostIntentTest {
    @Test
    fun testCreateIntentEvent() {
        val event = RuntimeHostIntentEvent(
            source = "dev.destack.host",
            payload = RuntimeHostIntentPayload.OpenUrl(url = "https://destack.dev"),
        )

        assertEquals("dev.destack.host", event.source)
        assertEquals(
            RuntimeHostIntentPayload.OpenUrl(url = "https://destack.dev"),
            event.payload,
        )
    }
}
