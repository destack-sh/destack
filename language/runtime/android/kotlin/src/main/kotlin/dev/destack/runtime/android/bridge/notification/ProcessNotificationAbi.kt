package dev.destack.runtime.android.bridge.notification

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.bridge.RuntimeHostLibraryLoader
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent

/**
 * The default notification ABI resolved through the Android JNI bridge.
 */
internal object ProcessNotificationAbi : NotificationAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun notifyNotificationEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostNotificationEvent,
        sequence: Long,
        timestampNs: Long,
    ): RuntimeAbiStatus {
        val values = nativeNotifyNotificationEvent(
            sessionHandle.rawValue,
            event.kind.ordinal + 1,
            sequence,
            timestampNs,
            event.request.identifier,
            event.request.title,
            event.request.body,
            event.actionIdentifier,
        )

        return RuntimeAbiStatus(
            code = values[0].toInt(),
            errorId = values[1],
        )
    }

    private external fun nativeNotifyNotificationEvent(
        sessionHandle: Long,
        kind: Int,
        sequence: Long,
        timestampNs: Long,
        identifier: String,
        title: String,
        body: String,
        actionIdentifier: String?,
    ): LongArray
}
