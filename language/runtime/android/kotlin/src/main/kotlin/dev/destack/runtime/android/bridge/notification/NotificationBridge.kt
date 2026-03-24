package dev.destack.runtime.android.bridge.notification

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.module.notification.NotificationEvents
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationEvent
import dev.destack.runtime.android.module.notification.RuntimeHostNotificationRequest

/**
 * One notification bridge lane for one attached Android runtime host.
 */
internal class NotificationBridge(
    private val sessionHandle: HostSessionHandle,
    private val bindings: NotificationAbi,
    private val timestampNs: () -> Long,
) : NotificationEvents {
    private var nextSequence: Long = 1L

    /**
     * Send one notification event into the runtime ingress path.
     */
    override fun sendNotificationEvent(
        event: RuntimeHostNotificationEvent,
    ) {
        val sequence = nextSequence
        nextSequence += 1L

        val status = bindings.notifyNotificationEvent(
            sessionHandle = sessionHandle,
            event = event,
            sequence = sequence,
            timestampNs = timestampNs(),
        )

        require(status.code == hostStatusOk) {
            "runtime bridge could not deliver notification event: code ${status.code}, error ${status.errorId}"
        }
    }

    /**
     * Post one notification through the attached host.
     */
    fun postNotification(
        runtimeHost: RuntimeHost,
        request: RuntimeHostNotificationRequest,
    ): Int {
        return runtimeHost.notificationRequests.postNotification(request)
    }

    /**
     * Cancel one notification through the attached host.
     */
    fun cancelNotification(
        runtimeHost: RuntimeHost,
        identifier: String,
    ): Int {
        return runtimeHost.notificationRequests.cancelNotification(identifier)
    }

    /**
     * Cancel every notification through the attached host.
     */
    fun cancelAllNotifications(
        runtimeHost: RuntimeHost,
    ): Int {
        return runtimeHost.notificationRequests.cancelAllNotifications()
    }
}
