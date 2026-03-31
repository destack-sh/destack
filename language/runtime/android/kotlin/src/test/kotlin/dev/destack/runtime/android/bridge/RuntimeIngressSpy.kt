package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEvent
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.document.RuntimeHostDocumentResult
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent
import dev.destack.runtime.android.module.text.RuntimeHostTextInputEvent

/**
 * Record runtime ingress interactions for bridge tests.
 */
class RuntimeIngressSpy : RuntimeIngress {
    val attachedSessionHandles: MutableList<HostSessionHandle> = mutableListOf()
    val detachedSessionHandles: MutableList<HostSessionHandle> = mutableListOf()
    val documentResults: MutableList<Pair<HostSessionHandle, RuntimeHostDocumentResult>> =
        mutableListOf()
    val permissionEvents: MutableList<Pair<HostSessionHandle, RuntimeHostPermissionEvent>> =
        mutableListOf()
    val intentEvents: MutableList<Pair<HostSessionHandle, RuntimeHostIntentEvent>> =
        mutableListOf()
    val locationSamples:
        MutableList<Triple<HostSessionHandle, String, RuntimeHostLocationSample>> = mutableListOf()
    val notificationEvents:
        MutableList<Pair<HostSessionHandle, RuntimeHostNotificationEvent>> = mutableListOf()
    val textInputEvents:
        MutableList<Pair<HostSessionHandle, RuntimeHostTextInputEvent>> = mutableListOf()
    val backgroundEvents:
        MutableList<Pair<HostSessionHandle, RuntimeHostBackgroundEvent>> = mutableListOf()
    var attachStatus: Int = 0

    override fun notifyBackgroundEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostBackgroundEvent,
    ): RuntimeIngressStatus {
        backgroundEvents += sessionHandle to event

        return RuntimeIngressStatus(code = 0, errorId = 0)
    }

    override fun notifyDocumentResult(
        sessionHandle: HostSessionHandle,
        requestId: HostRequestId,
        documents: List<RuntimeHostDocumentDescriptor>,
    ): RuntimeIngressStatus {
        documentResults += sessionHandle to RuntimeHostDocumentResult(
            requestId = requestId,
            documents = documents,
        )

        return RuntimeIngressStatus(code = 0, errorId = 0)
    }

    override fun notifyIntentEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostIntentEvent,
    ): RuntimeIngressStatus {
        intentEvents += sessionHandle to event

        return RuntimeIngressStatus(code = 0, errorId = 0)
    }

    override fun notifyLocationSample(
        sessionHandle: HostSessionHandle,
        watchId: String,
        sample: RuntimeHostLocationSample,
    ): RuntimeIngressStatus {
        locationSamples += Triple(sessionHandle, watchId, sample)

        return RuntimeIngressStatus(code = 0, errorId = 0)
    }

    override fun notifyNotificationEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostNotificationEvent,
    ): RuntimeIngressStatus {
        notificationEvents += sessionHandle to event

        return RuntimeIngressStatus(code = 0, errorId = 0)
    }

    override fun notifyPermissionResult(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostPermissionEvent,
    ): RuntimeIngressStatus {
        permissionEvents += sessionHandle to event

        return RuntimeIngressStatus(code = 0, errorId = 0)
    }

    override fun notifyTextInputState(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostTextInputEvent,
    ): RuntimeIngressStatus {
        textInputEvents += sessionHandle to event

        return RuntimeIngressStatus(code = 0, errorId = 0)
    }

    override fun attachBridge(
        sessionHandle: HostSessionHandle,
        bridge: RuntimeBridge,
    ): Int {
        attachedSessionHandles += sessionHandle

        return attachStatus
    }

    override fun detachBridge(
        sessionHandle: HostSessionHandle,
    ) {
        detachedSessionHandles += sessionHandle
    }
}
