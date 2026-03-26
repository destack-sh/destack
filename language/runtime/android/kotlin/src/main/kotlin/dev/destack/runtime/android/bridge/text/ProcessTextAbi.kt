package dev.destack.runtime.android.bridge.text

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.bridge.RuntimeHostLibraryLoader
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.input.text.RuntimeHostTextInputState

/**
 * The default text-input ABI resolved through the Android JNI bridge.
 */
internal object ProcessTextAbi : TextAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun notifyTextInputState(
        sessionHandle: HostSessionHandle,
        sessionId: Long,
        state: RuntimeHostTextInputState,
    ): RuntimeAbiStatus {
        val values = nativeNotifyTextInputState(
            sessionHandle.rawValue,
            sessionId,
            state.text,
            state.selection.startOffset,
            state.selection.endOffset,
            state.composing != null,
            state.composing?.startOffset ?: 0,
            state.composing?.endOffset ?: 0,
        )

        return RuntimeAbiStatus(
            code = values[0].toInt(),
            errorId = values[1],
        )
    }

    private external fun nativeNotifyTextInputState(
        sessionHandle: Long,
        sessionId: Long,
        text: String,
        selectionStart: Int,
        selectionEnd: Int,
        hasComposing: Boolean,
        composingStart: Int,
        composingEnd: Int,
    ): LongArray
}
