package dev.destack.runtime.android.bridge.background

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.bridge.RuntimeHostLibraryLoader
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEvent
import dev.destack.runtime.android.module.background.RuntimeHostBackgroundEventKind

/**
 * The default background ABI resolved through the Android JNI bridge.
 */
internal object ProcessBackgroundAbi : BackgroundAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun notifyBackgroundEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostBackgroundEvent,
    ): RuntimeAbiStatus {
        val values = nativeNotifyBackgroundEvent(
            sessionHandle = sessionHandle.rawValue,
            kind = when (event.kind) {
                RuntimeHostBackgroundEventKind.TaskReady -> 1
                RuntimeHostBackgroundEventKind.TaskExpired -> 2
            },
            timestampNs = event.metadata.timestampNs,
            sequence = event.metadata.sequence,
            identifier = event.metadata.identifier,
            executionId = event.metadata.executionId,
            deadlineUnixNs = event.metadata.deadlineUnixNs,
        )

        return RuntimeAbiStatus(
            code = values[0].toInt(),
            errorId = values[1],
        )
    }

    private external fun nativeNotifyBackgroundEvent(
        sessionHandle: Long,
        kind: Int,
        timestampNs: Long,
        sequence: Long,
        identifier: String,
        executionId: String,
        deadlineUnixNs: Long,
    ): LongArray
}
