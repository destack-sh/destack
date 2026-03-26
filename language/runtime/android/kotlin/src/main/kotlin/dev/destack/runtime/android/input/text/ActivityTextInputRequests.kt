package dev.destack.runtime.android.input.text

import android.content.Context
import android.graphics.Matrix
import android.text.Editable
import android.text.InputType
import android.text.Selection
import android.text.TextWatcher
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout

import androidx.activity.ComponentActivity

import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.hostStatusNotSupported

import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.max
import kotlin.math.min

/**
 * The Android text-input request surface backed by one attached activity.
 */
internal class ActivityTextInputRequests(
    private val activity: ComponentActivity?,
    private val events: TextInputEvents,
) : TextInputRequests {
    /**
     * One tracked text-input session record.
     */
    private data class SessionRecord(
        val configuration: RuntimeHostTextInputConfiguration,
        var geometry: RuntimeHostTextInputGeometry?,
        var state: RuntimeHostTextInputState,
    )

    /**
     * One hidden host editor that reports text and selection changes.
     */
    private class HostTextInputEditText(
        context: Context,
    ) : EditText(context) {
        var onStateChanged: (() -> Unit)? = null

        init {
            addTextChangedListener(
                object : TextWatcher {
                    override fun beforeTextChanged(
                        text: CharSequence?,
                        start: Int,
                        count: Int,
                        after: Int,
                    ) {}

                    override fun onTextChanged(
                        text: CharSequence?,
                        start: Int,
                        before: Int,
                        count: Int,
                    ) {}

                    override fun afterTextChanged(
                        editable: Editable?,
                    ) {
                        onStateChanged?.invoke()
                    }
                },
            )
        }

        override fun onSelectionChanged(
            selectionStart: Int,
            selectionEnd: Int,
        ) {
            super.onSelectionChanged(selectionStart, selectionEnd)

            onStateChanged?.invoke()
        }
    }

    private val sessions: MutableMap<Long, SessionRecord> = mutableMapOf()
    private var activeSessionId: Long? = null
    private var isApplyingHostState: Boolean = false
    private var hiddenEditor: HostTextInputEditText? = null

    override fun openTextInput(
        request: RuntimeHostTextInputOpenRequest,
    ): Int {
        val activity = activity ?: return hostStatusNotSupported
        val hiddenEditor = ensureHiddenEditor(activity)
        val sessionId = request.configuration.sessionId

        // open or replace the tracked session
        sessions[sessionId] = SessionRecord(
            configuration = request.configuration,
            geometry = sessions[sessionId]?.geometry,
            state = request.state,
        )
        activeSessionId = sessionId

        // apply the full host editor shape before focusing
        configureHiddenEditor(hiddenEditor, request.configuration)
        sessions.getValue(sessionId).geometry?.let { geometry ->
            applyTextInputGeometry(hiddenEditor, geometry)
        }
        applyTextInputState(hiddenEditor, request.state)
        hiddenEditor.post {
            // focus and ime
            hiddenEditor.requestFocus()
            inputMethodManager(activity).showSoftInput(
                hiddenEditor,
                InputMethodManager.SHOW_IMPLICIT,
            )

            // geometry hints
            synchronizeImeGeometry(hiddenEditor)
        }

        return hostStatusOk
    }

    override fun closeTextInput(
        request: RuntimeHostTextInputCloseRequest,
    ): Int {
        if (sessions.remove(request.sessionId) == null) {
            return hostStatusNotFound
        }

        // clear the focused session when the closed session was active
        if (activeSessionId == request.sessionId) {
            activeSessionId = null

            val hiddenEditor = hiddenEditor
            val activity = activity
            if (hiddenEditor != null && activity != null) {
                inputMethodManager(activity).hideSoftInputFromWindow(hiddenEditor.windowToken, 0)
                hiddenEditor.clearFocus()
            }
        }
        return hostStatusOk
    }

    override fun setTextInputGeometry(
        request: RuntimeHostTextInputGeometryRequest,
    ): Int {
        val session = sessions[request.sessionId] ?: return hostStatusNotFound
        session.geometry = request.geometry

        // keep the focused editor aligned with the current renderer hint
        if (activeSessionId == request.sessionId) {
            hiddenEditor?.let { hiddenEditor ->
                applyTextInputGeometry(hiddenEditor, request.geometry)
                synchronizeImeGeometry(hiddenEditor)
            }
        }

        return hostStatusOk
    }

    override fun setTextInputState(
        request: RuntimeHostTextInputStateRequest,
    ): Int {
        val session = sessions[request.sessionId] ?: return hostStatusNotFound
        session.state = request.state

        // push renderer-owned state into the focused host editor
        if (activeSessionId == request.sessionId) {
            hiddenEditor?.let { hiddenEditor ->
                applyTextInputState(hiddenEditor, request.state)
                synchronizeImeGeometry(hiddenEditor)
            }
        }

        return hostStatusOk
    }

    /**
     * Tear down the hidden host editor for this activity attachment.
     */
    fun detach() {
        val hiddenEditor = hiddenEditor ?: return
        val parent = hiddenEditor.parent as? ViewGroup ?: return

        parent.removeView(hiddenEditor)
        this.hiddenEditor = null
        activeSessionId = null
        sessions.clear()
    }

    /**
     * Resolve or install the hidden host editor for one activity.
     */
    private fun ensureHiddenEditor(
        activity: ComponentActivity,
    ): HostTextInputEditText {
        val existing = hiddenEditor
        if (existing != null) {
            return existing
        }

        val editor = HostTextInputEditText(activity).apply {
            alpha = 0.01f
            isSingleLine = true
            visibility = View.VISIBLE
            onStateChanged = { handleEditorStateChanged() }
            layoutParams = FrameLayout.LayoutParams(1, 1)
        }
        val root = activity.findViewById<ViewGroup>(android.R.id.content) ?: return editor

        root.addView(
            editor,
            FrameLayout.LayoutParams(1, 1),
        )
        hiddenEditor = editor

        return editor
    }

    /**
     * Reconfigure the hidden host editor for one session configuration.
     */
    private fun configureHiddenEditor(
        hiddenEditor: HostTextInputEditText,
        configuration: RuntimeHostTextInputConfiguration,
    ) {
        hiddenEditor.inputType = inputType(configuration)
        hiddenEditor.isSingleLine = !configuration.isMultiline
    }

    /**
     * Apply one renderer text-geometry hint to the hidden host editor.
     */
    private fun applyTextInputGeometry(
        hiddenEditor: HostTextInputEditText,
        geometry: RuntimeHostTextInputGeometry,
    ) {
        val editorRectangle = transformRectangleToBounds(
            rectangle = geometry.editorRectangle,
            transform = geometry.localToTargetTransform,
        )

        val layoutParams = FrameLayout.LayoutParams(
            max(editorRectangle.width, 1),
            max(editorRectangle.height, 1),
        ).apply {
            leftMargin = editorRectangle.x
            topMargin = editorRectangle.y
        }

        hiddenEditor.layoutParams = layoutParams
    }

    /**
     * Project one local rectangle into one axis-aligned target-space frame.
     */
    private fun transformRectangleToBounds(
        rectangle: RuntimeHostTextInputRectangle,
        transform: RuntimeHostTextInputTransform2D,
    ): ProjectedRectangle {
        val firstX = projectX(rectangle.x, rectangle.y, transform)
        val firstY = projectY(rectangle.x, rectangle.y, transform)
        val secondX = projectX(rectangle.x + rectangle.width, rectangle.y, transform)
        val secondY = projectY(rectangle.x + rectangle.width, rectangle.y, transform)
        val thirdX = projectX(rectangle.x, rectangle.y + rectangle.height, transform)
        val thirdY = projectY(rectangle.x, rectangle.y + rectangle.height, transform)
        val fourthX = projectX(rectangle.x + rectangle.width, rectangle.y + rectangle.height, transform)
        val fourthY = projectY(rectangle.x + rectangle.width, rectangle.y + rectangle.height, transform)

        val minX = floor(min(min(firstX, secondX), min(thirdX, fourthX))).toInt()
        val minY = floor(min(min(firstY, secondY), min(thirdY, fourthY))).toInt()
        val maxX = ceil(max(max(firstX, secondX), max(thirdX, fourthX))).toInt()
        val maxY = ceil(max(max(firstY, secondY), max(thirdY, fourthY))).toInt()

        return ProjectedRectangle(
            x = minX,
            y = minY,
            width = max(maxX - minX, 1),
            height = max(maxY - minY, 1),
        )
    }

    /**
     * Apply one affine transform to one X coordinate.
     */
    private fun projectX(
        x: Double,
        y: Double,
        transform: RuntimeHostTextInputTransform2D,
    ): Double {
        return transform.xx * x + transform.xy * y + transform.tx
    }

    /**
     * Apply one affine transform to one Y coordinate.
     */
    private fun projectY(
        x: Double,
        y: Double,
        transform: RuntimeHostTextInputTransform2D,
    ): Double {
        return transform.yx * x + transform.yy * y + transform.ty
    }

    /**
     * One projected axis-aligned rectangle.
     */
    private data class ProjectedRectangle(
        val x: Int,
        val y: Int,
        val width: Int,
        val height: Int,
    )

    /**
     * Apply one renderer-owned text state to the hidden host editor.
     */
    private fun applyTextInputState(
        hiddenEditor: HostTextInputEditText,
        state: RuntimeHostTextInputState,
    ) {
        isApplyingHostState = true

        try {
            hiddenEditor.setText(state.text)

            val editable = hiddenEditor.text
            val textLength = editable.length
            val selectionStart = state.selection.startOffset.coerceIn(0, textLength)
            val selectionEnd = state.selection.endOffset.coerceIn(selectionStart, textLength)
            Selection.setSelection(editable, selectionStart, selectionEnd)

            val inputConnection = BaseInputConnection(hiddenEditor, true)
            val composing = state.composing

            // keep the ime composing range aligned when one exists
            if (composing == null) {
                inputConnection.finishComposingText()
            } else {
                inputConnection.setComposingRegion(
                    composing.startOffset.coerceIn(0, textLength),
                    composing.endOffset.coerceIn(0, textLength),
                )
            }
        } finally {
            isApplyingHostState = false
        }
    }

    /**
     * Forward one host-originated text change into the runtime ingress surface.
     */
    private fun handleEditorStateChanged() {
        if (isApplyingHostState) {
            return
        }

        val sessionId = activeSessionId ?: return
        val hiddenEditor = hiddenEditor ?: return
        val session = sessions[sessionId] ?: return
        val state = readTextInputState(hiddenEditor)

        session.state = state

        // geometry hints
        synchronizeImeGeometry(hiddenEditor)

        events.sendTextInputEvent(
            RuntimeHostTextInputEvent(
                sessionId = sessionId,
                state = state,
            ),
        )
    }

    /**
     * Push the latest renderer geometry into the active IME session.
     */
    private fun synchronizeImeGeometry(
        hiddenEditor: HostTextInputEditText,
    ) {
        val activity = activity ?: return
        val sessionId = activeSessionId ?: return
        val session = sessions[sessionId] ?: return
        val geometry = session.geometry ?: return
        val cursorAnchorInfo = buildCursorAnchorInfo(
            state = session.state,
            geometry = geometry,
        )

        inputMethodManager(activity).updateCursorAnchorInfo(
            hiddenEditor,
            cursorAnchorInfo,
        )
    }

    /**
     * Build one ime cursor-anchor snapshot from renderer-owned state and geometry.
     */
    private fun buildCursorAnchorInfo(
        state: RuntimeHostTextInputState,
        geometry: RuntimeHostTextInputGeometry,
    ): CursorAnchorInfo {
        val builder = CursorAnchorInfo.Builder()

        // selection
        builder.setSelectionRange(
            state.selection.startOffset,
            state.selection.endOffset,
        )

        // transform
        val matrix = Matrix().apply {
            setValues(
                floatArrayOf(
                    geometry.localToTargetTransform.xx.toFloat(),
                    geometry.localToTargetTransform.xy.toFloat(),
                    geometry.localToTargetTransform.tx.toFloat(),
                    geometry.localToTargetTransform.yx.toFloat(),
                    geometry.localToTargetTransform.yy.toFloat(),
                    geometry.localToTargetTransform.ty.toFloat(),
                    0f,
                    0f,
                    1f,
                ),
            )
        }
        builder.setMatrix(matrix)

        // caret
        geometry.caretRectangle?.let { caretRectangle ->
            builder.setInsertionMarkerLocation(
                caretRectangle.x.toFloat(),
                caretRectangle.y.toFloat(),
                (caretRectangle.y + caretRectangle.height).toFloat(),
                (caretRectangle.y + caretRectangle.height).toFloat(),
                0,
            )
        }

        // composing
        val composing = state.composing
        val composingRectangle = geometry.composingRectangle
        if (composing != null && composingRectangle != null) {
            val composingText = substringForRange(
                text = state.text,
                range = composing,
            )
            val composingLength = max(composing.endOffset - composing.startOffset, 1)
            val characterWidth = composingRectangle.width / composingLength.toDouble()

            builder.setComposingText(
                composing.startOffset,
                composingText,
            )

            for (index in 0 until composingLength) {
                val characterLeft = composingRectangle.x + characterWidth * index.toDouble()
                val characterRight = characterLeft + characterWidth

                builder.addCharacterBounds(
                    composing.startOffset + index,
                    characterLeft.toFloat(),
                    composingRectangle.y.toFloat(),
                    characterRight.toFloat(),
                    (composingRectangle.y + composingRectangle.height).toFloat(),
                    0,
                )
            }
        }

        return builder.build()
    }

    /**
     * Read one substring for one runtime text range.
     */
    private fun substringForRange(
        text: String,
        range: RuntimeHostTextInputRange,
    ): String {
        val startOffset = range.startOffset.coerceIn(0, text.length)
        val endOffset = range.endOffset.coerceIn(startOffset, text.length)

        return text.substring(startOffset, endOffset)
    }

    /**
     * Read the current host editor state from one hidden editor.
     */
    private fun readTextInputState(
        hiddenEditor: HostTextInputEditText,
    ): RuntimeHostTextInputState {
        val editable = hiddenEditor.text
        val composingStart = BaseInputConnection.getComposingSpanStart(editable)
        val composingEnd = BaseInputConnection.getComposingSpanEnd(editable)
        val composing = if (composingStart >= 0 && composingEnd >= composingStart) {
            RuntimeHostTextInputRange(
                startOffset = composingStart,
                endOffset = composingEnd,
            )
        } else {
            null
        }

        return RuntimeHostTextInputState(
            text = editable.toString(),
            selection = RuntimeHostTextInputRange(
                startOffset = hiddenEditor.selectionStart.coerceAtLeast(0),
                endOffset = hiddenEditor.selectionEnd.coerceAtLeast(0),
            ),
            composing = composing,
        )
    }

    /**
     * Map one runtime text-input configuration into one Android input-type bitset.
     */
    private fun inputType(
        configuration: RuntimeHostTextInputConfiguration,
    ): Int {
        var inputType = when (configuration.inputType) {
            RuntimeHostTextInputType.TEXT -> InputType.TYPE_CLASS_TEXT
            RuntimeHostTextInputType.NUMBER -> InputType.TYPE_CLASS_NUMBER
            RuntimeHostTextInputType.EMAIL ->
                InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
            RuntimeHostTextInputType.URL ->
                InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            RuntimeHostTextInputType.PASSWORD ->
                InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            RuntimeHostTextInputType.PHONE -> InputType.TYPE_CLASS_PHONE
            RuntimeHostTextInputType.SEARCH ->
                InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_NORMAL
        }

        // widen the input flags for secure and multiline sessions
        if (configuration.isSecure) {
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
        }

        if (configuration.isMultiline) {
            inputType = inputType or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        }

        return inputType
    }

    /**
     * Resolve the input-method manager for one activity.
     */
    private fun inputMethodManager(
        activity: ComponentActivity,
    ): InputMethodManager {
        return requireNotNull(
            activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager,
        ) {
            "input method manager is not available"
        }
    }
}
