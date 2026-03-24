package dev.destack.runtime.android.module.notification

/**
 * The notification request surface attached to one Android runtime host.
 */
public interface NotificationRequests {
    /**
     * Post one notification through the Android host.
     */
    public fun postNotification(
        request: RuntimeHostNotificationRequest,
    ): Int

    /**
     * Cancel one posted notification through the Android host.
     */
    public fun cancelNotification(
        identifier: String,
    ): Int

    /**
     * Cancel every posted notification through the Android host.
     */
    public fun cancelAllNotifications(): Int
}
