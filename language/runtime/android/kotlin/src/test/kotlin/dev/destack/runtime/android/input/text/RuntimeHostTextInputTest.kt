package dev.destack.runtime.android

import dev.destack.runtime.android.input.text.RuntimeHostTextInputCloseRequest
import dev.destack.runtime.android.input.text.RuntimeHostTextInputConfiguration
import dev.destack.runtime.android.input.text.RuntimeHostTextInputEvent
import dev.destack.runtime.android.input.text.RuntimeHostTextInputOpenRequest
import dev.destack.runtime.android.input.text.RuntimeHostTextSelectionRange

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create text input requests and one text input ingress event.
 */
class RuntimeHostTextInputTest {
    @Test
    fun testCreateTextInputRequestsAndEvent() {
        val configuration = RuntimeHostTextInputConfiguration(
            identifier = "editor",
            isMultiline = true,
            isSecure = false,
        )
        val openRequest = RuntimeHostTextInputOpenRequest(configuration = configuration)
        val closeRequest = RuntimeHostTextInputCloseRequest(identifier = "editor")
        val event = RuntimeHostTextInputEvent(
            identifier = "editor",
            text = "hello",
            selection = RuntimeHostTextSelectionRange(start = 1, end = 4),
            composing = RuntimeHostTextSelectionRange(start = 1, end = 5),
        )

        assertEquals("editor", openRequest.configuration.identifier)
        assertEquals(true, openRequest.configuration.isMultiline)
        assertEquals("editor", closeRequest.identifier)
        assertEquals("hello", event.text)
        assertEquals(RuntimeHostTextSelectionRange(start = 1, end = 4), event.selection)
        assertEquals(RuntimeHostTextSelectionRange(start = 1, end = 5), event.composing)
    }
}
