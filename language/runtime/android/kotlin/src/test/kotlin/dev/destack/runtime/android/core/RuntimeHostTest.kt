package dev.destack.runtime.android

import dev.destack.runtime.android.core.HostEmbedderId
import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RendererSurface
import dev.destack.runtime.android.core.RendererSurfaceKind
import dev.destack.runtime.android.module.document.RuntimeHostDocumentRequest
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleEvent
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleSourceKind
import dev.destack.runtime.android.module.lifecycle.RuntimeHostLifecycleState
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
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionRequest

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Create one host with one primary renderer surface.
 */
class RuntimeHostTest {
    @Test
    fun testCreateHostWithPrimarySurface() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val embedderId = HostEmbedderId(rawValue = 11)
        val lifecycleEvents = RecordingLifecycleSink()
        val permissionRequests = RecordingPermissionRequestHandler()
        val permissionEvents = RecordingPermissionEventSink()
        val documentRequests = RecordingDocumentRequestHandler()
        val documentEvents = RecordingDocumentEventSink()
        val rendererSurface = RendererSurface(
            kind = RendererSurfaceKind.SurfaceView,
            identifier = "main-surface",
        )

        val host = createRuntimeHost(
            lifecycleEvents = lifecycleEvents,
            permissionRequests = permissionRequests,
            permissionEvents = permissionEvents,
            documentRequests = documentRequests,
            documentEvents = documentEvents,
            surfaceKind = rendererSurface.kind,
            rendererSurfaceIdentifier = rendererSurface.identifier,
        )

        assertEquals(sessionHandle, host.sessionHandle)
        assertEquals(embedderId, host.embedderId)
        assertEquals(rendererSurface, host.rendererSurface)
    }

    /**
     * Send one lifecycle event through the attached lifecycle sink.
     */
    @Test
    fun testSendLifecycleEvent() {
        val lifecycleEvents = RecordingLifecycleSink()
        val host = createRuntimeHost(
            lifecycleEvents = lifecycleEvents,
        )
        val event = RuntimeHostLifecycleEvent(
            sourceKind = RuntimeHostLifecycleSourceKind.Activity,
            state = RuntimeHostLifecycleState.Running,
        )

        host.lifecycle.sendLifecycleEvent(event)

        assertEquals(listOf(event), lifecycleEvents.events)
    }

    /**
     * Submit one permission request through the attached request handler.
     */
    @Test
    fun testSubmitPermissionRequest() {
        val permissionRequests = RecordingPermissionRequestHandler()
        val host = createRuntimeHost(
            permissionRequests = permissionRequests,
        )
        val request = RuntimeHostPermissionRequest(
            requestId = HostRequestId(rawValue = 3),
            permission = "location",
        )

        host.permission.request(request)

        assertEquals(listOf(request), permissionRequests.requests)
    }

    /**
     * Open the permission settings surface through the attached request handler.
     */
    @Test
    fun testOpenPermissionSettings() {
        val permissionRequests = RecordingPermissionRequestHandler()
        val host = createRuntimeHost(
            permissionRequests = permissionRequests,
        )

        val status = host.permission.openSettings()

        assertEquals(0, status)
        assertEquals(1, permissionRequests.openSettingsCalls)
    }

    /**
     * Send one permission event through the attached permission ingress sink.
     */
    @Test
    fun testSendPermissionEvent() {
        val permissionEvents = RecordingPermissionEventSink()
        val host = createRuntimeHost(
            permissionEvents = permissionEvents,
        )
        val event = RuntimeHostPermissionEvent(
            requestId = HostRequestId(rawValue = 4),
            permission = "location",
            isGranted = true,
        )

        host.permission.notifyPermissionResult(event)

        assertEquals(listOf(event), permissionEvents.events)
    }

    /**
     * Submit one document request through the attached request handler.
     */
    @Test
    fun testSubmitDocumentRequest() {
        val documentRequests = RecordingDocumentRequestHandler()
        val documentEvents = RecordingDocumentEventSink()
        val host = createRuntimeHost(
            documentRequests = documentRequests,
            documentEvents = documentEvents,
        )
        val request = RuntimeHostDocumentRequest(
            requestId = HostRequestId(rawValue = 5),
            allowsMultipleSelection = true,
            contentTypes = listOf("image/png"),
        )

        host.document.pick(request)
        val result = sampleDocumentResult(request.requestId)
        host.document.notifyDocumentResult(
            result.requestId,
            result.documents,
        )

        assertEquals(listOf(request), documentRequests.requests)
        assertEquals("example.png", documentEvents.results.first().documents.first().displayName)
    }

    /**
     * Route media requests through the attached media request handler.
     */
    @Test
    fun testRouteMediaRequests() {
        val mediaRequests = RecordingMediaRequestHandler().also {
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
                deletedCount = 3,
            )
        }
        val host = createRuntimeHost(
            mediaRequests = mediaRequests,
        )

        val listRequest = RuntimeHostMediaListRequest(
            cursor = "1",
            limit = 10,
            kinds = listOf(RuntimeHostMediaAssetKind.Image),
            includeHidden = true,
        )
        val importRequest = RuntimeHostMediaImportPathRequest(
            path = "/tmp/example.png",
            kind = RuntimeHostMediaAssetKind.Image,
        )
        val deleteRequest = RuntimeHostMediaDeleteRequest(
            identifiers = listOf("asset-1", "asset-2", "asset-3"),
        )

        val listResponse = host.media.list(listRequest)
        val readResponse = host.media.read("asset-1")
        val importResponse = host.media.importPath(importRequest)
        val deleteResponse = host.media.delete(deleteRequest)

        assertEquals(listOf(listRequest), mediaRequests.listRequests)
        assertEquals(listOf("asset-1"), mediaRequests.readIdentifiers)
        assertEquals(listOf(importRequest), mediaRequests.importRequests)
        assertEquals(listOf(deleteRequest), mediaRequests.deleteRequests)
        assertEquals("example.png", listResponse.page?.assets?.first()?.filename)
        assertEquals("asset-1", readResponse.descriptor?.identifier)
        assertEquals("asset-2", importResponse.identifier)
        assertEquals(3, deleteResponse.deletedCount)
    }

    /**
     * Report unsupported media operations through the explicit unsupported media surface.
     */
    @Test
    fun testReportUnsupportedMediaWithoutRequests() {
        val host = createRuntimeHost()

        val listResponse = host.media.list(RuntimeHostMediaListRequest())
        val readResponse = host.media.read("asset-1")
        val importResponse = host.media.importPath(
            RuntimeHostMediaImportPathRequest(
                path = "/tmp/example.png",
                kind = RuntimeHostMediaAssetKind.Image,
            ),
        )
        val deleteResponse = host.media.delete(
            RuntimeHostMediaDeleteRequest(
                identifiers = listOf("asset-1"),
            ),
        )

        assertEquals(1, listResponse.status)
        assertEquals(1, readResponse.status)
        assertEquals(1, importResponse.status)
        assertEquals(1, deleteResponse.status)
    }
}
