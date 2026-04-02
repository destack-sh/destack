package dev.destack.runtime.android

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create one document request and one document result.
 */
class RuntimeHostDocumentTest {
    @Test
    fun testCreateDocumentRequestAndResult() {
        val request = RuntimeHostDocumentRequest(
            requestId = HostRequestId(rawValue = 7),
            multiple = true,
            contentTypes = listOf("image/png", "image/jpeg"),
        )
        val result = sampleDocumentResult(request.requestId)

        assertEquals(HostRequestId(rawValue = 7), request.requestId)
        assertEquals(true, request.allowsMultipleSelection)
        assertEquals(listOf("image/png", "image/jpeg"), request.contentTypes)
        assertEquals(HostRequestId(rawValue = 7), result.requestId)
        assertEquals(1, result.documents.size)
        assertEquals("example.png", result.documents.first().displayName)
    }
}
