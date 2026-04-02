package dev.destack.runtime.android

import android.net.Uri

import androidx.lifecycle.Lifecycle

import dev.destack.runtime.android.embedder.ActivityResults
import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.module.document.ActivityDocumentRequests
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Register one stable launcher set per runtime session and complete one document request.
 */
class ActivityDocumentRequestsTest {
    @Test
    fun testActivityDocumentRequestsRegisterStableLaunchersAndCompleteRequests() {
        val registry = RecordingActivityResultRegistry()
        val owner = TestLifecycleOwner()
        val documentEvents = RecordingDocumentEventSink()
        val runtimeHost = createRuntimeHost(
            documentEvents = documentEvents.callback,
        )
        val activityResults = ActivityResults(
            embedderId = HostEmbedderId(rawValue = 11),
            registry = registry,
            lifecycleOwner = owner,
        )
        val documentRequests = ActivityDocumentRequests(
            runtimeHost = runtimeHost,
            activityResults = activityResults,
        )
        val request = RuntimeHostDocumentRequest(
            requestId = HostRequestId(rawValue = 1),
            multiple = true,
            mimeTypes = listOf("image/png"),
        )

        owner.handleEvent(Lifecycle.Event.ON_CREATE)
        owner.handleEvent(Lifecycle.Event.ON_START)
        documentRequests.pick(request)
        val requestCode = registry.launchedRequestCodes.last()

        assertEquals(
            listOf(
                "androidx.activity.result.contract.ActivityResultContracts\$OpenMultipleDocuments",
            ),
            registry.launchedContracts,
        )
        assertEquals(
            listOf("image/png"),
            (registry.launchedInputs.getValue(requestCode) as Array<*>).map { it as String },
        )

        registry.completeLastLaunch(
            result = emptyList<Uri>(),
        )

        assertEquals(
            listOf(
                RuntimeHostDocumentResult(
                    requestId = HostRequestId(rawValue = 1),
                    documents = emptyList<RuntimeHostDocumentDescriptor>(),
                ),
            ),
            documentEvents.results,
        )
    }
}
