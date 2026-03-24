package dev.destack.runtime.android.bridge.document

import dev.destack.runtime.android.bridge.RuntimeAbi
import dev.destack.runtime.android.bridge.core.MainThreadBridge
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.document.DocumentEvents
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult

/**
 * One document bridge lane for one attached Android runtime host.
 */
internal class DocumentBridge(
    private val sessionHandle: HostSessionHandle,
    private val bindings: RuntimeAbi,
    private val mainThreadBridge: MainThreadBridge,
) : DocumentEvents {
    /**
     * Send one document result into the runtime ingress path.
     */
    override fun sendDocumentResult(
        result: RuntimeHostDocumentResult,
    ) {
        val status = bindings.notifyDocumentResult(
            sessionHandle = sessionHandle,
            requestId = result.requestId,
            documents = result.documents,
        )

        require(status.code == hostStatusOk) {
            "runtime bridge could not deliver document result: code ${status.code}, error ${status.errorId}"
        }
    }

    /**
     * Submit one document request decoded from one runtime callback payload.
     */
    fun submitRequest(
        runtimeHost: RuntimeHost,
        requestId: Long,
        mimeTypes: Array<String>,
        extensions: Array<String>,
        allowsMultipleSelection: Boolean,
        allowsDirectorySelection: Boolean,
        copiesToSandbox: Boolean,
    ): Int {
        val request = RuntimeHostDocumentRequest(
            requestId = dev.destack.runtime.android.core.HostRequestId(rawValue = requestId),
            allowsMultipleSelection = allowsMultipleSelection,
            contentTypes = buildList {
                addAll(mimeTypes)

                // preserve extension-only filters as synthetic content types for now
                for (extension in extensions) {
                    if (extension.isNotBlank()) {
                        add("application/x.destack-extension.$extension")
                    }
                }
            },
        )

        mainThreadBridge.run {
            runtimeHost.documentRequests.submitDocumentRequest(request)
        }

        return hostStatusOk
    }
}
