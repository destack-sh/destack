package dev.destack.runtime.android

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.module.document.DocumentEvents
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult

/**
 * One recording document request handler for Android host tests.
 */
internal class RecordingDocumentRequestHandler : DocumentRequests {
    val requests: MutableList<RuntimeHostDocumentRequest> = mutableListOf()

    override fun submitDocumentRequest(
        request: RuntimeHostDocumentRequest,
    ) {
        requests += request
    }
}

/**
 * One recording document result sink for Android host tests.
 */
internal class RecordingDocumentEventSink : DocumentEvents {
    val results: MutableList<RuntimeHostDocumentResult> = mutableListOf()

    override fun sendDocumentResult(
        result: RuntimeHostDocumentResult,
    ) {
        results += result
    }
}

/**
 * One no-op document request handler for Android host tests.
 */
internal class NoopDocumentRequestHandler : DocumentRequests {
    override fun submitDocumentRequest(
        request: RuntimeHostDocumentRequest,
    ) {}
}

/**
 * Build one sample document result fixture.
 */
internal fun sampleDocumentResult(
    requestId: HostRequestId,
): RuntimeHostDocumentResult {
    return RuntimeHostDocumentResult(
        requestId = requestId,
        documents = listOf(
            RuntimeHostDocumentDescriptor(
                uri = "file:///tmp/example.png",
                displayName = "example.png",
                contentType = "image/png",
                localPath = "/tmp/example.png",
            ),
        ),
    )
}
