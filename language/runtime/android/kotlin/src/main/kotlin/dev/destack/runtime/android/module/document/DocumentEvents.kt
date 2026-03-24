package dev.destack.runtime.android.module.document

/**
 * The document event surface attached to one Android runtime host.
 */
public fun interface DocumentEvents {
    /**
     * Send one normalized document result into the attached runtime session.
     */
    public fun sendDocumentResult(
        result: RuntimeHostDocumentResult,
    )
}
