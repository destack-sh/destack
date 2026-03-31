package dev.destack.runtime.android.module.text

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create text input requests and one text input ingress event.
 */
class RuntimeHostTextInputTest {
    @Test
    fun testCreateTextInputRequestsAndEvent() {
        val configuration = RuntimeHostTextInputConfiguration(
            sessionId = 7,
            inputType = RuntimeHostTextInputType.TEXT,
            isMultiline = true,
            isSecure = false,
        )
        val state = RuntimeHostTextInputState(
            text = "hello",
            selection = RuntimeHostTextInputRange(startOffset = 1, endOffset = 4),
            composing = RuntimeHostTextInputRange(startOffset = 1, endOffset = 5),
        )
        val geometry = RuntimeHostTextInputGeometry(
            localToTargetTransform = RuntimeHostTextInputTransform2D(
                xx = 1.0,
                xy = 0.0,
                yx = 0.0,
                yy = 1.0,
                tx = 0.0,
                ty = 0.0,
            ),
            editorRectangle = RuntimeHostTextInputRectangle(
                x = 10.0,
                y = 20.0,
                width = 300.0,
                height = 120.0,
            ),
            caretRectangle = RuntimeHostTextInputRectangle(
                x = 16.0,
                y = 24.0,
                width = 2.0,
                height = 18.0,
            ),
        )
        val openRequest = RuntimeHostTextInputOpenRequest(
            configuration = configuration,
            state = state,
        )
        val closeRequest = RuntimeHostTextInputCloseRequest(sessionId = 7)
        val geometryRequest = RuntimeHostTextInputGeometryRequest(
            sessionId = 7,
            geometry = geometry,
        )
        val stateRequest = RuntimeHostTextInputStateRequest(sessionId = 7, state = state)
        val event = RuntimeHostTextInputEvent(
            sessionId = 7,
            state = state,
        )

        assertEquals(7, openRequest.configuration.sessionId)
        assertEquals(RuntimeHostTextInputType.TEXT, openRequest.configuration.inputType)
        assertEquals(true, openRequest.configuration.isMultiline)
        assertEquals("hello", openRequest.state.text)
        assertEquals(7, closeRequest.sessionId)
        assertEquals(7, geometryRequest.sessionId)
        assertEquals(geometry, geometryRequest.geometry)
        assertEquals(state, stateRequest.state)
        assertEquals("hello", event.state.text)
        assertEquals(RuntimeHostTextInputRange(startOffset = 1, endOffset = 4), event.state.selection)
        assertEquals(RuntimeHostTextInputRange(startOffset = 1, endOffset = 5), event.state.composing)
    }
}
