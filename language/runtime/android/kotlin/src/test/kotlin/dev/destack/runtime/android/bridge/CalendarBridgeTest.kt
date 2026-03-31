package dev.destack.runtime.android.bridge

import dev.destack.runtime.android.core.HostSessionHandle
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarAccess
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarAvailability
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarDescriptor
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEvent
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventCreateResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventDraft
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventListResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventQuery
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarEventReadResponse
import dev.destack.runtime.android.module.calendar.RuntimeHostCalendarListResponse

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Exercise the calendar bridge lane.
 */
class CalendarBridgeTest {
    /**
     * Route calendar requests through the attached runtime host.
     */
    @Test
    fun testCalendarRequestsRouteThroughRuntimeHost() {
        val sessionHandle = HostSessionHandle(rawValue = 7)
        val runtimeApi = RuntimeIngressSpy()
        val bridge = RuntimeBridge(sessionHandle, runtimeApi)
        val calendarRequests = CalendarRequestRecorder().also {
            it.listResponse = RuntimeHostCalendarListResponse(
                status = 0,
                calendars = listOf(sampleCalendarDescriptor("calendar-1")),
            )
            it.eventListResponse = RuntimeHostCalendarEventListResponse(
                status = 0,
                events = listOf(sampleCalendarEvent("event-1")),
            )
            it.readResponse = RuntimeHostCalendarEventReadResponse(
                status = 0,
                event = sampleCalendarEvent("event-2"),
            )
            it.createResponse = RuntimeHostCalendarEventCreateResponse(
                status = 0,
                id = "event-3",
            )
            it.updateStatus = 0
            it.deleteStatus = 0
        }
        val runtimeHost = createBridgeRuntimeHost(
            sessionHandle = sessionHandle,
            permissionRequests = PermissionRequestRecorder(),
            documentRequests = DocumentRequestRecorder(),
            contactRequests = ContactRequestRecorder(),
            calendarRequests = calendarRequests,
            intentRequests = IntentRequestRecorder(),
        )

        bridge.attach(runtimeHost)

        val listResponse = bridge.calendarList()
        val eventListResponse = bridge.calendarEventList(
            RuntimeHostCalendarEventQuery(
                calendarIds = listOf("calendar-1"),
                startUnixNs = 1_700_000_000_000_000_000L,
                endUnixNs = 1_700_003_600_000_000_000L,
                limit = 25,
                includeCanceled = true,
                includeDeclined = false,
                includeRecurrenceInstances = true,
            ),
        )
        val readResponse = bridge.calendarEventRead("event-2")
        val createResponse = bridge.calendarEventCreate(sampleCalendarDraft("calendar-1"))
        val updateStatus = bridge.calendarEventUpdate(
            "event-3",
            sampleCalendarDraft("calendar-2"),
        )
        val deleteStatus = bridge.calendarEventDelete("event-3")

        assertEquals(0, listResponse.status)
        assertEquals(0, eventListResponse.status)
        assertEquals(0, readResponse.status)
        assertEquals("event-3", createResponse.id)
        assertEquals(0, updateStatus)
        assertEquals(0, deleteStatus)
        assertEquals(
            listOf(
                RuntimeHostCalendarEventQuery(
                    calendarIds = listOf("calendar-1"),
                    startUnixNs = 1_700_000_000_000_000_000L,
                    endUnixNs = 1_700_003_600_000_000_000L,
                    limit = 25,
                    includeCanceled = true,
                    includeDeclined = false,
                    includeRecurrenceInstances = true,
                ),
            ),
            calendarRequests.eventQueries,
        )
        assertEquals(listOf("event-2"), calendarRequests.readIdentifiers)
        assertEquals(listOf(sampleCalendarDraft("calendar-1")), calendarRequests.createDrafts)
        assertEquals(
            listOf("event-3" to sampleCalendarDraft("calendar-2")),
            calendarRequests.updateCalls,
        )
        assertEquals(listOf("event-3"), calendarRequests.deleteIdentifiers)
    }
}

/**
 * Build one representative calendar descriptor fixture.
 */
private fun sampleCalendarDescriptor(
    id: String,
): RuntimeHostCalendarDescriptor {
    return RuntimeHostCalendarDescriptor(
        id = id,
        title = "Work",
        source = "local",
        colorArgb = 0xFF336699.toInt(),
        primary = true,
        access = RuntimeHostCalendarAccess.Write,
    )
}

/**
 * Build one representative calendar event fixture.
 */
private fun sampleCalendarEvent(
    id: String,
): RuntimeHostCalendarEvent {
    return RuntimeHostCalendarEvent(
        id = id,
        calendarId = "calendar-1",
        title = "Design review",
        startUnixNs = 1_700_000_000_000_000_000L,
        endUnixNs = 1_700_000_900_000_000_000L,
        availability = RuntimeHostCalendarAvailability.Busy,
    )
}

/**
 * Build one representative calendar draft fixture.
 */
private fun sampleCalendarDraft(
    calendarId: String,
): RuntimeHostCalendarEventDraft {
    return RuntimeHostCalendarEventDraft(
        calendarId = calendarId,
        title = "Design review",
        location = "Studio",
        startUnixNs = 1_700_000_000_000_000_000L,
        endUnixNs = 1_700_000_900_000_000_000L,
        availability = RuntimeHostCalendarAvailability.Busy,
    )
}
