package dev.destack.runtime.android.module.notification

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import dev.destack.runtime.android.core.hostStatusFailed
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusNotSupported
import dev.destack.runtime.android.core.hostStatusOk

private const val defaultNotificationChannelId: String = "destack.runtime.default"
private const val defaultNotificationChannelName: String = "Destack"

/**
 * The Android notification request surface backed by one application context.
 */
public class ContextNotificationRequests(
    private val context: Context,
) : NotificationRequests {
    override fun post(
        request: RuntimeHostNotificationRequest,
    ): Int {
        // validate the notification request before posting
        if (request.tag.isBlank()) {
            return hostStatusInvalidArgument
        }

        // keep the mobile host surface honest until richer notification support lands
        if (
            request.subtitle != null ||
            request.channelId != null ||
            request.badgeCount != null ||
            request.sound != null ||
            request.categoryId != null ||
            request.threadId != null ||
            request.actionId != null ||
            request.priority != RuntimeHostNotificationPriority.Normal ||
            request.trigger.kind != RuntimeHostNotificationTriggerKind.Immediate
        ) {
            return hostStatusNotSupported
        }

        val manager = context.getSystemService(NotificationManager::class.java)
            ?: return hostStatusNotSupported

        val iconResource = context.applicationInfo.icon
        if (iconResource == 0) {
            return hostStatusNotSupported
        }

        ensureDefaultChannel(manager)

        val notification = buildNotification(
            context = context,
            request = request,
            iconResource = iconResource,
        )

        return try {
            manager.notify(
                request.tag,
                notificationId(request.tag),
                notification,
            )
            hostStatusOk
        }
        catch (_: IllegalArgumentException) {
            hostStatusInvalidArgument
        }
        catch (_: SecurityException) {
            hostStatusNotSupported
        }
        catch (_: Exception) {
            hostStatusFailed
        }
    }

    override fun cancel(
        identifier: String,
    ): Int {
        // validate the notification identifier before canceling
        if (identifier.isBlank()) {
            return hostStatusInvalidArgument
        }

        val manager = context.getSystemService(NotificationManager::class.java)
            ?: return hostStatusNotSupported

        return try {
            manager.cancel(identifier, notificationId(identifier))
            hostStatusOk
        }
        catch (_: Exception) {
            hostStatusFailed
        }
    }

    override fun cancelAll(): Int {
        val manager = context.getSystemService(NotificationManager::class.java)
            ?: return hostStatusNotSupported

        return try {
            manager.cancelAll()
            hostStatusOk
        }
        catch (_: Exception) {
            hostStatusFailed
        }
    }
}

/**
 * Build one Android notification from one runtime request.
 */
@Suppress("DEPRECATION")
private fun buildNotification(
    context: Context,
    request: RuntimeHostNotificationRequest,
    iconResource: Int,
): Notification {
    val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        Notification.Builder(context, defaultNotificationChannelId)
    } else {
        Notification.Builder(context)
    }

    return builder
        .setSmallIcon(iconResource)
        .setContentTitle(request.title)
        .setContentText(request.body)
        .setAutoCancel(true)
        .build()
}

/**
 * Ensure the default Android notification channel exists when channels are required.
 */
private fun ensureDefaultChannel(
    manager: NotificationManager,
) {
    // pre-oreo devices do not require channels
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
        return
    }

    val existingChannel = manager.getNotificationChannel(defaultNotificationChannelId)
    if (existingChannel != null) {
        return
    }

    val channel = NotificationChannel(
        defaultNotificationChannelId,
        defaultNotificationChannelName,
        NotificationManager.IMPORTANCE_DEFAULT,
    )
    manager.createNotificationChannel(channel)
}

/**
 * Derive one stable Android notification integer id from one runtime tag.
 */
private fun notificationId(
    identifier: String,
): Int {
    return identifier.hashCode()
}
