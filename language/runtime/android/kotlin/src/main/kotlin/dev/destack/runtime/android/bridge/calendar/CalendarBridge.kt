package dev.destack.runtime.android.bridge.calendar

import dev.destack.runtime.android.core.RuntimeHost
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventCreateResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventDraft
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventListResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventQuery
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarListResponse

/**
 * One calendar bridge lane for one attached Android runtime host.
 */
internal class CalendarBridge {
    /**
     * List calendars through the attached runtime host.
     */
    fun listCalendars(
        runtimeHost: RuntimeHost,
    ): RuntimeHostCalendarListResponse {
        return runtimeHost.calendarRequests.listCalendars()
    }

    /**
     * List calendar events through the attached runtime host.
     */
    fun listCalendarEvents(
        runtimeHost: RuntimeHost,
        query: RuntimeHostCalendarEventQuery,
    ): RuntimeHostCalendarEventListResponse {
        return runtimeHost.calendarRequests.listCalendarEvents(query)
    }

    /**
     * Read one calendar event through the attached runtime host.
     */
    fun readCalendarEvent(
        runtimeHost: RuntimeHost,
        id: String,
    ): RuntimeHostCalendarEventResponse {
        return runtimeHost.calendarRequests.readCalendarEvent(id)
    }

    /**
     * Create one calendar event through the attached runtime host.
     */
    fun createCalendarEvent(
        runtimeHost: RuntimeHost,
        draft: RuntimeHostCalendarEventDraft,
    ): RuntimeHostCalendarEventCreateResponse {
        return runtimeHost.calendarRequests.createCalendarEvent(draft)
    }

    /**
     * Update one calendar event through the attached runtime host.
     */
    fun updateCalendarEvent(
        runtimeHost: RuntimeHost,
        id: String,
        draft: RuntimeHostCalendarEventDraft,
    ): Int {
        return runtimeHost.calendarRequests.updateCalendarEvent(id, draft)
    }

    /**
     * Delete one calendar event through the attached runtime host.
     */
    fun deleteCalendarEvent(
        runtimeHost: RuntimeHost,
        id: String,
    ): Int {
        return runtimeHost.calendarRequests.deleteCalendarEvent(id)
    }
}
