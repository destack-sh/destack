package dev.destack.runtime.android.module.document

/**
 * The document request surface attached to one Android runtime host.
 */
public fun interface DocumentRequests {
    /**
     * Submit one document request to the Android host.
     */
    public fun submitDocumentRequest(
        request: RuntimeHostDocumentRequest,
    )
}
