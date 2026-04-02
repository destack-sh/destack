package dev.destack.runtime.android.module.calendar

import android.Manifest
import android.content.ContentProviderOperation
import android.content.ContentResolver
import android.content.ContentUris
import android.content.Context
import android.content.pm.PackageManager
import android.database.Cursor
import android.provider.CalendarContract

import androidx.core.content.ContextCompat
import dev.destack.runtime.android.core.hostStatusFailed
import dev.destack.runtime.android.core.hostStatusInvalidArgument
import dev.destack.runtime.android.core.hostStatusNotFound
import dev.destack.runtime.android.core.hostStatusOk
import dev.destack.runtime.android.core.hostStatusPermissionDenied

import java.time.Instant
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter

private const val nanosecondsPerMillisecond: Long = 1_000_000L
private const val nanosecondsPerMinute: Long = 60L * 1_000_000_000L
private const val writeCalendarAccessThreshold: Int = 500

private val recurrenceUntilFormatter: DateTimeFormatter = DateTimeFormatter
    .ofPattern("yyyyMMdd'T'HHmmss'Z'")
    .withZone(ZoneOffset.UTC)

/**
 * The Android calendar request surface backed by `CalendarContract`.
 */
public class CalendarProviderRequests(
    context: Context,
) : CalendarRequests {
    private val context: Context = context.applicationContext

    override fun list(): RuntimeHostCalendarListResponse {
        return list(context)
    }

    override fun eventList(
        query: RuntimeHostCalendarEventQuery,
    ): RuntimeHostCalendarEventListResponse {
        return eventList(context, query)
    }

    override fun eventRead(
        id: String,
    ): RuntimeHostCalendarEventReadResponse {
        return eventRead(context, id)
    }

    override fun eventCreate(
        draft: RuntimeHostCalendarEventDraft,
    ): RuntimeHostCalendarEventCreateResponse {
        return eventCreate(context, draft)
    }

    override fun eventUpdate(
        id: String,
        draft: RuntimeHostCalendarEventDraft,
    ): Int {
        return eventUpdate(context, id, draft)
    }

    override fun eventDelete(
        id: String,
    ): Int {
        return eventDelete(context, id)
    }
}

/**
 * List readable calendars through the Android calendar provider.
 */
private fun list(
    context: Context,
): RuntimeHostCalendarListResponse {
    // require read access before touching the provider
    if (!hasReadCalendarPermission(context)) {
        return RuntimeHostCalendarListResponse(status = hostStatusPermissionDenied)
    }

    val projection = arrayOf(
        CalendarContract.Calendars._ID,
        CalendarContract.Calendars.CALENDAR_DISPLAY_NAME,
        CalendarContract.Calendars.ACCOUNT_NAME,
        CalendarContract.Calendars.OWNER_ACCOUNT,
        CalendarContract.Calendars.CALENDAR_COLOR,
        CalendarContract.Calendars.IS_PRIMARY,
        CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL,
    )

    // materialize the visible calendar descriptors
    return try {
        context.contentResolver.query(
            CalendarContract.Calendars.CONTENT_URI,
            projection,
            null,
            null,
            "${CalendarContract.Calendars.CALENDAR_DISPLAY_NAME} COLLATE NOCASE ASC",
        )?.use { cursor ->
            val calendars = buildList {
                while (cursor.moveToNext()) {
                    add(runtimeHostCalendarDescriptor(cursor))
                }
            }

            RuntimeHostCalendarListResponse(
                status = hostStatusOk,
                calendars = calendars,
            )
        } ?: RuntimeHostCalendarListResponse(status = hostStatusFailed)
    }
    catch (_: SecurityException) {
        RuntimeHostCalendarListResponse(status = hostStatusPermissionDenied)
    }
    catch (_: Exception) {
        RuntimeHostCalendarListResponse(status = hostStatusFailed)
    }
}

/**
 * List calendar events through the Android calendar provider.
 */
private fun eventList(
    context: Context,
    query: RuntimeHostCalendarEventQuery,
): RuntimeHostCalendarEventListResponse {
    // require read access before touching the provider
    if (!hasReadCalendarPermission(context)) {
        return RuntimeHostCalendarEventListResponse(status = hostStatusPermissionDenied)
    }

    // validate the query window before executing provider work
    if (!isValidEventWindow(query.startUnixNs, query.endUnixNs)) {
        return RuntimeHostCalendarEventListResponse(status = hostStatusInvalidArgument)
    }

    val eventLimit = query.limit ?: runtimeHostCalendarDefaultEventLimit
    if (eventLimit <= 0) {
        return RuntimeHostCalendarEventListResponse(status = hostStatusInvalidArgument)
    }

    val contentResolver = context.contentResolver

    // route either the event table or the expanded instance table
    return try {
        val events = if (query.includeRecurrenceInstances) {
            loadInstanceEvents(contentResolver, query, eventLimit)
        } else {
            loadDirectEvents(contentResolver, query, eventLimit)
        }

        RuntimeHostCalendarEventListResponse(
            status = hostStatusOk,
            events = events,
        )
    }
    catch (_: SecurityException) {
        RuntimeHostCalendarEventListResponse(status = hostStatusPermissionDenied)
    }
    catch (_: Exception) {
        RuntimeHostCalendarEventListResponse(status = hostStatusFailed)
    }
}

/**
 * Read one calendar event by stable identifier through the Android calendar provider.
 */
private fun eventRead(
    context: Context,
    id: String,
): RuntimeHostCalendarEventReadResponse {
    // require read access before touching the provider
    if (!hasReadCalendarPermission(context)) {
        return RuntimeHostCalendarEventReadResponse(status = hostStatusPermissionDenied)
    }

    // decode one stable event identifier
    val eventId = id.toLongOrNull()
        ?: return RuntimeHostCalendarEventReadResponse(status = hostStatusInvalidArgument)
    val projection = eventProjection()

    return try {
        val event = context.contentResolver.query(
            ContentUris.withAppendedId(CalendarContract.Events.CONTENT_URI, eventId),
            projection,
            null,
            null,
            null,
        )?.use { cursor ->
            if (!cursor.moveToFirst()) {
                null
            } else {
                runtimeHostCalendarEvent(
                    contentResolver = context.contentResolver,
                    cursor = cursor,
                    eventId = eventId,
                    calendarId = cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Events.CALENDAR_ID)),
                    startUnixNs = millisecondsToNanoseconds(
                        cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Events.DTSTART)),
                    ),
                    endUnixNs = millisecondsToNanoseconds(
                        cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Events.DTEND)),
                    ),
                    recurrenceIdUnixNs = cursor.optionalLong(CalendarContract.Events.ORIGINAL_INSTANCE_TIME)
                        ?.let(::millisecondsToNanoseconds),
                )
            }
        }

        if (event == null) {
            RuntimeHostCalendarEventReadResponse(status = hostStatusNotFound)
        } else {
            RuntimeHostCalendarEventReadResponse(
                status = hostStatusOk,
                event = event,
            )
        }
    }
    catch (_: SecurityException) {
        RuntimeHostCalendarEventReadResponse(status = hostStatusPermissionDenied)
    }
    catch (_: Exception) {
        RuntimeHostCalendarEventReadResponse(status = hostStatusFailed)
    }
}

/**
 * Create one calendar event through the Android calendar provider.
 */
private fun eventCreate(
    context: Context,
    draft: RuntimeHostCalendarEventDraft,
): RuntimeHostCalendarEventCreateResponse {
    // require write access before mutating the provider
    if (!hasWriteCalendarPermission(context)) {
        return RuntimeHostCalendarEventCreateResponse(status = hostStatusPermissionDenied)
    }

    // validate the draft before mutating provider state
    val calendarId = draft.calendarId.toLongOrNull()
        ?: return RuntimeHostCalendarEventCreateResponse(status = hostStatusInvalidArgument)
    if (!isValidEventWindow(draft.startUnixNs, draft.endUnixNs)) {
        return RuntimeHostCalendarEventCreateResponse(status = hostStatusInvalidArgument)
    }

    val eventValues = eventContentValues(draft)

    // insert the event row before its child tables
    return try {
        val eventUri = context.contentResolver.insert(
            CalendarContract.Events.CONTENT_URI,
            eventValues,
        ) ?: return RuntimeHostCalendarEventCreateResponse(status = hostStatusFailed)
        val eventId = ContentUris.parseId(eventUri)
        upsertEventChildren(
            contentResolver = context.contentResolver,
            eventId = eventId,
            draft = draft,
        )

        RuntimeHostCalendarEventCreateResponse(
            status = hostStatusOk,
            id = eventId.toString(),
        )
    }
    catch (_: SecurityException) {
        RuntimeHostCalendarEventCreateResponse(status = hostStatusPermissionDenied)
    }
    catch (_: Exception) {
        RuntimeHostCalendarEventCreateResponse(status = hostStatusFailed)
    }
}

/**
 * Update one calendar event through the Android calendar provider.
 */
private fun eventUpdate(
    context: Context,
    id: String,
    draft: RuntimeHostCalendarEventDraft,
): Int {
    // require write access before mutating the provider
    if (!hasWriteCalendarPermission(context)) {
        return hostStatusPermissionDenied
    }

    // validate the target identifier and draft window
    val eventId = id.toLongOrNull() ?: return hostStatusInvalidArgument
    if (!isValidEventWindow(draft.startUnixNs, draft.endUnixNs)) {
        return hostStatusInvalidArgument
    }

    // update the event row and rewrite the child tables
    return try {
        val updatedCount = context.contentResolver.update(
            ContentUris.withAppendedId(CalendarContract.Events.CONTENT_URI, eventId),
            eventContentValues(draft),
            null,
            null,
        )
        if (updatedCount <= 0) {
            return hostStatusNotFound
        }

        replaceEventChildren(
            contentResolver = context.contentResolver,
            eventId = eventId,
            draft = draft,
        )

        hostStatusOk
    }
    catch (_: SecurityException) {
        hostStatusPermissionDenied
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

/**
 * Delete one calendar event through the Android calendar provider.
 */
private fun eventDelete(
    context: Context,
    id: String,
): Int {
    // require write access before mutating the provider
    if (!hasWriteCalendarPermission(context)) {
        return hostStatusPermissionDenied
    }

    // decode one stable event identifier
    val eventId = id.toLongOrNull() ?: return hostStatusInvalidArgument

    return try {
        val deletedCount = context.contentResolver.delete(
            ContentUris.withAppendedId(CalendarContract.Events.CONTENT_URI, eventId),
            null,
            null,
        )

        if (deletedCount > 0) {
            hostStatusOk
        } else {
            hostStatusNotFound
        }
    }
    catch (_: SecurityException) {
        hostStatusPermissionDenied
    }
    catch (_: Exception) {
        hostStatusFailed
    }
}

/**
 * Return whether the Android calendar read permission is granted.
 */
private fun hasReadCalendarPermission(
    context: Context,
): Boolean {
    return ContextCompat.checkSelfPermission(
        context,
        Manifest.permission.READ_CALENDAR,
    ) == PackageManager.PERMISSION_GRANTED
}

/**
 * Return whether the Android calendar write permission is granted.
 */
private fun hasWriteCalendarPermission(
    context: Context,
): Boolean {
    return ContextCompat.checkSelfPermission(
        context,
        Manifest.permission.WRITE_CALENDAR,
    ) == PackageManager.PERMISSION_GRANTED
}

/**
 * Return whether one event window is structurally valid.
 */
private fun isValidEventWindow(
    startUnixNs: Long,
    endUnixNs: Long,
): Boolean {
    return startUnixNs >= 0L && endUnixNs >= startUnixNs
}

/**
 * Return one stable event projection for direct event rows.
 */
private fun eventProjection(): Array<String> {
    return arrayOf(
        CalendarContract.Events._ID,
        CalendarContract.Events.CALENDAR_ID,
        CalendarContract.Events.TITLE,
        CalendarContract.Events.DESCRIPTION,
        CalendarContract.Events.EVENT_LOCATION,
        CalendarContract.Events.DTSTART,
        CalendarContract.Events.DTEND,
        CalendarContract.Events.ALL_DAY,
        CalendarContract.Events.STATUS,
        CalendarContract.Events.EVENT_TIMEZONE,
        CalendarContract.Events.AVAILABILITY,
        CalendarContract.Events.CUSTOM_APP_URI,
        CalendarContract.Events.ORGANIZER,
        CalendarContract.Events.RRULE,
        CalendarContract.Events.ORIGINAL_ID,
        CalendarContract.Events.ORIGINAL_INSTANCE_TIME,
    )
}

/**
 * Return one stable event projection for expanded instance rows.
 */
private fun instanceProjection(): Array<String> {
    return arrayOf(
        CalendarContract.Instances.EVENT_ID,
        CalendarContract.Instances.CALENDAR_ID,
        CalendarContract.Instances.TITLE,
        CalendarContract.Instances.DESCRIPTION,
        CalendarContract.Instances.EVENT_LOCATION,
        CalendarContract.Instances.BEGIN,
        CalendarContract.Instances.END,
        CalendarContract.Instances.ALL_DAY,
        CalendarContract.Instances.STATUS,
        CalendarContract.Instances.EVENT_TIMEZONE,
        CalendarContract.Instances.AVAILABILITY,
        CalendarContract.Instances.CUSTOM_APP_URI,
        CalendarContract.Instances.ORGANIZER,
        CalendarContract.Instances.RRULE,
        CalendarContract.Instances.ORIGINAL_ID,
        CalendarContract.Instances.ORIGINAL_INSTANCE_TIME,
    )
}

/**
 * Load direct event rows for one calendar query.
 */
private fun loadDirectEvents(
    contentResolver: ContentResolver,
    query: RuntimeHostCalendarEventQuery,
    eventLimit: Int,
): List<RuntimeHostCalendarEvent> {
    val projection = eventProjection()
    val selection = buildEventSelection(query, CalendarContract.Events.CALENDAR_ID)
    val selectionArgs = buildEventSelectionArgs(query)

    return contentResolver.query(
        CalendarContract.Events.CONTENT_URI,
        projection,
        selection,
        selectionArgs,
        "${CalendarContract.Events.DTSTART} ASC",
    )?.use { cursor ->
        val events = buildList {
            while (cursor.moveToNext()) {
                add(
                    runtimeHostCalendarEvent(
                        contentResolver = contentResolver,
                        cursor = cursor,
                        eventId = cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Events._ID)),
                        calendarId = cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Events.CALENDAR_ID)),
                        startUnixNs = millisecondsToNanoseconds(
                            cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Events.DTSTART)),
                        ),
                        endUnixNs = millisecondsToNanoseconds(
                            cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Events.DTEND)),
                        ),
                        recurrenceIdUnixNs = cursor.optionalLong(CalendarContract.Events.ORIGINAL_INSTANCE_TIME)
                            ?.let(::millisecondsToNanoseconds),
                    ),
                )
            }
        }

        events.take(eventLimit)
    } ?: emptyList()
}

/**
 * Load expanded instance rows for one calendar query.
 */
private fun loadInstanceEvents(
    contentResolver: ContentResolver,
    query: RuntimeHostCalendarEventQuery,
    eventLimit: Int,
): List<RuntimeHostCalendarEvent> {
    val projection = instanceProjection()
    val selection = buildEventSelection(query, CalendarContract.Instances.CALENDAR_ID)
    val selectionArgs = buildEventSelectionArgs(query)
    val uri = CalendarContract.Instances.CONTENT_URI.buildUpon().also { builder ->
        ContentUris.appendId(builder, nanosecondsToMilliseconds(query.startUnixNs))
        ContentUris.appendId(builder, nanosecondsToMilliseconds(query.endUnixNs))
    }.build()

    return contentResolver.query(
        uri,
        projection,
        selection,
        selectionArgs,
        "${CalendarContract.Instances.BEGIN} ASC",
    )?.use { cursor ->
        val events = buildList {
            while (cursor.moveToNext()) {
                add(
                    runtimeHostCalendarEvent(
                        contentResolver = contentResolver,
                        cursor = cursor,
                        eventId = cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Instances.EVENT_ID)),
                        calendarId = cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Instances.CALENDAR_ID)),
                        startUnixNs = millisecondsToNanoseconds(
                            cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Instances.BEGIN)),
                        ),
                        endUnixNs = millisecondsToNanoseconds(
                            cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Instances.END)),
                        ),
                        recurrenceIdUnixNs = cursor.optionalLong(CalendarContract.Instances.BEGIN)
                            ?.let(::millisecondsToNanoseconds),
                    ),
                )
            }
        }

        events.take(eventLimit)
    } ?: emptyList()
}

/**
 * Build one SQL selection for event and instance queries.
 */
private fun buildEventSelection(
    query: RuntimeHostCalendarEventQuery,
    calendarIdColumn: String,
): String? {
    val clauses = mutableListOf<String>()
    if (query.calendarIds.isNotEmpty()) {
        clauses += "$calendarIdColumn IN (${query.calendarIds.joinToString(",") { "?" }})"
    }
    if (!query.includeCanceled) {
        clauses += "(${CalendarContract.Events.STATUS} IS NULL OR ${CalendarContract.Events.STATUS} != ${CalendarContract.Events.STATUS_CANCELED})"
    }
    if (!query.includeDeclined) {
        clauses += "(${CalendarContract.Events.SELF_ATTENDEE_STATUS} IS NULL OR ${CalendarContract.Events.SELF_ATTENDEE_STATUS} != ${CalendarContract.Attendees.ATTENDEE_STATUS_DECLINED})"
    }

    return if (clauses.isEmpty()) {
        null
    } else {
        clauses.joinToString(" AND ")
    }
}

/**
 * Build one SQL selection-argument array for event and instance queries.
 */
private fun buildEventSelectionArgs(
    query: RuntimeHostCalendarEventQuery,
): Array<String>? {
    if (query.calendarIds.isEmpty()) {
        return null
    }

    return query.calendarIds.toTypedArray()
}

/**
 * Decode one runtime calendar descriptor from one provider cursor row.
 */
private fun runtimeHostCalendarDescriptor(
    cursor: Cursor,
): RuntimeHostCalendarDescriptor {
    val accessLevel = cursor.getInt(
        cursor.columnIndexOrThrow(CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL),
    )

    return RuntimeHostCalendarDescriptor(
        id = cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Calendars._ID)).toString(),
        title = cursor.getString(
            cursor.columnIndexOrThrow(CalendarContract.Calendars.CALENDAR_DISPLAY_NAME),
        ) ?: "",
        source = cursor.getString(
            cursor.columnIndexOrThrow(CalendarContract.Calendars.ACCOUNT_NAME),
        ) ?: "",
        owner = cursor.getString(
            cursor.columnIndexOrThrow(CalendarContract.Calendars.OWNER_ACCOUNT),
        ),
        colorArgb = cursor.getInt(
            cursor.columnIndexOrThrow(CalendarContract.Calendars.CALENDAR_COLOR),
        ),
        primary = cursor.getInt(
            cursor.columnIndexOrThrow(CalendarContract.Calendars.IS_PRIMARY),
        ) != 0,
        access = runtimeHostCalendarAccess(accessLevel),
    )
}

/**
 * Decode one runtime calendar event from one provider cursor row.
 */
private fun runtimeHostCalendarEvent(
    contentResolver: ContentResolver,
    cursor: Cursor,
    eventId: Long,
    calendarId: Long,
    startUnixNs: Long,
    endUnixNs: Long,
    recurrenceIdUnixNs: Long?,
): RuntimeHostCalendarEvent {
    val recurrenceRuleText = cursor.getString(cursor.columnIndexOrThrow(CalendarContract.Events.RRULE))
    val organizer = cursor.getString(cursor.columnIndexOrThrow(CalendarContract.Events.ORGANIZER))

    return RuntimeHostCalendarEvent(
        id = eventId.toString(),
        calendarId = calendarId.toString(),
        title = cursor.getString(cursor.columnIndexOrThrow(CalendarContract.Events.TITLE)) ?: "",
        notes = cursor.getString(cursor.columnIndexOrThrow(CalendarContract.Events.DESCRIPTION)),
        location = cursor.getString(
            cursor.columnIndexOrThrow(CalendarContract.Events.EVENT_LOCATION),
        ),
        startUnixNs = startUnixNs,
        endUnixNs = endUnixNs,
        allDay = cursor.getInt(cursor.columnIndexOrThrow(CalendarContract.Events.ALL_DAY)) != 0,
        canceled = cursor.getInt(cursor.columnIndexOrThrow(CalendarContract.Events.STATUS)) ==
            CalendarContract.Events.STATUS_CANCELED,
        timeZone = cursor.getString(
            cursor.columnIndexOrThrow(CalendarContract.Events.EVENT_TIMEZONE),
        ),
        availability = runtimeHostCalendarAvailability(
            cursor.getInt(cursor.columnIndexOrThrow(CalendarContract.Events.AVAILABILITY)),
        ),
        url = cursor.getString(cursor.columnIndexOrThrow(CalendarContract.Events.CUSTOM_APP_URI)),
        organizerName = null,
        organizerEmail = organizer,
        recurring = !recurrenceRuleText.isNullOrBlank() ||
            cursor.optionalLong(CalendarContract.Events.ORIGINAL_ID) != null,
        recurrenceMasterId = cursor.optionalLong(CalendarContract.Events.ORIGINAL_ID)?.toString(),
        recurrenceIdUnixNs = recurrenceIdUnixNs,
        recurrenceRule = recurrenceRuleText?.let(::decodeRecurrenceRule),
        attendees = loadEventAttendees(contentResolver, eventId),
        reminders = loadEventReminders(contentResolver, eventId),
    )
}

/**
 * Build one event content-value payload from one draft.
 */
private fun eventContentValues(
    draft: RuntimeHostCalendarEventDraft,
): android.content.ContentValues {
    val values = android.content.ContentValues()
    values.put(CalendarContract.Events.CALENDAR_ID, draft.calendarId.toLong())
    values.put(CalendarContract.Events.TITLE, draft.title)
    values.put(CalendarContract.Events.DESCRIPTION, draft.notes)
    values.put(CalendarContract.Events.EVENT_LOCATION, draft.location)
    values.put(CalendarContract.Events.DTSTART, nanosecondsToMilliseconds(draft.startUnixNs))
    values.put(CalendarContract.Events.DTEND, nanosecondsToMilliseconds(draft.endUnixNs))
    values.put(CalendarContract.Events.ALL_DAY, if (draft.allDay) 1 else 0)
    values.put(CalendarContract.Events.EVENT_TIMEZONE, draft.timeZone ?: "UTC")
    values.put(CalendarContract.Events.AVAILABILITY, encodeAvailability(draft.availability))
    values.put(CalendarContract.Events.CUSTOM_APP_URI, draft.url)

    // encode recurrence only when the draft carries one rule
    if (draft.recurrenceRule != null) {
        values.put(CalendarContract.Events.RRULE, encodeRecurrenceRule(draft.recurrenceRule))
    } else {
        values.putNull(CalendarContract.Events.RRULE)
    }

    return values
}

/**
 * Insert or rewrite the attendee and reminder child tables for one event.
 */
private fun upsertEventChildren(
    contentResolver: ContentResolver,
    eventId: Long,
    draft: RuntimeHostCalendarEventDraft,
) {
    insertEventAttendees(contentResolver, eventId, draft)
    insertEventReminders(contentResolver, eventId, draft)
}

/**
 * Replace the attendee and reminder child tables for one event.
 */
private fun replaceEventChildren(
    contentResolver: ContentResolver,
    eventId: Long,
    draft: RuntimeHostCalendarEventDraft,
) {
    contentResolver.delete(
        CalendarContract.Attendees.CONTENT_URI,
        "${CalendarContract.Attendees.EVENT_ID} = ?",
        arrayOf(eventId.toString()),
    )
    contentResolver.delete(
        CalendarContract.Reminders.CONTENT_URI,
        "${CalendarContract.Reminders.EVENT_ID} = ?",
        arrayOf(eventId.toString()),
    )
    upsertEventChildren(contentResolver, eventId, draft)
}

/**
 * Insert the attendee child rows for one event draft.
 */
private fun insertEventAttendees(
    contentResolver: ContentResolver,
    eventId: Long,
    draft: RuntimeHostCalendarEventDraft,
) {
    val attendees = draft.attendees ?: return
    if (attendees.isEmpty()) {
        return
    }

    val operations = ArrayList<ContentProviderOperation>(attendees.size)
    for (attendee in attendees) {
        val builder = ContentProviderOperation.newInsert(CalendarContract.Attendees.CONTENT_URI)
        builder.withValue(CalendarContract.Attendees.EVENT_ID, eventId)
        builder.withValue(CalendarContract.Attendees.ATTENDEE_NAME, attendee.name)
        builder.withValue(CalendarContract.Attendees.ATTENDEE_EMAIL, attendee.email)
        builder.withValue(
            CalendarContract.Attendees.ATTENDEE_STATUS,
            encodeParticipantStatus(attendee.responseStatus),
        )
        builder.withValue(
            CalendarContract.Attendees.ATTENDEE_RELATIONSHIP,
            if (attendee.organizer) {
                CalendarContract.Attendees.RELATIONSHIP_ORGANIZER
            } else {
                CalendarContract.Attendees.RELATIONSHIP_ATTENDEE
            },
        )
        builder.withValue(
            CalendarContract.Attendees.ATTENDEE_TYPE,
            if (attendee.optional) {
                CalendarContract.Attendees.TYPE_OPTIONAL
            } else {
                CalendarContract.Attendees.TYPE_REQUIRED
            },
        )
        operations += builder.build()
    }

    contentResolver.applyBatch(CalendarContract.AUTHORITY, operations)
}

/**
 * Insert the reminder child rows for one event draft.
 */
private fun insertEventReminders(
    contentResolver: ContentResolver,
    eventId: Long,
    draft: RuntimeHostCalendarEventDraft,
) {
    val reminders = draft.reminders ?: return
    if (reminders.isEmpty()) {
        return
    }

    val operations = ArrayList<ContentProviderOperation>(reminders.size)
    for (reminder in reminders) {
        val minutes = when (reminder.kind) {
            RuntimeHostCalendarReminderKind.Absolute -> {
                ((draft.startUnixNs - reminder.absoluteUnixNs) / nanosecondsPerMinute).toInt()
            }
            RuntimeHostCalendarReminderKind.Relative -> {
                reminder.minutesBeforeStart
            }
        }
        val builder = ContentProviderOperation.newInsert(CalendarContract.Reminders.CONTENT_URI)
        builder.withValue(CalendarContract.Reminders.EVENT_ID, eventId)
        builder.withValue(CalendarContract.Reminders.MINUTES, minutes)
        builder.withValue(
            CalendarContract.Reminders.METHOD,
            CalendarContract.Reminders.METHOD_ALERT,
        )
        operations += builder.build()
    }

    contentResolver.applyBatch(CalendarContract.AUTHORITY, operations)
}

/**
 * Load the attendee rows for one event identifier.
 */
private fun loadEventAttendees(
    contentResolver: ContentResolver,
    eventId: Long,
): List<RuntimeHostCalendarAttendee>? {
    val projection = arrayOf(
        CalendarContract.Attendees._ID,
        CalendarContract.Attendees.ATTENDEE_NAME,
        CalendarContract.Attendees.ATTENDEE_EMAIL,
        CalendarContract.Attendees.ATTENDEE_TYPE,
        CalendarContract.Attendees.ATTENDEE_RELATIONSHIP,
        CalendarContract.Attendees.ATTENDEE_STATUS,
    )

    return contentResolver.query(
        CalendarContract.Attendees.CONTENT_URI,
        projection,
        "${CalendarContract.Attendees.EVENT_ID} = ?",
        arrayOf(eventId.toString()),
        null,
    )?.use { cursor ->
        buildList {
            while (cursor.moveToNext()) {
                add(
                    RuntimeHostCalendarAttendee(
                        id = cursor.getLong(cursor.columnIndexOrThrow(CalendarContract.Attendees._ID)).toString(),
                        name = cursor.getString(
                            cursor.columnIndexOrThrow(CalendarContract.Attendees.ATTENDEE_NAME),
                        ),
                        email = cursor.getString(
                            cursor.columnIndexOrThrow(CalendarContract.Attendees.ATTENDEE_EMAIL),
                        ),
                        optional = cursor.getInt(
                            cursor.columnIndexOrThrow(CalendarContract.Attendees.ATTENDEE_TYPE),
                        ) == CalendarContract.Attendees.TYPE_OPTIONAL,
                        organizer = cursor.getInt(
                            cursor.columnIndexOrThrow(CalendarContract.Attendees.ATTENDEE_RELATIONSHIP),
                        ) == CalendarContract.Attendees.RELATIONSHIP_ORGANIZER,
                        responseStatus = runtimeHostParticipantStatus(
                            cursor.getInt(
                                cursor.columnIndexOrThrow(CalendarContract.Attendees.ATTENDEE_STATUS),
                            ),
                        ),
                    ),
                )
            }
        }
    }?.takeIf { it.isNotEmpty() }
}

/**
 * Load the reminder rows for one event identifier.
 */
private fun loadEventReminders(
    contentResolver: ContentResolver,
    eventId: Long,
): List<RuntimeHostCalendarReminder>? {
    val projection = arrayOf(
        CalendarContract.Reminders.MINUTES,
    )

    return contentResolver.query(
        CalendarContract.Reminders.CONTENT_URI,
        projection,
        "${CalendarContract.Reminders.EVENT_ID} = ?",
        arrayOf(eventId.toString()),
        null,
    )?.use { cursor ->
        buildList {
            while (cursor.moveToNext()) {
                add(
                    RuntimeHostCalendarReminder(
                        kind = RuntimeHostCalendarReminderKind.Relative,
                        minutesBeforeStart = cursor.getInt(
                            cursor.columnIndexOrThrow(CalendarContract.Reminders.MINUTES),
                        ),
                    ),
                )
            }
        }
    }?.takeIf { it.isNotEmpty() }
}

/**
 * Encode one runtime availability value for the Android provider.
 */
private fun encodeAvailability(
    availability: RuntimeHostCalendarAvailability,
): Int {
    return when (availability) {
        RuntimeHostCalendarAvailability.Busy -> CalendarContract.Events.AVAILABILITY_BUSY
        RuntimeHostCalendarAvailability.Free -> CalendarContract.Events.AVAILABILITY_FREE
        RuntimeHostCalendarAvailability.Tentative -> CalendarContract.Events.AVAILABILITY_TENTATIVE
        RuntimeHostCalendarAvailability.OutOfOffice -> CalendarContract.Events.AVAILABILITY_BUSY
        RuntimeHostCalendarAvailability.Unavailable -> CalendarContract.Events.AVAILABILITY_BUSY
        RuntimeHostCalendarAvailability.Unknown -> CalendarContract.Events.AVAILABILITY_BUSY
    }
}

/**
 * Decode one Android provider availability value.
 */
private fun runtimeHostCalendarAvailability(
    availability: Int,
): RuntimeHostCalendarAvailability {
    return when (availability) {
        CalendarContract.Events.AVAILABILITY_FREE -> RuntimeHostCalendarAvailability.Free
        CalendarContract.Events.AVAILABILITY_TENTATIVE -> RuntimeHostCalendarAvailability.Tentative
        CalendarContract.Events.AVAILABILITY_BUSY -> RuntimeHostCalendarAvailability.Busy
        else -> RuntimeHostCalendarAvailability.Unknown
    }
}

/**
 * Decode one Android calendar access level.
 */
private fun runtimeHostCalendarAccess(
    accessLevel: Int,
): RuntimeHostCalendarAccess {
    return if (accessLevel >= writeCalendarAccessThreshold) {
        RuntimeHostCalendarAccess.Write
    } else {
        RuntimeHostCalendarAccess.Read
    }
}

/**
 * Encode one runtime participant status for the Android provider.
 */
private fun encodeParticipantStatus(
    status: RuntimeHostCalendarParticipantStatus,
): Int {
    return when (status) {
        RuntimeHostCalendarParticipantStatus.Unknown -> CalendarContract.Attendees.ATTENDEE_STATUS_NONE
        RuntimeHostCalendarParticipantStatus.Pending -> CalendarContract.Attendees.ATTENDEE_STATUS_INVITED
        RuntimeHostCalendarParticipantStatus.Accepted -> CalendarContract.Attendees.ATTENDEE_STATUS_ACCEPTED
        RuntimeHostCalendarParticipantStatus.Tentative -> CalendarContract.Attendees.ATTENDEE_STATUS_TENTATIVE
        RuntimeHostCalendarParticipantStatus.Declined -> CalendarContract.Attendees.ATTENDEE_STATUS_DECLINED
        RuntimeHostCalendarParticipantStatus.Delegated -> CalendarContract.Attendees.ATTENDEE_STATUS_NONE
        RuntimeHostCalendarParticipantStatus.Completed -> CalendarContract.Attendees.ATTENDEE_STATUS_NONE
        RuntimeHostCalendarParticipantStatus.InProcess -> CalendarContract.Attendees.ATTENDEE_STATUS_NONE
    }
}

/**
 * Decode one Android attendee status into the runtime enum.
 */
private fun runtimeHostParticipantStatus(
    status: Int,
): RuntimeHostCalendarParticipantStatus {
    return when (status) {
        CalendarContract.Attendees.ATTENDEE_STATUS_INVITED -> RuntimeHostCalendarParticipantStatus.Pending
        CalendarContract.Attendees.ATTENDEE_STATUS_ACCEPTED -> RuntimeHostCalendarParticipantStatus.Accepted
        CalendarContract.Attendees.ATTENDEE_STATUS_TENTATIVE -> RuntimeHostCalendarParticipantStatus.Tentative
        CalendarContract.Attendees.ATTENDEE_STATUS_DECLINED -> RuntimeHostCalendarParticipantStatus.Declined
        else -> RuntimeHostCalendarParticipantStatus.Unknown
    }
}

/**
 * Encode one recurrence rule into one Android RRULE string.
 */
private fun encodeRecurrenceRule(
    rule: RuntimeHostCalendarRecurrenceRule,
): String {
    val parts = mutableListOf<String>()
    parts += "FREQ=${encodeRecurrenceFrequency(rule.frequency)}"
    parts += "INTERVAL=${rule.interval}"
    rule.count?.let { count ->
        parts += "COUNT=$count"
    }
    rule.untilUnixNs?.let { untilUnixNs ->
        parts += "UNTIL=${recurrenceUntilFormatter.format(Instant.ofEpochMilli(nanosecondsToMilliseconds(untilUnixNs)))}"
    }
    if (rule.byWeekdayOrdinals.isNotEmpty()) {
        parts += "BYDAY=${rule.byWeekdayOrdinals.joinToString(",") { weekday ->
            "${weekday.weekNumber ?: ""}${encodeWeekday(weekday.day)}"
        }}"
    } else if (rule.byWeekDays.isNotEmpty()) {
        parts += "BYDAY=${rule.byWeekDays.joinToString(",") { day ->
            encodeWeekday(day)
        }}"
    }
    if (rule.byMonthDays.isNotEmpty()) {
        parts += "BYMONTHDAY=${rule.byMonthDays.joinToString(",")}"
    }
    if (rule.byMonths.isNotEmpty()) {
        parts += "BYMONTH=${rule.byMonths.joinToString(",")}"
    }
    if (rule.byYearDays.isNotEmpty()) {
        parts += "BYYEARDAY=${rule.byYearDays.joinToString(",")}"
    }
    if (rule.byWeekNumbers.isNotEmpty()) {
        parts += "BYWEEKNO=${rule.byWeekNumbers.joinToString(",")}"
    }
    if (rule.bySetPositions.isNotEmpty()) {
        parts += "BYSETPOS=${rule.bySetPositions.joinToString(",")}"
    }

    return parts.joinToString(";")
}

/**
 * Decode one Android RRULE string into the runtime recurrence payload.
 */
private fun decodeRecurrenceRule(
    recurrenceRule: String,
): RuntimeHostCalendarRecurrenceRule? {
    val fields = recurrenceRule
        .split(';')
        .mapNotNull { field ->
            val separatorIndex = field.indexOf('=')
            if (separatorIndex <= 0) {
                null
            } else {
                field.substring(0, separatorIndex) to field.substring(separatorIndex + 1)
            }
        }
        .toMap()
    val frequency = fields["FREQ"]?.let(::decodeRecurrenceFrequency)
        ?: return null
    val interval = fields["INTERVAL"]?.toIntOrNull() ?: 1
    val weekdayTokens = fields["BYDAY"]
        ?.split(',')
        ?.filter { token ->
            token.isNotBlank()
        }
        .orEmpty()

    return RuntimeHostCalendarRecurrenceRule(
        frequency = frequency,
        interval = interval,
        count = fields["COUNT"]?.toIntOrNull(),
        untilUnixNs = fields["UNTIL"]?.let(::decodeRecurrenceUntil),
        byWeekDays = weekdayTokens.mapNotNull(::decodeWeekdayTokenDay),
        byWeekdayOrdinals = weekdayTokens.mapNotNull(::decodeWeekdayTokenOrdinal),
        byMonthDays = decodeIntegerList(fields["BYMONTHDAY"]),
        byMonths = decodeIntegerList(fields["BYMONTH"]),
        byYearDays = decodeIntegerList(fields["BYYEARDAY"]),
        byWeekNumbers = decodeIntegerList(fields["BYWEEKNO"]),
        bySetPositions = decodeIntegerList(fields["BYSETPOS"]),
    )
}

/**
 * Encode one recurrence frequency token.
 */
private fun encodeRecurrenceFrequency(
    frequency: RuntimeHostCalendarRecurrenceFrequency,
): String {
    return when (frequency) {
        RuntimeHostCalendarRecurrenceFrequency.Daily -> "DAILY"
        RuntimeHostCalendarRecurrenceFrequency.Weekly -> "WEEKLY"
        RuntimeHostCalendarRecurrenceFrequency.Monthly -> "MONTHLY"
        RuntimeHostCalendarRecurrenceFrequency.Yearly -> "YEARLY"
    }
}

/**
 * Decode one recurrence frequency token.
 */
private fun decodeRecurrenceFrequency(
    frequency: String,
): RuntimeHostCalendarRecurrenceFrequency? {
    return when (frequency) {
        "DAILY" -> RuntimeHostCalendarRecurrenceFrequency.Daily
        "WEEKLY" -> RuntimeHostCalendarRecurrenceFrequency.Weekly
        "MONTHLY" -> RuntimeHostCalendarRecurrenceFrequency.Monthly
        "YEARLY" -> RuntimeHostCalendarRecurrenceFrequency.Yearly
        else -> null
    }
}

/**
 * Encode one ISO weekday number into one RRULE token.
 */
private fun encodeWeekday(
    day: Int,
): String {
    return when (day) {
        1 -> "MO"
        2 -> "TU"
        3 -> "WE"
        4 -> "TH"
        5 -> "FR"
        6 -> "SA"
        7 -> "SU"
        else -> "MO"
    }
}

/**
 * Decode one RRULE weekday token into one day number when it has no ordinal.
 */
private fun decodeWeekdayTokenDay(
    token: String,
): Int? {
    val suffix = token.takeLast(2)
    val prefix = token.dropLast(2)
    if (prefix.isNotEmpty()) {
        return null
    }

    return decodeWeekdaySuffix(suffix)
}

/**
 * Decode one RRULE weekday token into one ordinal weekday payload when present.
 */
private fun decodeWeekdayTokenOrdinal(
    token: String,
): RuntimeHostCalendarRecurrenceWeekday? {
    if (token.length <= 2) {
        return null
    }

    val suffix = token.takeLast(2)
    val prefix = token.dropLast(2)
    val day = decodeWeekdaySuffix(suffix) ?: return null
    val weekNumber = prefix.toIntOrNull() ?: return null

    return RuntimeHostCalendarRecurrenceWeekday(
        day = day,
        weekNumber = weekNumber,
    )
}

/**
 * Decode one RRULE weekday suffix.
 */
private fun decodeWeekdaySuffix(
    suffix: String,
): Int? {
    return when (suffix) {
        "MO" -> 1
        "TU" -> 2
        "WE" -> 3
        "TH" -> 4
        "FR" -> 5
        "SA" -> 6
        "SU" -> 7
        else -> null
    }
}

/**
 * Decode one recurrence UNTIL token.
 */
private fun decodeRecurrenceUntil(
    until: String,
): Long? {
    return try {
        val instant = Instant.from(recurrenceUntilFormatter.parse(until))
        millisecondsToNanoseconds(instant.toEpochMilli())
    }
    catch (_: Exception) {
        null
    }
}

/**
 * Decode one comma-separated integer list.
 */
private fun decodeIntegerList(
    value: String?,
): List<Int> {
    return value
        ?.split(',')
        ?.mapNotNull(String::toIntOrNull)
        .orEmpty()
}

/**
 * Return one required cursor column index.
 */
private fun Cursor.columnIndexOrThrow(
    columnName: String,
): Int {
    return getColumnIndexOrThrow(columnName)
}

/**
 * Return one optional long from one cursor column.
 */
private fun Cursor.optionalLong(
    columnName: String,
): Long? {
    val columnIndex = getColumnIndex(columnName)
    if (columnIndex < 0 || isNull(columnIndex)) {
        return null
    }

    return getLong(columnIndex)
}

/**
 * Convert one millisecond timestamp into nanoseconds.
 */
private fun millisecondsToNanoseconds(
    milliseconds: Long,
): Long {
    return milliseconds * nanosecondsPerMillisecond
}

/**
 * Convert one nanosecond timestamp into milliseconds.
 */
private fun nanosecondsToMilliseconds(
    nanoseconds: Long,
): Long {
    return nanoseconds / nanosecondsPerMillisecond
}
