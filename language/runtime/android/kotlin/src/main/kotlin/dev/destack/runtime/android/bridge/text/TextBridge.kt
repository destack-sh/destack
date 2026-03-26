package dev.destack.runtime.android.bridge.text

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.input.text.RuntimeHostTextInputCloseRequest
import dev.destack.runtime.android.input.text.RuntimeHostTextInputConfiguration
import dev.destack.runtime.android.input.text.RuntimeHostTextInputEvent
import dev.destack.runtime.android.input.text.RuntimeHostTextInputGeometry
import dev.destack.runtime.android.input.text.RuntimeHostTextInputGeometryRequest
import dev.destack.runtime.android.input.text.RuntimeHostTextInputOpenRequest
import dev.destack.runtime.android.input.text.RuntimeHostTextInputRectangle
import dev.destack.runtime.android.input.text.RuntimeHostTextInputRange
import dev.destack.runtime.android.input.text.RuntimeHostTextInputState
import dev.destack.runtime.android.input.text.RuntimeHostTextInputStateRequest
import dev.destack.runtime.android.input.text.RuntimeHostTextInputTransform2D
import dev.destack.runtime.android.input.text.RuntimeHostTextInputType
import dev.destack.runtime.android.input.text.TextInputEvents

/**
 * One text-input bridge lane for one attached Android runtime host.
 */
internal class TextBridge(
    private val sessionHandle: HostSessionHandle,
    private val bindings: TextAbi,
) : TextInputEvents {
    /**
     * Send one host text-input event into the runtime ingress path.
     */
    override fun sendTextInputEvent(
        event: RuntimeHostTextInputEvent,
    ) {
        val status = bindings.notifyTextInputState(
            sessionHandle = sessionHandle,
            sessionId = event.sessionId,
            state = event.state,
        )

        require(status.code == hostStatusOk) {
            "runtime bridge could not deliver text-input state: code ${status.code}, error ${status.errorId}"
        }
    }

    /**
     * Open one Android text-input session through the attached host.
     */
    fun openTextInput(
        runtimeHost: RuntimeHost,
        sessionId: Long,
        inputType: Int,
        isMultiline: Boolean,
        isSecure: Boolean,
        text: String,
        selectionStart: Int,
        selectionEnd: Int,
        hasComposing: Boolean,
        composingStart: Int,
        composingEnd: Int,
    ): Int {
        return runtimeHost.textInputRequests.openTextInput(
            RuntimeHostTextInputOpenRequest(
                configuration = RuntimeHostTextInputConfiguration(
                    sessionId = sessionId,
                    inputType = decodeInputType(inputType),
                    isMultiline = isMultiline,
                    isSecure = isSecure,
                ),
                state = decodeState(
                    text = text,
                    selectionStart = selectionStart,
                    selectionEnd = selectionEnd,
                    hasComposing = hasComposing,
                    composingStart = composingStart,
                    composingEnd = composingEnd,
                ),
            ),
        )
    }

    /**
     * Close one Android text-input session through the attached host.
     */
    fun closeTextInput(
        runtimeHost: RuntimeHost,
        sessionId: Long,
    ): Int {
        return runtimeHost.textInputRequests.closeTextInput(
            RuntimeHostTextInputCloseRequest(
                sessionId = sessionId,
            ),
        )
    }

    /**
     * Update one Android text-input geometry through the attached host.
     */
    fun setTextInputGeometry(
        runtimeHost: RuntimeHost,
        sessionId: Long,
        transformXx: Double,
        transformXy: Double,
        transformYx: Double,
        transformYy: Double,
        transformTx: Double,
        transformTy: Double,
        editorX: Double,
        editorY: Double,
        editorWidth: Double,
        editorHeight: Double,
        hasCaretRectangle: Boolean,
        caretX: Double,
        caretY: Double,
        caretWidth: Double,
        caretHeight: Double,
        hasComposingRectangle: Boolean,
        composingX: Double,
        composingY: Double,
        composingWidth: Double,
        composingHeight: Double,
    ): Int {
        return runtimeHost.textInputRequests.setTextInputGeometry(
            RuntimeHostTextInputGeometryRequest(
                sessionId = sessionId,
                geometry = RuntimeHostTextInputGeometry(
                    localToTargetTransform = RuntimeHostTextInputTransform2D(
                        xx = transformXx,
                        xy = transformXy,
                        yx = transformYx,
                        yy = transformYy,
                        tx = transformTx,
                        ty = transformTy,
                    ),
                    editorRectangle = RuntimeHostTextInputRectangle(
                        x = editorX,
                        y = editorY,
                        width = editorWidth,
                        height = editorHeight,
                    ),
                    caretRectangle = if (hasCaretRectangle) {
                        RuntimeHostTextInputRectangle(
                            x = caretX,
                            y = caretY,
                            width = caretWidth,
                            height = caretHeight,
                        )
                    } else {
                        null
                    },
                    composingRectangle = if (hasComposingRectangle) {
                        RuntimeHostTextInputRectangle(
                            x = composingX,
                            y = composingY,
                            width = composingWidth,
                            height = composingHeight,
                        )
                    } else {
                        null
                    },
                ),
            ),
        )
    }

    /**
     * Update one Android text-input state through the attached host.
     */
    fun setTextInputState(
        runtimeHost: RuntimeHost,
        sessionId: Long,
        text: String,
        selectionStart: Int,
        selectionEnd: Int,
        hasComposing: Boolean,
        composingStart: Int,
        composingEnd: Int,
    ): Int {
        return runtimeHost.textInputRequests.setTextInputState(
            RuntimeHostTextInputStateRequest(
                sessionId = sessionId,
                state = decodeState(
                    text = text,
                    selectionStart = selectionStart,
                    selectionEnd = selectionEnd,
                    hasComposing = hasComposing,
                    composingStart = composingStart,
                    composingEnd = composingEnd,
                ),
            ),
        )
    }

    /**
     * Decode one bridge text-input type value.
     */
    private fun decodeInputType(
        value: Int,
    ): RuntimeHostTextInputType {
        return RuntimeHostTextInputType.entries.firstOrNull { entry ->
            entry.ordinal == value
        } ?: RuntimeHostTextInputType.TEXT
    }

    /**
     * Decode one bridge text-input state payload.
     */
    private fun decodeState(
        text: String,
        selectionStart: Int,
        selectionEnd: Int,
        hasComposing: Boolean,
        composingStart: Int,
        composingEnd: Int,
    ): RuntimeHostTextInputState {
        return RuntimeHostTextInputState(
            text = text,
            selection = RuntimeHostTextInputRange(
                startOffset = selectionStart,
                endOffset = selectionEnd,
            ),
            composing = if (hasComposing) {
                RuntimeHostTextInputRange(
                    startOffset = composingStart,
                    endOffset = composingEnd,
                )
            } else {
                null
            },
        )
    }
}
