package dev.destack.runtime.android.module.calendar

/**
 * The default calendar event page size when one query omits an explicit limit.
 */
internal const val runtimeHostCalendarDefaultEventLimit: Int = 100

/**
 * The Android calendar access level returned by one host surface.
 */
public enum class RuntimeHostCalendarAccess {
    /**
     * One read-only calendar.
     */
    Read,

    /**
     * One read-write calendar.
     */
    Write,
}

/**
 * The Android calendar availability class for one event.
 */
public enum class RuntimeHostCalendarAvailability {
    /**
     * One busy slot.
     */
    Busy,

    /**
     * One free slot.
     */
    Free,

    /**
     * One tentative slot.
     */
    Tentative,

    /**
     * One out-of-office slot.
     */
    OutOfOffice,

    /**
     * One unavailable slot.
     */
    Unavailable,

    /**
     * One unknown slot.
     */
    Unknown,
}

/**
 * The Android participant response status for one attendee.
 */
public enum class RuntimeHostCalendarParticipantStatus {
    /**
     * One unknown response.
     */
    Unknown,

    /**
     * One pending response.
     */
    Pending,

    /**
     * One accepted response.
     */
    Accepted,

    /**
     * One tentative response.
     */
    Tentative,

    /**
     * One declined response.
     */
    Declined,

    /**
     * One delegated response.
     */
    Delegated,

    /**
     * One completed response.
     */
    Completed,

    /**
     * One in-process response.
     */
    InProcess,
}

/**
 * The Android recurrence frequency for one event rule.
 */
public enum class RuntimeHostCalendarRecurrenceFrequency {
    /**
     * One daily recurrence.
     */
    Daily,

    /**
     * One weekly recurrence.
     */
    Weekly,

    /**
     * One monthly recurrence.
     */
    Monthly,

    /**
     * One yearly recurrence.
     */
    Yearly,
}

/**
 * One Android calendar attendee payload.
 */
public data class RuntimeHostCalendarAttendee(
    /**
     * The stable attendee identifier when available.
     */
    val id: String? = null,

    /**
     * The attendee display name when available.
     */
    val name: String? = null,

    /**
     * The attendee email address when available.
     */
    val email: String? = null,

    /**
     * Whether this attendee is optional.
     */
    val optional: Boolean = false,

    /**
     * Whether this attendee is the organizer.
     */
    val organizer: Boolean = false,

    /**
     * The attendee response status.
     */
    val responseStatus: RuntimeHostCalendarParticipantStatus =
        RuntimeHostCalendarParticipantStatus.Unknown,
)

/**
 * One Android recurrence weekday payload.
 */
public data class RuntimeHostCalendarRecurrenceWeekday(
    /**
     * The ISO-8601 weekday number.
     */
    val day: Int,

    /**
     * The optional week-number ordinal.
     */
    val weekNumber: Int? = null,
)

/**
 * One Android absolute reminder payload.
 */
public data class RuntimeHostCalendarAbsoluteReminder(
    /**
     * The absolute reminder timestamp in UTC nanoseconds.
     */
    val absoluteUnixNs: Long,
)

/**
 * One Android relative reminder payload.
 */
public data class RuntimeHostCalendarRelativeReminder(
    /**
     * The reminder offset in minutes before the event start.
     */
    val minutesBeforeStart: Int,
)

/**
 * One Android reminder payload.
 */
public sealed interface RuntimeHostCalendarReminder {
    /**
     * One absolute reminder payload.
     */
    public data class Absolute(
        /**
         * The absolute reminder payload.
         */
        val value: RuntimeHostCalendarAbsoluteReminder,
    ) : RuntimeHostCalendarReminder

    /**
     * One relative reminder payload.
     */
    public data class Relative(
        /**
         * The relative reminder payload.
         */
        val value: RuntimeHostCalendarRelativeReminder,
    ) : RuntimeHostCalendarReminder
}

/**
 * One Android recurrence rule payload.
 */
public data class RuntimeHostCalendarRecurrenceRule(
    /**
     * The recurrence frequency.
     */
    val frequency: RuntimeHostCalendarRecurrenceFrequency,

    /**
     * The recurrence interval.
     */
    val interval: Int,

    /**
     * The optional occurrence count.
     */
    val count: Int? = null,

    /**
     * The optional end timestamp in UTC nanoseconds.
     */
    val untilUnixNs: Long? = null,

    /**
     * The weekday numbers in ISO-8601 encoding.
     */
    val byWeekDays: List<Int> = emptyList(),

    /**
     * The structured weekday selectors.
     */
    val byWeekdayOrdinals: List<RuntimeHostCalendarRecurrenceWeekday> = emptyList(),

    /**
     * The day-of-month set.
     */
    val byMonthDays: List<Int> = emptyList(),

    /**
     * The month set.
     */
    val byMonths: List<Int> = emptyList(),

    /**
     * The day-of-year set.
     */
    val byYearDays: List<Int> = emptyList(),

    /**
     * The week-of-year set.
     */
    val byWeekNumbers: List<Int> = emptyList(),

    /**
     * The set-position filters.
     */
    val bySetPositions: List<Int> = emptyList(),
)

/**
 * One Android calendar descriptor payload.
 */
public data class RuntimeHostCalendarDescriptor(
    /**
     * The stable calendar identifier.
     */
    val id: String,

    /**
     * The calendar title.
     */
    val title: String,

    /**
     * The source or account label.
     */
    val source: String,

    /**
     * The owner label when available.
     */
    val owner: String? = null,

    /**
     * The ARGB color value.
     */
    val colorArgb: Int = 0,

    /**
     * Whether this is the primary write target.
     */
    val primary: Boolean = false,

    /**
     * The calendar access level.
     */
    val access: RuntimeHostCalendarAccess = RuntimeHostCalendarAccess.Read,
)

/**
 * One Android calendar event payload.
 */
public data class RuntimeHostCalendarEvent(
    /**
     * The stable event identifier.
     */
    val id: String,

    /**
     * The calendar identifier.
     */
    val calendarId: String,

    /**
     * The event title.
     */
    val title: String,

    /**
     * The optional notes.
     */
    val notes: String? = null,

    /**
     * The optional location text.
     */
    val location: String? = null,

    /**
     * The start timestamp in UTC nanoseconds.
     */
    val startUnixNs: Long,

    /**
     * The end timestamp in UTC nanoseconds.
     */
    val endUnixNs: Long,

    /**
     * Whether this event is all-day.
     */
    val allDay: Boolean = false,

    /**
     * Whether this event is canceled.
     */
    val canceled: Boolean = false,

    /**
     * The timezone identifier when available.
     */
    val timeZone: String? = null,

    /**
     * The availability class.
     */
    val availability: RuntimeHostCalendarAvailability = RuntimeHostCalendarAvailability.Unknown,

    /**
     * The optional event URL.
     */
    val url: String? = null,

    /**
     * The organizer display name when available.
     */
    val organizerName: String? = null,

    /**
     * The organizer email address when available.
     */
    val organizerEmail: String? = null,

    /**
     * Whether this event is recurring.
     */
    val recurring: Boolean = false,

    /**
     * The recurrence master identifier when available.
     */
    val recurrenceMasterId: String? = null,

    /**
     * The recurrence instance timestamp when available.
     */
    val recurrenceIdUnixNs: Long? = null,

    /**
     * The recurrence rule when available.
     */
    val recurrenceRule: RuntimeHostCalendarRecurrenceRule? = null,

    /**
     * The attendee list when available.
     */
    val attendees: List<RuntimeHostCalendarAttendee>? = null,

    /**
     * The reminder list when available.
     */
    val reminders: List<RuntimeHostCalendarReminder>? = null,
)

/**
 * One Android calendar event draft payload.
 */
public data class RuntimeHostCalendarEventDraft(
    /**
     * The calendar identifier.
     */
    val calendarId: String,

    /**
     * The event title.
     */
    val title: String,

    /**
     * The optional notes.
     */
    val notes: String? = null,

    /**
     * The optional location text.
     */
    val location: String? = null,

    /**
     * The start timestamp in UTC nanoseconds.
     */
    val startUnixNs: Long,

    /**
     * The end timestamp in UTC nanoseconds.
     */
    val endUnixNs: Long,

    /**
     * Whether this event is all-day.
     */
    val allDay: Boolean = false,

    /**
     * The timezone identifier when available.
     */
    val timeZone: String? = null,

    /**
     * The availability class.
     */
    val availability: RuntimeHostCalendarAvailability = RuntimeHostCalendarAvailability.Unknown,

    /**
     * The optional event URL.
     */
    val url: String? = null,

    /**
     * The recurrence rule when available.
     */
    val recurrenceRule: RuntimeHostCalendarRecurrenceRule? = null,

    /**
     * The attendee list when available.
     */
    val attendees: List<RuntimeHostCalendarAttendee>? = null,

    /**
     * The reminder list when available.
     */
    val reminders: List<RuntimeHostCalendarReminder>? = null,
)

/**
 * One Android calendar event query payload.
 */
public data class RuntimeHostCalendarEventQuery(
    /**
     * The selected calendar identifiers, empty means all readable calendars.
     */
    val calendarIds: List<String> = emptyList(),

    /**
     * The query start timestamp in UTC nanoseconds.
     */
    val startUnixNs: Long,

    /**
     * The query end timestamp in UTC nanoseconds.
     */
    val endUnixNs: Long,

    /**
     * The maximum returned events when available.
     */
    val limit: Int? = null,

    /**
     * Whether canceled events should be included.
     */
    val includeCanceled: Boolean = false,

    /**
     * Whether declined events should be included.
     */
    val includeDeclined: Boolean = false,

    /**
     * Whether expanded recurrence instances should be included.
     */
    val includeRecurrenceInstances: Boolean = false,
)

/**
 * One Android calendar-list response returned by the host.
 */
public data class RuntimeHostCalendarListResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned calendar list when available.
     */
    val calendars: List<RuntimeHostCalendarDescriptor>? = null,
)

/**
 * One Android calendar event-list response returned by the host.
 */
public data class RuntimeHostCalendarEventListResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned event list when available.
     */
    val events: List<RuntimeHostCalendarEvent>? = null,
)

/**
 * One Android calendar event-read response returned by the host.
 */
public data class RuntimeHostCalendarEventResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The returned event when available.
     */
    val event: RuntimeHostCalendarEvent? = null,
)

/**
 * One Android calendar event-create response returned by the host.
 */
public data class RuntimeHostCalendarEventCreateResponse(
    /**
     * The host status code.
     */
    val status: Int,

    /**
     * The created event identifier when available.
     */
    val id: String? = null,
)
