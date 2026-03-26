package dev.destack.runtime.android.input.text

/**
 * The text-input ingress surface attached to one Android runtime host.
 */
public fun interface TextInputEvents {
    /**
     * Send one host text-input event into the attached runtime session.
     */
    public fun sendTextInputEvent(
        event: RuntimeHostTextInputEvent,
    )
}
