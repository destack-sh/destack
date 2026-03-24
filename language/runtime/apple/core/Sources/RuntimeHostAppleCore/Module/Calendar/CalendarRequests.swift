import Foundation

/// The Apple calendar request surface for one runtime host embedder.
@MainActor
public protocol CalendarRequests: AnyObject {
    /// List the readable host calendars.
    func listCalendars() -> (status: UInt32, calendars: [RuntimeHostCalendarDescriptor]?)

    /// List host calendar events for one query.
    func listCalendarEvents(
        _ query: RuntimeHostCalendarEventQuery
    ) -> (status: UInt32, events: [RuntimeHostCalendarEvent]?)

    /// Read one host calendar event by stable identifier.
    func readCalendarEvent(
        id: String
    ) -> (status: UInt32, event: RuntimeHostCalendarEvent?)

    /// Create one host calendar event and return its identifier.
    func createCalendarEvent(
        _ draft: RuntimeHostCalendarEventDraft
    ) -> (status: UInt32, id: String?)

    /// Update one existing host calendar event.
    func updateCalendarEvent(
        id: String,
        draft: RuntimeHostCalendarEventDraft
    ) -> UInt32

    /// Delete one existing host calendar event.
    func deleteCalendarEvent(
        id: String
    ) -> UInt32
}

/// The explicit unsupported calendar request surface for one Apple runtime host.
@MainActor
public final class UnsupportedCalendarRequests: CalendarRequests {
    /// Create one unsupported calendar request surface.
    public init() {}

    public func listCalendars() -> (status: UInt32, calendars: [RuntimeHostCalendarDescriptor]?) {
        (hostStatusNotSupported, nil)
    }

    public func listCalendarEvents(
        _ query: RuntimeHostCalendarEventQuery
    ) -> (status: UInt32, events: [RuntimeHostCalendarEvent]?) {
        let _ = query

        return (hostStatusNotSupported, nil)
    }

    public func readCalendarEvent(
        id: String
    ) -> (status: UInt32, event: RuntimeHostCalendarEvent?) {
        let _ = id

        return (hostStatusNotSupported, nil)
    }

    public func createCalendarEvent(
        _ draft: RuntimeHostCalendarEventDraft
    ) -> (status: UInt32, id: String?) {
        let _ = draft

        return (hostStatusNotSupported, nil)
    }

    public func updateCalendarEvent(
        id: String,
        draft: RuntimeHostCalendarEventDraft
    ) -> UInt32 {
        let _ = id
        let _ = draft

        return hostStatusNotSupported
    }

    public func deleteCalendarEvent(
        id: String
    ) -> UInt32 {
        let _ = id

        return hostStatusNotSupported
    }
}
