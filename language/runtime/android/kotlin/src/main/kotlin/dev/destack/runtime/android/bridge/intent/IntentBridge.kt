package dev.destack.runtime.android.bridge.intent

import dev.destack.runtime.android.bridge.RuntimeAbi
import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.module.intent.IntentEvents
import dev.destack.runtime.android.module.intent.RuntimeHostIntentEvent
/**
 * One intent bridge lane for one attached Android runtime host.
 */
internal class IntentBridge(
    private val sessionHandle: HostSessionHandle,
    private val bindings: RuntimeAbi,
) : IntentEvents {
    /**
     * Send one intent event into the runtime ingress path.
     */
    override fun sendIntentEvent(
        event: RuntimeHostIntentEvent,
    ) {
        val status = bindings.notifyIntentEvent(
            sessionHandle = sessionHandle,
            event = event,
        )

        require(status.code == hostStatusOk) {
            "runtime bridge could not deliver intent event: code ${status.code}, error ${status.errorId}"
        }
    }

    /**
     * Return whether one outbound URL can be opened by the attached host.
     */
    fun canOpenUrl(
        runtimeHost: RuntimeHost,
        url: String,
    ): Boolean {
        return runtimeHost.intentRequests.canOpenUrl(url)
    }

    /**
     * Open one outbound URL through the attached host.
     */
    fun openUrl(
        runtimeHost: RuntimeHost,
        url: String,
    ): Int {
        return runtimeHost.intentRequests.openUrl(url)
    }

    /**
     * Open one outbound path through the attached host.
     */
    fun openPath(
        runtimeHost: RuntimeHost,
        path: String,
    ): Int {
        return runtimeHost.intentRequests.openPath(path)
    }

    /**
     * Share one outbound text payload through the attached host.
     */
    fun shareText(
        runtimeHost: RuntimeHost,
        text: String,
        contentType: String?,
    ): Int {
        return runtimeHost.intentRequests.shareText(text, contentType)
    }

    /**
     * Share one outbound file-path list through the attached host.
     */
    fun sharePaths(
        runtimeHost: RuntimeHost,
        paths: List<String>,
        contentType: String?,
    ): Int {
        return runtimeHost.intentRequests.sharePaths(paths, contentType)
    }
}
