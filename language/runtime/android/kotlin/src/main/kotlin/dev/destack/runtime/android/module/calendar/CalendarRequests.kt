package dev.destack.runtime.android.module.calendar

import dev.destack.runtime.android.core.hostStatusNotSupported

/**
 * The calendar request surface for one Android runtime host.
 */
public interface CalendarRequests {
    /**
     * List readable calendars through the host.
     */
    public fun listCalendars(): RuntimeHostCalendarListResponse

    /**
     * List calendar events through the host.
     */
    public fun listCalendarEvents(
        query: RuntimeHostCalendarEventQuery,
    ): RuntimeHostCalendarEventListResponse

    /**
     * Read one calendar event by stable identifier.
     */
    public fun readCalendarEvent(
        id: String,
    ): RuntimeHostCalendarEventResponse

    /**
     * Create one calendar event and return its stable identifier.
     */
    public fun createCalendarEvent(
        draft: RuntimeHostCalendarEventDraft,
    ): RuntimeHostCalendarEventCreateResponse

    /**
     * Update one calendar event by stable identifier.
     */
    public fun updateCalendarEvent(
        id: String,
        draft: RuntimeHostCalendarEventDraft,
    ): Int

    /**
     * Delete one calendar event by stable identifier.
     */
    public fun deleteCalendarEvent(
        id: String,
    ): Int
}

/**
 * The explicit unsupported calendar request surface for one Android runtime host.
 */
public object UnsupportedCalendarRequests : CalendarRequests {
    override fun listCalendars(): RuntimeHostCalendarListResponse {
        return RuntimeHostCalendarListResponse(status = hostStatusNotSupported)
    }

    override fun listCalendarEvents(
        query: RuntimeHostCalendarEventQuery,
    ): RuntimeHostCalendarEventListResponse {
        return RuntimeHostCalendarEventListResponse(status = hostStatusNotSupported)
    }

    override fun readCalendarEvent(
        id: String,
    ): RuntimeHostCalendarEventResponse {
        return RuntimeHostCalendarEventResponse(status = hostStatusNotSupported)
    }

    override fun createCalendarEvent(
        draft: RuntimeHostCalendarEventDraft,
    ): RuntimeHostCalendarEventCreateResponse {
        return RuntimeHostCalendarEventCreateResponse(status = hostStatusNotSupported)
    }

    override fun updateCalendarEvent(
        id: String,
        draft: RuntimeHostCalendarEventDraft,
    ): Int {
        return hostStatusNotSupported
    }

    override fun deleteCalendarEvent(
        id: String,
    ): Int {
        return hostStatusNotSupported
    }
}
