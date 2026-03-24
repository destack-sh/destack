package dev.destack.runtime.android

import dev.destack.runtime.android.module.media.RuntimeHostMediaAssetDescriptor
import dev.destack.runtime.android.module.media.RuntimeHostMediaAssetKind
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaListRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaListResult

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create media list, import, and delete requests.
 */
class RuntimeHostMediaTest {
    @Test
    fun testCreateMediaRequestsAndResult() {
        val listRequest = RuntimeHostMediaListRequest(
            cursor = "1",
            limit = 10,
            kinds = listOf(RuntimeHostMediaAssetKind.Image),
            includeHidden = true,
        )
        val listResult = RuntimeHostMediaListResult(
            assets = listOf(
                RuntimeHostMediaAssetDescriptor(
                    identifier = "asset-1",
                    uri = "content://media/external/file/1",
                    filename = "example.png",
                    mimeType = "image/png",
                    kind = RuntimeHostMediaAssetKind.Image,
                ),
            ),
            nextCursor = "2",
            hasMore = true,
        )
        val importRequest = RuntimeHostMediaImportPathRequest(
            path = "/tmp/example.png",
            kind = RuntimeHostMediaAssetKind.Image,
        )
        val deleteRequest = RuntimeHostMediaDeleteRequest(identifiers = listOf("asset-1"))

        assertEquals(listOf(RuntimeHostMediaAssetKind.Image), listRequest.kinds)
        assertEquals(1, listResult.assets.size)
        assertEquals("asset-1", listResult.assets.first().identifier)
        assertEquals("/tmp/example.png", importRequest.path)
        assertEquals(listOf("asset-1"), deleteRequest.identifiers)
    }
}
