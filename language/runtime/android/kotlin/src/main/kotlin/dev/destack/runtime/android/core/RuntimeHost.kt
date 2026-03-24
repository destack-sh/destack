package dev.destack.runtime.android.core

import dev.destack.runtime.android.module.calendar.CalendarRequests
import dev.destack.runtime.android.module.document.DocumentEvents
import dev.destack.runtime.android.module.document.DocumentRequests
import dev.destack.runtime.android.module.contact.ContactRequests
import dev.destack.runtime.android.module.intent.IntentEvents
import dev.destack.runtime.android.module.intent.IntentRequests
import dev.destack.runtime.android.module.lifecycle.LifecycleEvents
import dev.destack.runtime.android.module.location.LocationEvents
import dev.destack.runtime.android.module.location.LocationRequests
import dev.destack.runtime.android.module.media.MediaRequests
import dev.destack.runtime.android.module.notification.NotificationEvents
import dev.destack.runtime.android.module.notification.NotificationRequests
import dev.destack.runtime.android.module.permission.PermissionEvents
import dev.destack.runtime.android.module.permission.PermissionRequests

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
     * The lifecycle ingress surface for this embedder.
     */
    public val lifecycleEvents: LifecycleEvents,

    /**
     * The permission request surface for this embedder.
     */
    public val permissionRequests: PermissionRequests,

    /**
     * The permission ingress surface for this embedder.
     */
    public val permissionEvents: PermissionEvents,

    /**
     * The document request surface for this embedder.
     */
    public val documentRequests: DocumentRequests,

    /**
     * The document result surface for this embedder.
     */
    public val documentEvents: DocumentEvents,

    /**
     * The contact request surface for this embedder.
     */
    public val contactRequests: ContactRequests,

    /**
     * The calendar request surface for this embedder.
     */
    public val calendarRequests: CalendarRequests,

    /**
     * The intent request surface for this embedder.
     */
    public val intentRequests: IntentRequests,

    /**
     * The intent ingress surface for this embedder.
     */
    public val intentEvents: IntentEvents,

    /**
     * The location request surface for this embedder.
     */
    public val locationRequests: LocationRequests,

    /**
     * The location ingress surface for this embedder.
     */
    public val locationEvents: LocationEvents,

    /**
     * The media request surface for this embedder.
     */
    public val mediaRequests: MediaRequests,

    /**
     * The notification request surface for this embedder.
     */
    public val notificationRequests: NotificationRequests,

    /**
     * The notification ingress surface for this embedder.
     */
    public val notificationEvents: NotificationEvents,

    /**
     * The primary renderer surface for this embedder.
     */
    rendererSurface: RendererSurface,
) {
    private data class PendingPermissionRequest(
        val requestId: HostRequestId,
        val permission: String,
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
        permission: String,
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
    internal fun pendingPermission(): String? {
        return pendingPermissionRequest?.permission
    }

    /**
     * Restore the in-flight permission request when one exists.
     */
    internal fun restorePermissionRequest(
        requestId: HostRequestId?,
        permission: String?,
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
    internal fun finishPermissionRequest(): Pair<HostRequestId, String> {
        val state = requireNotNull(pendingPermissionRequest) {
            "permission result arrived without one in-flight request"
        }

        pendingPermissionRequest = null

        return state.requestId to state.permission
    }
}
