package dev.destack.runtime.android.input.text

/**
 * One Android text input ingress event delivered into one runtime session.
 */
public data class RuntimeHostTextInputEvent(
    /**
     * The stable text session identifier.
     */
    val sessionId: Long,

    /**
     * The current text state reported by the Android host.
     */
    val state: RuntimeHostTextInputState,
)
