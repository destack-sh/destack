package dev.destack.runtime.android.input.text

/**
 * One host text-selection range.
 */
public data class RuntimeHostTextInputRange(
    /**
     * The inclusive selection start offset.
     */
    val startOffset: Int,

    /**
     * The exclusive selection end offset.
     */
    val endOffset: Int,
)

/**
 * One host text-input rectangle in local logical units.
 */
public data class RuntimeHostTextInputRectangle(
    /**
     * The left edge in local logical units.
     */
    val x: Double,

    /**
     * The top edge in local logical units.
     */
    val y: Double,

    /**
     * The rectangle width in local logical units.
     */
    val width: Double,

    /**
     * The rectangle height in local logical units.
     */
    val height: Double,
)

/**
 * One host text-input affine transform into target-local coordinates.
 */
public data class RuntimeHostTextInputTransform2D(
    /**
     * The first-row X coefficient.
     */
    val xx: Double,

    /**
     * The first-row Y coefficient.
     */
    val xy: Double,

    /**
     * The second-row X coefficient.
     */
    val yx: Double,

    /**
     * The second-row Y coefficient.
     */
    val yy: Double,

    /**
     * The translation X component.
     */
    val tx: Double,

    /**
     * The translation Y component.
     */
    val ty: Double,
)

/**
 * One host text-input geometry hint.
 */
public data class RuntimeHostTextInputGeometry(
    /**
     * The local-to-target transform.
     */
    val localToTargetTransform: RuntimeHostTextInputTransform2D,

    /**
     * The full editor rectangle.
     */
    val editorRectangle: RuntimeHostTextInputRectangle,

    /**
     * The caret rectangle when one focused insertion point is known.
     */
    val caretRectangle: RuntimeHostTextInputRectangle? = null,

    /**
     * The composing rectangle when one active composition span is known.
     */
    val composingRectangle: RuntimeHostTextInputRectangle? = null,
)

/**
 * One host text-input type hint.
 */
public enum class RuntimeHostTextInputType {
    TEXT,
    NUMBER,
    EMAIL,
    URL,
    PASSWORD,
    PHONE,
    SEARCH,
}

/**
 * One host text-input state snapshot.
 */
public data class RuntimeHostTextInputState(
    /**
     * The full editor text.
     */
    val text: String,

    /**
     * The current selection range.
     */
    val selection: RuntimeHostTextInputRange,

    /**
     * The composing range when one IME composition is active.
     */
    val composing: RuntimeHostTextInputRange? = null,
)

/**
 * One host text input configuration submitted by one runtime session.
 */
public data class RuntimeHostTextInputConfiguration(
    /**
     * The stable text session identifier.
     */
    val sessionId: Long,

    /**
     * The text input type hint.
     */
    val inputType: RuntimeHostTextInputType = RuntimeHostTextInputType.TEXT,

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

    /**
     * The initial renderer-owned text state.
     */
    val state: RuntimeHostTextInputState,

)

/**
 * One Android text input close request submitted by one runtime session.
 */
public data class RuntimeHostTextInputCloseRequest(
    /**
     * The stable text session identifier to close.
     */
    val sessionId: Long,
)

/**
 * One Android text input geometry update submitted by one runtime session.
 */
public data class RuntimeHostTextInputGeometryRequest(
    /**
     * The stable text session identifier to update.
     */
    val sessionId: Long,

    /**
     * The next text input geometry hint.
     */
    val geometry: RuntimeHostTextInputGeometry,
)

/**
 * One Android text input state update submitted by one runtime session.
 */
public data class RuntimeHostTextInputStateRequest(
    /**
     * The stable text session identifier to update.
     */
    val sessionId: Long,

    /**
     * The next renderer-owned text state.
     */
    val state: RuntimeHostTextInputState,
)
