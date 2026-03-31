package dev.destack.runtime.android.module.document

import android.net.Uri

import androidx.activity.result.ActivityResultCallback
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts

import dev.destack.runtime.android.embedder.ActivityResults
import dev.destack.runtime.android.embedder.ActivityResultLauncherKey
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.core.hostStatusOk

/**
 * The document activity-result bridge for one Android runtime host.
 */
internal class DocumentActivityResults(
    private val runtimeHost: RuntimeHost,
    activityResults: ActivityResults,
) : DocumentRequests {
    private val singleDocumentLauncher: ActivityResultLauncher<Array<String>> =
        activityResults.registerLauncher(
            launcherKey = ActivityResultLauncherKey.DocumentPickSingle,
            contract = ActivityResultContracts.OpenDocument(),
            callback = ActivityResultCallback(::completeSingleDocumentPick),
        )

    private val multipleDocumentsLauncher: ActivityResultLauncher<Array<String>> =
        activityResults.registerLauncher(
            launcherKey = ActivityResultLauncherKey.DocumentPickMultiple,
            contract = ActivityResultContracts.OpenMultipleDocuments(),
            callback = ActivityResultCallback(::completeMultipleDocumentPick),
        )

    /**
     * Submit one document request through the Android activity-result registry.
     */
    override fun pick(
        request: RuntimeHostDocumentRequest,
    ): Int {
        runtimeHost.beginDocumentRequest(request.requestId)
        val requestedContentTypes = request.contentTypes.toTypedArray()
        val contentTypes = if (requestedContentTypes.isEmpty()) arrayOf("*/*") else requestedContentTypes

        // one launcher per selection mode keeps the registration order stable
        if (request.allowsMultipleSelection) {
            multipleDocumentsLauncher.launch(contentTypes)
        }
        else {
            singleDocumentLauncher.launch(contentTypes)
        }

        return hostStatusOk
    }

    /**
     * Complete one single-documenter flow.
     */
    private fun completeSingleDocumentPick(
        uri: Uri?,
    ) {
        val requestId = runtimeHost.finishDocumentRequest()
        val result = if (uri == null) {
            RuntimeHostDocumentResult(
                requestId = requestId,
                documents = emptyList(),
            )
        }
        else {
            RuntimeHostDocumentResult(
                requestId = requestId,
                documents = listOf(documentDescriptorForUri(uri)),
            )
        }

        completeDocumentPick(result)
    }

    /**
     * Complete one multi-documenter flow.
     */
    private fun completeMultipleDocumentPick(
        uris: List<Uri>,
    ) {
        val requestId = runtimeHost.finishDocumentRequest()
        val result = RuntimeHostDocumentResult(
            requestId = requestId,
            documents = uris.map(::documentDescriptorForUri),
        )

        completeDocumentPick(result)
    }

    /**
     * Finish one in-flight document request.
     */
    private fun completeDocumentPick(
        result: RuntimeHostDocumentResult,
    ) {
        runtimeHost.document.notifyDocumentResult(
            result.requestId,
            result.documents,
        )
    }

    /**
     * Build one normalized document descriptor from one Android content URI.
     */
    private fun documentDescriptorForUri(
        uri: Uri,
    ): RuntimeHostDocumentDescriptor {
        return RuntimeHostDocumentDescriptor(uri = uri.toString())
    }
}
