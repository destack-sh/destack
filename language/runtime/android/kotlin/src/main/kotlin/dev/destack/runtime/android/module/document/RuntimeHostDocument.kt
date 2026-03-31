package dev.destack.runtime.android.module.document

import dev.destack.runtime.android.core.HostRequestId

/**
 * One document descriptor returned by the Android host.
 */
public data class RuntimeHostDocumentDescriptor(
    /**
     * The stable URI or content identifier returned by the host.
     */
    val uri: String,

    /**
     * The normalized display name when available.
     */
    val displayName: String? = null,

    /**
     * The normalized content type when available.
     */
    val contentType: String? = null,

    /**
     * The optional host-local path when the host exposes one directly.
     */
    val localPath: String? = null,
)

/**
 * One Android document request submitted by one runtime session.
 */
public data class RuntimeHostDocumentRequest(
    /**
     * The stable request identifier for this interactive host flow.
     */
    val requestId: HostRequestId,

    /**
     * The accepted content types for the picker.
     */
    val contentTypes: List<String> = emptyList(),

    /**
     * The accepted MIME types for the picker.
     */
    val mimeTypes: List<String> = contentTypes,

    /**
     * The accepted file extensions for the picker.
     */
    val extensions: List<String> = emptyList(),

    /**
     * Whether multiple documents may be selected.
     */
    val allowsMultipleSelection: Boolean = false,

    /**
     * Whether directory selection is allowed.
     */
    val allowsDirectorySelection: Boolean = false,

    /**
     * Whether selected files should be copied into one runtime sandbox when possible.
     */
    val copiesToSandbox: Boolean = false,
) {
    /**
     * The normalized content filters for the picker.
     */
    public val normalizedContentTypes: List<String>
        get() {
            if (mimeTypes.isNotEmpty()) {
                return mimeTypes
            }

            return contentTypes
        }
}

/**
 * One Android document result returned to one runtime session.
 */
public data class RuntimeHostDocumentResult(
    /**
     * The stable request identifier for this interactive host flow.
     */
    val requestId: HostRequestId,

    /**
     * The selected document descriptors.
     */
    val documents: List<RuntimeHostDocumentDescriptor>,
)
