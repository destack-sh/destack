package dev.destack.runtime.android.bridge.document

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor

/**
 * The low-level document lane ABI for one Android runtime bridge.
 */
public interface DocumentAbi {
    /**
     * Deliver one document result into one runtime session.
     */
    public fun notifyDocumentResult(
        sessionHandle: HostSessionHandle,
        requestId: HostRequestId,
        documents: List<RuntimeHostDocumentDescriptor>,
    ): RuntimeAbiStatus
}
