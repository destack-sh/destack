package dev.destack.runtime.android.module.permission

import android.Manifest

import dev.destack.runtime.android.core.HostRequestId

/**
 * One Android permission selector.
 */
public enum class RuntimeHostPermission(
    /**
     * The stable ABI discriminant.
     */
    public val rawValue: Int,
) {
    /**
     * Location.
     */
    Location(1),

    /**
     * LocationBackground.
     */
    LocationBackground(2),

    /**
     * Camera.
     */
    Camera(3),

    /**
     * Microphone.
     */
    Microphone(4),

    /**
     * Bluetooth.
     */
    Bluetooth(5),

    /**
     * Notifications.
     */
    Notifications(6),

    /**
     * ContactsRead.
     */
    ContactsRead(7),

    /**
     * ContactsWrite.
     */
    ContactsWrite(8),

    /**
     * MediaRead.
     */
    MediaRead(9),

    /**
     * MediaWrite.
     */
    MediaWrite(10),

    /**
     * Motion.
     */
    Motion(11),

    /**
     * ClipboardRead.
     */
    ClipboardRead(12),

    /**
     * CalendarRead.
     */
    CalendarRead(13),

    /**
     * CalendarWrite.
     */
    CalendarWrite(14),
}

/**
 * Return the canonical host permission token for one runtime selector.
 */
internal fun runtimeHostPermissionName(
    permission: RuntimeHostPermission,
): String {
    return when (permission) {
        RuntimeHostPermission.Location -> "location"
        RuntimeHostPermission.LocationBackground -> "locationBackground"
        RuntimeHostPermission.Camera -> "camera"
        RuntimeHostPermission.Microphone -> "microphone"
        RuntimeHostPermission.Bluetooth -> "bluetooth"
        RuntimeHostPermission.Notifications -> "notifications"
        RuntimeHostPermission.ContactsRead -> "contactsRead"
        RuntimeHostPermission.ContactsWrite -> "contactsWrite"
        RuntimeHostPermission.MediaRead -> "mediaRead"
        RuntimeHostPermission.MediaWrite -> "mediaWrite"
        RuntimeHostPermission.Motion -> "motion"
        RuntimeHostPermission.ClipboardRead -> "clipboardRead"
        RuntimeHostPermission.CalendarRead -> "calendarRead"
        RuntimeHostPermission.CalendarWrite -> "calendarWrite"
    }
}

/**
 * Decode one runtime selector from one canonical host permission token.
 */
internal fun decodeRuntimeHostPermission(
    permissionName: String,
): RuntimeHostPermission? {
    return when (permissionName) {
        "location" -> RuntimeHostPermission.Location
        "locationBackground" -> RuntimeHostPermission.LocationBackground
        "camera" -> RuntimeHostPermission.Camera
        "microphone" -> RuntimeHostPermission.Microphone
        "bluetooth" -> RuntimeHostPermission.Bluetooth
        "notifications", "notification" -> RuntimeHostPermission.Notifications
        "contactsRead" -> RuntimeHostPermission.ContactsRead
        "contactsWrite" -> RuntimeHostPermission.ContactsWrite
        "mediaRead" -> RuntimeHostPermission.MediaRead
        "mediaWrite" -> RuntimeHostPermission.MediaWrite
        "motion" -> RuntimeHostPermission.Motion
        "clipboardRead" -> RuntimeHostPermission.ClipboardRead
        "calendarRead" -> RuntimeHostPermission.CalendarRead
        "calendarWrite" -> RuntimeHostPermission.CalendarWrite
        else -> null
    }
}

/**
 * One Android permission request submitted by one runtime session.
 */
public data class RuntimeHostPermissionRequest(
    /**
     * The stable request identifier for this interactive host flow.
     */
    val requestId: HostRequestId,

    /**
     * The normalized permission name requested by the runtime.
     */
    val permission: RuntimeHostPermission,
)

/**
 * One Android permission result event delivered into one runtime session.
 */
public data class RuntimeHostPermissionEvent(
    /**
     * The stable request identifier for this interactive host flow.
     */
    val requestId: HostRequestId,

    /**
     * The normalized permission name associated with this result.
     */
    val permission: RuntimeHostPermission,

    /**
     * Whether the Android host granted the permission.
     */
    val isGranted: Boolean,
)

/**
 * Return the Android permission constant for one runtime permission selector.
 */
internal fun androidPermissionName(
    permission: RuntimeHostPermission,
): String? {
    return when (permission) {
        RuntimeHostPermission.Location -> Manifest.permission.ACCESS_FINE_LOCATION
        RuntimeHostPermission.LocationBackground -> Manifest.permission.ACCESS_BACKGROUND_LOCATION
        RuntimeHostPermission.Camera -> Manifest.permission.CAMERA
        RuntimeHostPermission.Microphone -> Manifest.permission.RECORD_AUDIO
        RuntimeHostPermission.Bluetooth -> Manifest.permission.BLUETOOTH_CONNECT
        RuntimeHostPermission.ContactsRead -> Manifest.permission.READ_CONTACTS
        RuntimeHostPermission.ContactsWrite -> Manifest.permission.WRITE_CONTACTS
        RuntimeHostPermission.MediaRead -> Manifest.permission.READ_MEDIA_IMAGES
        RuntimeHostPermission.MediaWrite -> null
        RuntimeHostPermission.Motion -> Manifest.permission.ACTIVITY_RECOGNITION
        RuntimeHostPermission.CalendarRead -> Manifest.permission.READ_CALENDAR
        RuntimeHostPermission.CalendarWrite -> Manifest.permission.WRITE_CALENDAR
        RuntimeHostPermission.Notifications -> Manifest.permission.POST_NOTIFICATIONS
        RuntimeHostPermission.ClipboardRead -> null
    }
}
