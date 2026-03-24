package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostRequestId
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.document.RuntimeHostDocumentDescriptor
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
import dev.destack.runtime.android.module.location.RuntimeHostLocationSample
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.permission.RuntimeHostPermissionEvent

/**
 * Record runtime ABI interactions for bridge tests.
 */
class RuntimeAbiSpy : RuntimeAbi {
    val attachedSessionHandles: MutableList<HostSessionHandle> = mutableListOf()
    val detachedSessionHandles: MutableList<HostSessionHandle> = mutableListOf()
    val documentResults: MutableList<Pair<HostSessionHandle, dev.destack.runtime.android.module.document.RuntimeHostDocumentResult>> =
        mutableListOf()
    val permissionEvents: MutableList<Pair<HostSessionHandle, RuntimeHostPermissionEvent>> =
        mutableListOf()
    val intentEvents: MutableList<Pair<HostSessionHandle, RuntimeHostIntentEvent>> =
        mutableListOf()
    val locationSamples:
        MutableList<Triple<HostSessionHandle, String, RuntimeHostLocationSample>> = mutableListOf()
    val notificationEvents:
        MutableList<Pair<HostSessionHandle, RuntimeHostNotificationEvent>> = mutableListOf()
    var attachStatus: Int = 0

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

    override fun notifyDocumentResult(
        sessionHandle: HostSessionHandle,
        requestId: HostRequestId,
        documents: List<RuntimeHostDocumentDescriptor>,
    ): RuntimeAbiStatus {
        val result = dev.destack.runtime.android.module.document.RuntimeHostDocumentResult(
            requestId = requestId,
            documents = documents,
        )
        documentResults += sessionHandle to result

        return RuntimeAbiStatus(code = 0, errorId = 0)
    }

    override fun notifyPermissionResult(
        sessionHandle: HostSessionHandle,
        requestId: HostRequestId?,
        permission: String,
        isGranted: Boolean,
    ): RuntimeAbiStatus {
        val event = RuntimeHostPermissionEvent(
            requestId = requestId ?: HostRequestId(rawValue = 0),
            permission = permission,
            isGranted = isGranted,
        )
        permissionEvents += sessionHandle to event

        return RuntimeAbiStatus(code = 0, errorId = 0)
    }

    override fun notifyIntentEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostIntentEvent,
    ): RuntimeAbiStatus {
        intentEvents += sessionHandle to event

        return RuntimeAbiStatus(code = 0, errorId = 0)
    }

    override fun notifyLocationSample(
        sessionHandle: HostSessionHandle,
        watchId: String,
        sample: RuntimeHostLocationSample,
    ): RuntimeAbiStatus {
        locationSamples += Triple(sessionHandle, watchId, sample)

        return RuntimeAbiStatus(code = 0, errorId = 0)
    }

    override fun notifyNotificationEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostNotificationEvent,
        sequence: Long,
        timestampNs: Long,
    ): RuntimeAbiStatus {
        notificationEvents += sessionHandle to event

        return RuntimeAbiStatus(code = 0, errorId = 0)
    }
}
