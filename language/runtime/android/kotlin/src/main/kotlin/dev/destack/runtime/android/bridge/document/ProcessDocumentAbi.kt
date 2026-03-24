package dev.destack.runtime.android.bridge.document

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.bridge.RuntimeHostLibraryLoader
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor

/**
 * The default document lane ABI resolved through the Android JNI bridge.
 */
internal object ProcessDocumentAbi : DocumentAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun notifyDocumentResult(
        sessionHandle: HostSessionHandle,
        requestId: HostRequestId,
        documents: List<RuntimeHostDocumentDescriptor>,
    ): RuntimeAbiStatus {
        val descriptors = documents
            .map(DocumentDescriptorAbi::fromDocument)
            .toTypedArray()
        val values = nativeNotifyDocumentResult(
            sessionHandle.rawValue,
            requestId.rawValue,
            descriptors,
        )

        return RuntimeAbiStatus(
            code = values[0].toInt(),
            errorId = values[1],
        )
    }

    private external fun nativeNotifyDocumentResult(
        sessionHandle: Long,
        requestId: Long,
        documents: Array<DocumentDescriptorAbi>,
    ): LongArray
}
