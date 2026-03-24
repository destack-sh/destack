package dev.destack.runtime.android.bridge.document

import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor

/**
 * One bridged document descriptor delivered into the runtime ingress path.
 */
internal data class DocumentDescriptorAbi(
    /**
     * The stable URI or content identifier returned by the host.
     */
    val uri: String,

    /**
     * The normalized document name.
     */
    val name: String,

    /**
     * The normalized content type when available.
     */
    val mimeType: String?,

    /**
     * The document size in bytes when available.
     */
    val sizeBytes: Long?,

    /**
     * The document modification timestamp in UTC nanoseconds when available.
     */
    val modifiedUnixNs: Long?,

    /**
     * Whether this descriptor represents one directory.
     */
    val isDirectory: Boolean,

    /**
     * The optional host-local path when the host exposes one directly.
     */
    val localPath: String?,
) {
    companion object {
        /**
         * Build one bridged descriptor from one runtime host descriptor.
         */
        fun fromDocument(
            document: RuntimeHostDocumentDescriptor,
        ): DocumentDescriptorAbi {
            return DocumentDescriptorAbi(
                uri = document.uri,
                name = document.displayName ?: "",
                mimeType = document.contentType,
                sizeBytes = null,
                modifiedUnixNs = null,
                isDirectory = false,
                localPath = document.localPath,
            )
        }
    }
}
