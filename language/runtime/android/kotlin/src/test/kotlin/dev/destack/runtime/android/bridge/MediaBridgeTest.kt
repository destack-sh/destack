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
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
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

        val listRequest = RuntimeHostMediaListRequest(
            cursor = "1",
            limit = 10,
            kinds = listOf(
                RuntimeHostMediaAssetKind.Image,
                RuntimeHostMediaAssetKind.Video,
            ),
            includeHidden = true,
        )

        val listResponse = bridge.mediaList(
            runtimeHost = runtimeHost,
            request = listRequest,
        )
        val readResponse = bridge.mediaRead(runtimeHost, "asset-1")
        val importResponse = bridge.mediaImportPath(
            runtimeHost = runtimeHost,
            request = RuntimeHostMediaImportPathRequest(
                path = "/tmp/example.png",
                kind = RuntimeHostMediaAssetKind.Image,
            ),
        )
        val deleteResponse = bridge.mediaDelete(
            runtimeHost = runtimeHost,
            request = RuntimeHostMediaDeleteRequest(
                identifiers = listOf("asset-1", "asset-2"),
            ),
        )

        assertEquals(
            listOf(
                listRequest,
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
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val mediaRequests = MediaRequestRecorder()
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            intentRequests = IntentRequestRecorder(),
            mediaRequests = mediaRequests,
        )

        bridge.attach(runtimeHost)

        val listResponse = bridge.mediaList(
            hasCursor = false,
            cursor = "",
            hasLimit = false,
            limit = 0,
            kinds = emptyArray<RuntimeHostMediaAssetKind>(),
            includeHidden = false,
        )
        val importResponse = bridge.mediaImportPath("/tmp/example.png", 9)

        assertEquals(0, listResponse.status)
        assertEquals(2, importResponse.status)
        assertEquals(
            listOf(
                RuntimeHostMediaListRequest(
                    cursor = null,
                    limit = null,
                    kinds = emptyList(),
                    includeHidden = false,
                ),
            ),
            mediaRequests.listRequests,
        )
        assertEquals(
            emptyList<RuntimeHostMediaImportPathRequest>(),
            mediaRequests.importRequests,
        )
    }
}
