package dev.destack.runtime.android.input.text

/**
 * One Android text input ingress event delivered into one runtime session.
 */
public data class RuntimeHostTextInputEvent(
    /**
     * The stable text input attachment identifier.
     */
    val identifier: String,

    /**
     * The current text state reported by the Android host.
     */
    val text: String,

    /**
     * The current selection range.
     */
    val selection: RuntimeHostTextSelectionRange,

    /**
     * The optional composing range when one IME composition is active.
     */
    val composing: RuntimeHostTextSelectionRange? = null,
)
