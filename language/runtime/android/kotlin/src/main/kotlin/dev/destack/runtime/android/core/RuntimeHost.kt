package dev.destack.runtime.android.core

import dev.destack.runtime.android.module.text.TextRequests
import dev.destack.runtime.android.module.text.UnsupportedTextRequests
import dev.destack.runtime.android.module.background.BackgroundEvents
import dev.destack.runtime.android.module.background.BackgroundRequests
import dev.destack.runtime.android.module.background.UnsupportedBackgroundRequests
import dev.destack.runtime.android.module.permission.RuntimeHostPermission

/**
 * The Android runtime host for one attached Destack runtime session.
 */
public class RuntimeHost(
    /**
     * The attached host session handle.
     */
    public val sessionHandle: HostSessionHandle,

    /**
     * The stable embedder identifier for this embedder.
     */
    public val embedderId: HostEmbedderId,

    /**
     * The lifecycle host surface for this embedder.
     */
    public val lifecycle: LifecycleHost,

    /**
     * The permission host surface for this embedder.
     */
    public val permission: PermissionHost,

    /**
     * The document host surface for this embedder.
     */
    public val document: DocumentHost,

    /**
     * The text host surface for this embedder.
     */
    public val text: TextHost = TextHost(
        UnsupportedTextRequests,
        dev.destack.runtime.android.module.text.NoopTextEvents,
    ),

    /**
     * The background host surface for this embedder.
     */
    public val background: BackgroundHost = BackgroundHost(
        UnsupportedBackgroundRequests,
        BackgroundEvents { },
    ),

    /**
     * The contact host surface for this embedder.
     */
    public val contact: ContactHost,

    /**
     * The calendar host surface for this embedder.
     */
    public val calendar: CalendarHost,

    /**
     * The intent host surface for this embedder.
     */
    public val intent: IntentHost,

    /**
     * The location host surface for this embedder.
     */
    public val location: LocationHost,

    /**
     * The media host surface for this embedder.
     */
    public val media: MediaHost,

    /**
     * The notification host surface for this embedder.
     */
    public val notification: NotificationHost,

    /**
     * The primary renderer surface for this embedder.
     */
    rendererSurface: RendererSurface,
) {
    private data class PendingPermissionRequest(
        val requestId: HostRequestId,
        val permission: RuntimeHostPermission,
    )

    private var pendingDocumentRequestId: HostRequestId? = null
    private var pendingPermissionRequest: PendingPermissionRequest? = null

    /**
     * The current primary renderer surface for this embedder.
     */
    public var rendererSurface: RendererSurface = rendererSurface
        private set

    /**
     * Replace the primary renderer surface for this host embedder.
     */
    internal fun updateRendererSurface(
        rendererSurface: RendererSurface,
    ) {
        this.rendererSurface = rendererSurface
    }

    /**
     * Replace the text request surface for this host.
     */
    internal fun updateTextRequests(
        requests: TextRequests,
    ) {
        text.updateRequests(requests)
    }

    /**
     * Replace the background request surface for this host.
     */
    internal fun updateBackgroundRequests(
        requests: BackgroundRequests,
    ) {
        background.updateRequests(requests)
    }

    /**
     * Start tracking one in-flight document request.
     */
    internal fun beginDocumentRequest(
        requestId: HostRequestId,
    ) {
        // reject overlapping document flows
        require(pendingDocumentRequestId == null) {
            "document request already in flight"
        }

        pendingDocumentRequestId = requestId
    }

    /**
     * Return the in-flight document request identifier when present.
     */
    internal fun pendingDocumentRequestId(
    ): HostRequestId? {
        return pendingDocumentRequestId
    }

    /**
     * Restore the in-flight document request when one exists.
     */
    internal fun restoreDocumentRequest(
        requestId: HostRequestId?,
    ) {
        pendingDocumentRequestId = requestId
    }

    /**
     * Finish the in-flight document request.
     */
    internal fun finishDocumentRequest(): HostRequestId {
        val requestId = requireNotNull(pendingDocumentRequestId) {
            "document result arrived without one in-flight request"
        }

        pendingDocumentRequestId = null

        return requestId
    }

    /**
     * Start tracking one in-flight permission request.
     */
    internal fun beginPermissionRequest(
        requestId: HostRequestId,
        permission: RuntimeHostPermission,
    ) {
        // reject overlapping permission flows
        require(pendingPermissionRequest == null) {
            "permission request already in flight"
        }

        pendingPermissionRequest = PendingPermissionRequest(
            requestId = requestId,
            permission = permission,
        )
    }

    /**
     * Return the in-flight permission request identifier when present.
     */
    internal fun pendingPermissionRequestId(): HostRequestId? {
        return pendingPermissionRequest?.requestId
    }

    /**
     * Return the in-flight permission payload when present.
     */
    internal fun pendingPermission(): RuntimeHostPermission? {
        return pendingPermissionRequest?.permission
    }

    /**
     * Restore the in-flight permission request when one exists.
     */
    internal fun restorePermissionRequest(
        requestId: HostRequestId?,
        permission: RuntimeHostPermission?,
    ) {
        // restore a complete permission flow or nothing
        require((requestId == null) == (permission == null)) {
            "permission request restoration requires both request id and permission"
        }

        pendingPermissionRequest = if (requestId == null || permission == null) {
            null
        } else {
            PendingPermissionRequest(
                requestId = requestId,
                permission = permission,
            )
        }
    }

    /**
     * Finish the in-flight permission request.
     */
    internal fun finishPermissionRequest(): Pair<HostRequestId, RuntimeHostPermission> {
        val state = requireNotNull(pendingPermissionRequest) {
            "permission result arrived without one in-flight request"
        }

        pendingPermissionRequest = null

        return state.requestId to state.permission
    }
}
