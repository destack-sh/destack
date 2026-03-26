package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.bridge.calendar.CalendarAbi
import dev.destack.runtime.android.bridge.background.BackgroundAbi
import dev.destack.runtime.android.bridge.document.DocumentAbi
import dev.destack.runtime.android.bridge.intent.IntentAbi
import dev.destack.runtime.android.bridge.location.LocationAbi
import dev.destack.runtime.android.bridge.notification.NotificationAbi
import dev.destack.runtime.android.bridge.permission.PermissionAbi
import dev.destack.runtime.android.bridge.text.TextAbi
import dev.destack.runtime.android.core.HostSessionHandle

/**
 * One runtime status returned by one runtime ABI ingress call.
 */
public data class RuntimeAbiStatus(
    /**
     * The status code, where zero means success.
     */
    val code: Int,

    /**
     * The recorded runtime error identifier when one failure occurred.
     */
    val errorId: Long,
)

/**
 * The low-level runtime ABI surface for one Android host bridge.
 */
public interface RuntimeAbi :
    BackgroundAbi,
    CalendarAbi,
    DocumentAbi,
    IntentAbi,
    LocationAbi,
    NotificationAbi,
    PermissionAbi,
    TextAbi {
    /**
     * Attach one bridge instance to one runtime session.
     */
    public fun attachBridge(
        sessionHandle: HostSessionHandle,
        bridge: RuntimeBridge,
    ): Int

    /**
     * Detach one bridge instance from one runtime session.
     */
    public fun detachBridge(
        sessionHandle: HostSessionHandle,
    )
}
