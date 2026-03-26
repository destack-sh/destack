package dev.destack.runtime.android.input.text

import dev.destack.runtime.android.core.hostStatusNotSupported

/**
 * The text-input request surface attached to one Android runtime host.
 */
public interface TextInputRequests {
    /**
     * Open one Android text-input session.
     */
    public fun openTextInput(
        request: RuntimeHostTextInputOpenRequest,
    ): Int

    /**
     * Close one Android text-input session.
     */
    public fun closeTextInput(
        request: RuntimeHostTextInputCloseRequest,
    ): Int

    /**
     * Update one Android text-input geometry hint.
     */
    public fun setTextInputGeometry(
        request: RuntimeHostTextInputGeometryRequest,
    ): Int

    /**
     * Update one Android text-input state.
     */
    public fun setTextInputState(
        request: RuntimeHostTextInputStateRequest,
    ): Int
}

/**
 * The explicit unsupported text-input request surface for one Android runtime host.
 */
public object UnsupportedTextInputRequests : TextInputRequests {
    override fun openTextInput(
        request: RuntimeHostTextInputOpenRequest,
    ): Int {
        return hostStatusNotSupported
    }

    override fun closeTextInput(
        request: RuntimeHostTextInputCloseRequest,
    ): Int {
        return hostStatusNotSupported
    }

    override fun setTextInputGeometry(
        request: RuntimeHostTextInputGeometryRequest,
    ): Int {
        return hostStatusNotSupported
    }

    override fun setTextInputState(
        request: RuntimeHostTextInputStateRequest,
    ): Int {
        return hostStatusNotSupported
    }
}
