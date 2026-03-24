package dev.destack.runtime.android.input.text

/**
 * One host text-selection range.
 */
public data class RuntimeHostTextSelectionRange(
    /**
     * The inclusive selection start offset.
     */
    val start: Int,

    /**
     * The exclusive selection end offset.
     */
    val end: Int,
)

/**
 * One host text input configuration submitted by one runtime session.
 */
public data class RuntimeHostTextInputConfiguration(
    /**
     * The stable text input attachment identifier.
     */
    val identifier: String,

    /**
     * Whether the text input session is multiline.
     */
    val isMultiline: Boolean = false,

    /**
     * Whether the text input session is secure or password-like.
     */
    val isSecure: Boolean = false,
)

/**
 * One Android text input open request submitted by one runtime session.
 */
public data class RuntimeHostTextInputOpenRequest(
    /**
     * The text input configuration for the requested attachment.
     */
    val configuration: RuntimeHostTextInputConfiguration,
)

/**
 * One Android text input close request submitted by one runtime session.
 */
public data class RuntimeHostTextInputCloseRequest(
    /**
     * The stable text input attachment identifier to close.
     */
    val identifier: String,
)
