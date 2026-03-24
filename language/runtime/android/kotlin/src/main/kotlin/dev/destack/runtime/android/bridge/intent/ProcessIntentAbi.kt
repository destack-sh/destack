package dev.destack.runtime.android.bridge.intent

import dev.destack.runtime.android.bridge.RuntimeAbiStatus
import dev.destack.runtime.android.bridge.RuntimeHostLibraryLoader
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
import dev.destack.runtime.android.module.intent.RuntimeHostIntentPayload

/**
 * The default intent ingress ABI resolved through the Android JNI bridge.
 */
internal object ProcessIntentAbi : IntentAbi {
    init {
        RuntimeHostLibraryLoader.ensureLoaded()
    }

    override fun notifyIntentEvent(
        sessionHandle: HostSessionHandle,
        event: RuntimeHostIntentEvent,
    ): RuntimeAbiStatus {
        val values = when (val payload = event.payload) {
            is RuntimeHostIntentPayload.OpenUrl -> {
                nativeNotifyIntentOpenUrl(
                    sessionHandle = sessionHandle.rawValue,
                    hasSource = event.source != null,
                    source = event.source.orEmpty(),
                    url = payload.url,
                )
            }

            is RuntimeHostIntentPayload.OpenFile -> {
                nativeNotifyIntentOpenFile(
                    sessionHandle = sessionHandle.rawValue,
                    hasSource = event.source != null,
                    source = event.source.orEmpty(),
                    path = payload.path,
                    hasContentType = payload.contentType != null,
                    contentType = payload.contentType.orEmpty(),
                )
            }

            is RuntimeHostIntentPayload.ShareText -> {
                nativeNotifyIntentShareText(
                    sessionHandle = sessionHandle.rawValue,
                    hasSource = event.source != null,
                    source = event.source.orEmpty(),
                    text = payload.text,
                    hasContentType = payload.contentType != null,
                    contentType = payload.contentType.orEmpty(),
                )
            }

            is RuntimeHostIntentPayload.ShareFiles -> {
                nativeNotifyIntentShareFiles(
                    sessionHandle = sessionHandle.rawValue,
                    hasSource = event.source != null,
                    source = event.source.orEmpty(),
                    paths = payload.paths.toTypedArray(),
                    hasContentType = payload.contentType != null,
                    contentType = payload.contentType.orEmpty(),
                )
            }

            is RuntimeHostIntentPayload.CustomAction -> {
                nativeNotifyIntentCustomAction(
                    sessionHandle = sessionHandle.rawValue,
                    hasSource = event.source != null,
                    source = event.source.orEmpty(),
                    action = payload.action,
                    hasUrl = payload.url != null,
                    url = payload.url.orEmpty(),
                    paths = payload.paths.toTypedArray(),
                    hasText = payload.text != null,
                    text = payload.text.orEmpty(),
                    hasContentType = payload.contentType != null,
                    contentType = payload.contentType.orEmpty(),
                )
            }
        }

        return RuntimeAbiStatus(
            code = values[0].toInt(),
            errorId = values[1],
        )
    }

    private external fun nativeNotifyIntentOpenUrl(
        sessionHandle: Long,
        hasSource: Boolean,
        source: String,
        url: String,
    ): LongArray

    private external fun nativeNotifyIntentOpenFile(
        sessionHandle: Long,
        hasSource: Boolean,
        source: String,
        path: String,
        hasContentType: Boolean,
        contentType: String,
    ): LongArray

    private external fun nativeNotifyIntentShareText(
        sessionHandle: Long,
        hasSource: Boolean,
        source: String,
        text: String,
        hasContentType: Boolean,
        contentType: String,
    ): LongArray

    private external fun nativeNotifyIntentShareFiles(
        sessionHandle: Long,
        hasSource: Boolean,
        source: String,
        paths: Array<String>,
        hasContentType: Boolean,
        contentType: String,
    ): LongArray

    private external fun nativeNotifyIntentCustomAction(
        sessionHandle: Long,
        hasSource: Boolean,
        source: String,
        action: String,
        hasUrl: Boolean,
        url: String,
        paths: Array<String>,
        hasText: Boolean,
        text: String,
        hasContentType: Boolean,
        contentType: String,
    ): LongArray
}
