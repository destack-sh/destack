package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.media.RuntimeHostMediaAssetDescriptor
import dev.destack.runtime.android.module.media.RuntimeHostMediaAssetKind
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaDeleteResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaImportPathResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaListRequest
import dev.destack.runtime.android.module.media.RuntimeHostMediaListResponse
import dev.destack.runtime.android.module.media.RuntimeHostMediaListResult
import dev.destack.runtime.android.module.media.RuntimeHostMediaReadResponse

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Exercise the media bridge lane.
 */
class MediaBridgeTest {
    /**
     * Route media requests into the attached runtime host.
     */
    @Test
    fun testMediaRequestsRouteIntoRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val mediaRequests = MediaRequestRecorder().also {
            it.listResponse = RuntimeHostMediaListResponse(
                status = 0,
                page = RuntimeHostMediaListResult(
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
                ),
            )
            it.readResponse = RuntimeHostMediaReadResponse(
                status = 0,
                descriptor = RuntimeHostMediaAssetDescriptor(
                    identifier = "asset-1",
                    uri = "content://media/external/file/1",
                    filename = "example.png",
                    mimeType = "image/png",
                    kind = RuntimeHostMediaAssetKind.Image,
                ),
            )
            it.importResponse = RuntimeHostMediaImportPathResponse(
                status = 0,
                identifier = "asset-2",
            )
            it.deleteResponse = RuntimeHostMediaDeleteResponse(
                status = 0,
                deletedCount = 2,
            )
        }
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
            mediaRequests = mediaRequests,
        )

        bridge.attach(runtimeHost)

        val listResponse = bridge.listMedia(
            cursor = "1",
            hasLimit = true,
            limit = 10,
            kinds = intArrayOf(1, 2),
            includeHidden = true,
        )
        val readResponse = bridge.readMedia("asset-1")
        val importResponse = bridge.importMediaPath("/tmp/example.png", 1)
        val deleteResponse = bridge.deleteMedia(arrayOf("asset-1", "asset-2"))

        assertEquals(
            listOf(
                RuntimeHostMediaListRequest(
                    cursor = "1",
                    limit = 10,
                    kinds = listOf(
                        RuntimeHostMediaAssetKind.Image,
                        RuntimeHostMediaAssetKind.Video,
                    ),
                    includeHidden = true,
                ),
            ),
            mediaRequests.listRequests,
        )
        assertEquals(listOf("asset-1"), mediaRequests.readIdentifiers)
        assertEquals(
            listOf(
                RuntimeHostMediaImportPathRequest(
                    path = "/tmp/example.png",
                    kind = RuntimeHostMediaAssetKind.Image,
                ),
            ),
            mediaRequests.importRequests,
        )
        assertEquals(
            listOf(
                RuntimeHostMediaDeleteRequest(
                    identifiers = listOf("asset-1", "asset-2"),
                ),
            ),
            mediaRequests.deleteRequests,
        )
        assertEquals("example.png", listResponse.page?.assets?.first()?.filename)
        assertEquals("asset-1", readResponse.descriptor?.identifier)
        assertEquals("asset-2", importResponse.identifier)
        assertEquals(2, deleteResponse.deletedCount)
    }

    /**
     * Reject invalid media-kind payloads before touching the runtime host.
     */
    @Test
    fun testMediaBridgeRejectsInvalidKinds() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val bindings = RuntimeAbiSpy()
        val bridge = RuntimeBridge(sessionHandle, bindings)
        val mediaRequests = MediaRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
            mediaRequests = mediaRequests,
        )

        bridge.attach(runtimeHost)

        val listResponse = bridge.listMedia(
            cursor = null,
            hasLimit = false,
            limit = 0,
            kinds = intArrayOf(9),
            includeHidden = false,
        )
        val importResponse = bridge.importMediaPath("/tmp/example.png", 9)

        assertEquals(2, listResponse.status)
        assertEquals(2, importResponse.status)
        assertEquals(emptyList<RuntimeHostMediaListRequest>(), mediaRequests.listRequests)
        assertEquals(
            emptyList<RuntimeHostMediaImportPathRequest>(),
            mediaRequests.importRequests,
        )
    }
}
