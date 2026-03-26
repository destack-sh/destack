import EventKit
import Foundation

/// The Apple calendar request surface backed by `EKEventStore`.
@MainActor
public final class CalendarStoreRequests: CalendarRequests {
  /// The underlying EventKit store.
  private let eventStore: EKEventStore

  /// Create one EventKit-backed calendar request surface.
  public init(
    eventStore: EKEventStore = EKEventStore()
  ) {
    self.eventStore = eventStore
  }

  public func listCalendars() -> (status: UInt32, calendars: [RuntimeHostCalendarDescriptor]?) {
    guard isCalendarAccessAuthorized() else {
      return (hostStatusPermissionDenied, nil)
    }

    let defaultCalendarIdentifier = eventStore.defaultCalendarForNewEvents?.calendarIdentifier
    let calendars = eventStore.calendars(for: .event).map { calendar in
      runtimeHostCalendarDescriptor(
        calendar,
        defaultCalendarIdentifier: defaultCalendarIdentifier
      )
    }

    return (hostStatusOk, calendars)
  }

  public func listCalendarEvents(
    _ query: RuntimeHostCalendarEventQuery
  ) -> (status: UInt32, events: [RuntimeHostCalendarEvent]?) {
    guard isCalendarAccessAuthorized() else {
      return (hostStatusPermissionDenied, nil)
    }

    guard query.endUnixNs >= query.startUnixNs else {
      return (hostStatusInvalidArgument, nil)
    }

    guard
      let calendars = resolveQueryCalendars(
        eventStore: eventStore,
        calendarIds: query.calendarIds
      )
    else {
      return (hostStatusNotFound, nil)
    }

    let startDate = dateFromUnixNs(query.startUnixNs)
    let endDate = dateFromUnixNs(query.endUnixNs)
    let predicate = eventStore.predicateForEvents(
      withStart: startDate,
      end: endDate,
      calendars: calendars
    )
    var events = eventStore.events(matching: predicate).map { event in
      runtimeHostCalendarEvent(event)
    }

    if !query.includeDeclined {
      events.removeAll { event in
        eventHasDeclinedParticipant(event)
      }
    }

    if !query.includeCanceled {
      events.removeAll { event in
        event.canceled
      }
    }

    if !query.includeRecurrenceInstances {
      events = collapseRecurrenceInstances(events)
    }

    events.sort { left, right in
      left.startUnixNs < right.startUnixNs
        || (left.startUnixNs == right.startUnixNs && left.id < right.id)
    }

    if let limit = query.limit {
      events = Array(events.prefix(Int(limit)))
    }

    return (hostStatusOk, events)
  }

  public func readCalendarEvent(
    id: String
  ) -> (status: UInt32, event: RuntimeHostCalendarEvent?) {
    guard isCalendarAccessAuthorized() else {
      return (hostStatusPermissionDenied, nil)
    }

    guard let event = eventStore.event(withIdentifier: id) else {
      return (hostStatusNotFound, nil)
    }

    return (hostStatusOk, runtimeHostCalendarEvent(event))
  }

  public func createCalendarEvent(
    _ draft: RuntimeHostCalendarEventDraft
  ) -> (status: UInt32, id: String?) {
    guard isCalendarAccessAuthorized() else {
      return (hostStatusPermissionDenied, nil)
    }

    let event = EKEvent(eventStore: eventStore)
    let applyStatus = applyEventDraft(
      draft,
      to: event,
      eventStore: eventStore,
      operation: .create
    )
    guard applyStatus == hostStatusOk else {
      return (applyStatus, nil)
    }

    do {
      try eventStore.save(
        event,
        span: .thisEvent
      )

      return (hostStatusOk, event.eventIdentifier)
    } catch {
      return (hostStatusFailed, nil)
    }
  }

  public func updateCalendarEvent(
    id: String,
    draft: RuntimeHostCalendarEventDraft
  ) -> UInt32 {
    guard isCalendarAccessAuthorized() else {
      return hostStatusPermissionDenied
    }

    guard let event = eventStore.event(withIdentifier: id) else {
      return hostStatusNotFound
    }

    let applyStatus = applyEventDraft(
      draft,
      to: event,
      eventStore: eventStore,
      operation: .update
    )
    guard applyStatus == hostStatusOk else {
      return applyStatus
    }

    do {
      try eventStore.save(
        event,
        span: .thisEvent
      )

      return hostStatusOk
    } catch {
      return hostStatusFailed
    }
  }

  public func deleteCalendarEvent(
    id: String
  ) -> UInt32 {
    guard isCalendarAccessAuthorized() else {
      return hostStatusPermissionDenied
    }

    guard let event = eventStore.event(withIdentifier: id) else {
      return hostStatusNotFound
    }

    do {
      try eventStore.remove(
        event,
        span: .thisEvent
      )

      return hostStatusOk
    } catch {
      return hostStatusFailed
    }
  }
}

/// The active calendar write operation.
private enum CalendarWriteOperation {
  /// One create operation.
  case create

  /// One update operation.
  case update
}

/// Return whether EventKit access is authorized for the current process.
@MainActor
private func isCalendarAccessAuthorized() -> Bool {
  return EKEventStore.authorizationStatus(for: .event) == .authorized
}

/// Resolve one optional calendar filter list from one identifier set.
@MainActor
private func resolveQueryCalendars(
  eventStore: EKEventStore,
  calendarIds: [String]
) -> [EKCalendar]? {
  if calendarIds.isEmpty {
    return nil
  }

  var calendars: [EKCalendar] = []

  for id in calendarIds {
    guard let calendar = eventStore.calendar(withIdentifier: id) else {
      return nil
    }

    calendars.append(calendar)
  }

  return calendars
}

/// Convert one Unix timestamp in nanoseconds into one `Date`.
private func dateFromUnixNs(
  _ unixNs: UInt64
) -> Date {
  Date(timeIntervalSince1970: TimeInterval(unixNs) / 1_000_000_000.0)
}

/// Convert one `Date` into one Unix timestamp in nanoseconds.
private func unixNsFromDate(
  _ date: Date
) -> UInt64 {
  UInt64(max(0, (date.timeIntervalSince1970 * 1_000_000_000.0).rounded()))
}

/// Materialize one runtime calendar descriptor from one EventKit calendar.
private func runtimeHostCalendarDescriptor(
  _ calendar: EKCalendar,
  defaultCalendarIdentifier: String?
) -> RuntimeHostCalendarDescriptor {
  RuntimeHostCalendarDescriptor(
    id: calendar.calendarIdentifier,
    title: calendar.title,
    source: calendar.source.title,
    owner: calendar.source.sourceIdentifier,
    colorArgb: colorArgb(calendar.cgColor),
    primary: calendar.calendarIdentifier == defaultCalendarIdentifier,
    access: calendar.allowsContentModifications ? .write : .read
  )
}

/// Materialize one runtime calendar event from one EventKit event.
private func runtimeHostCalendarEvent(
  _ event: EKEvent
) -> RuntimeHostCalendarEvent {
  RuntimeHostCalendarEvent(
    id: event.eventIdentifier,
    calendarId: event.calendar.calendarIdentifier,
    title: event.title,
    notes: event.notes,
    location: event.location,
    startUnixNs: unixNsFromDate(event.startDate),
    endUnixNs: unixNsFromDate(event.endDate),
    allDay: event.isAllDay,
    canceled: event.status == .canceled,
    timeZone: event.timeZone?.identifier,
    availability: runtimeHostAvailability(event.availability),
    url: event.url?.absoluteString,
    organizerName: event.organizer?.name,
    organizerEmail: event.organizer?.url.absoluteString,
    recurring: event.hasRecurrenceRules,
    recurrenceMasterId: event.calendarItemExternalIdentifier == event.eventIdentifier
      ? nil : event.calendarItemExternalIdentifier,
    recurrenceIdUnixNs: event.occurrenceDate.map(unixNsFromDate),
    recurrenceRule: event.recurrenceRules?.first.map(runtimeHostRecurrenceRule),
    attendees: event.attendees?.map(runtimeHostAttendee),
    reminders: event.alarms?.map(runtimeHostReminder)
  )
}

/// Materialize one runtime attendee payload from one EventKit participant.
private func runtimeHostAttendee(
  _ attendee: EKParticipant
) -> RuntimeHostCalendarAttendee {
  let email = emailFromParticipantURL(attendee.url)

  return RuntimeHostCalendarAttendee(
    id: attendee.url.absoluteString,
    name: attendee.name,
    email: email,
    optional: attendee.participantRole == .optional,
    organizer: attendee.isCurrentUser,
    responseStatus: runtimeHostParticipantStatus(attendee.participantStatus)
  )
}

/// Materialize one runtime reminder payload from one EventKit alarm.
private func runtimeHostReminder(
  _ alarm: EKAlarm
) -> RuntimeHostCalendarReminder {
  if let absoluteDate = alarm.absoluteDate {
    return .absolute(
      RuntimeHostCalendarAbsoluteReminder(
        absoluteUnixNs: unixNsFromDate(absoluteDate)
      )
    )
  }

  let minutesBeforeStart = Int32((alarm.relativeOffset / 60.0).rounded())

  return .relative(
    RuntimeHostCalendarRelativeReminder(
      minutesBeforeStart: minutesBeforeStart
    )
  )
}

/// Materialize one runtime recurrence rule from one EventKit rule.
private func runtimeHostRecurrenceRule(
  _ rule: EKRecurrenceRule
) -> RuntimeHostCalendarRecurrenceRule {
  RuntimeHostCalendarRecurrenceRule(
    frequency: runtimeHostRecurrenceFrequency(rule.frequency),
    interval: UInt32(rule.interval),
    count: rule.recurrenceEnd.map { end in
      UInt32(end.occurrenceCount)
    },
    untilUnixNs: rule.recurrenceEnd?.endDate.map(unixNsFromDate),
    byWeekDays: rule.daysOfTheWeek?.compactMap { value in
      UInt8(value.dayOfTheWeek.rawValue)
    } ?? [],
    byWeekdayOrdinals: rule.daysOfTheWeek?.compactMap { value in
      return RuntimeHostCalendarRecurrenceWeekday(
        day: UInt8(value.dayOfTheWeek.rawValue),
        weekNumber: Int8(exactly: value.weekNumber)
      )
    } ?? [],
    byMonthDays: rule.daysOfTheMonth?.compactMap(Int8.init(exactly:)) ?? [],
    byMonths: rule.monthsOfTheYear?.compactMap(UInt8.init(exactly:)) ?? [],
    byYearDays: rule.daysOfTheYear?.compactMap(Int16.init(exactly:)) ?? [],
    byWeekNumbers: rule.weeksOfTheYear?.compactMap(Int8.init(exactly:)) ?? [],
    bySetPositions: rule.setPositions?.compactMap(Int16.init(exactly:)) ?? []
  )
}

/// Collapse one expanded recurrence result list into one representative event per series.
private func collapseRecurrenceInstances(
  _ events: [RuntimeHostCalendarEvent]
) -> [RuntimeHostCalendarEvent] {
  var seenSeries: Set<String> = []
  var results: [RuntimeHostCalendarEvent] = []

  for event in events {
    if !event.recurring {
      results.append(event)
      continue
    }

    let seriesKey = event.recurrenceMasterId ?? event.id
    if seenSeries.insert(seriesKey).inserted {
      results.append(event)
    }
  }

  return results
}

/// Return whether one runtime event has one declined participant.
private func eventHasDeclinedParticipant(
  _ event: RuntimeHostCalendarEvent
) -> Bool {
  guard let attendees = event.attendees else {
    return false
  }

  return attendees.contains { attendee in
    attendee.responseStatus == .declined
  }
}

/// Map one EventKit availability into one runtime availability.
private func runtimeHostAvailability(
  _ availability: EKEventAvailability
) -> RuntimeHostCalendarAvailability {
  switch availability {
  case .busy:
    return .busy
  case .free:
    return .free
  case .tentative:
    return .tentative
  case .unavailable:
    return .unavailable
  case .notSupported:
    return .unknown
  @unknown default:
    return .unknown
  }
}

/// Map one EventKit participant status into one runtime participant status.
private func runtimeHostParticipantStatus(
  _ status: EKParticipantStatus
) -> RuntimeHostCalendarParticipantStatus {
  switch status {
  case .pending:
    return .pending
  case .accepted:
    return .accepted
  case .tentative:
    return .tentative
  case .declined:
    return .declined
  case .delegated:
    return .delegated
  case .completed:
    return .completed
  case .inProcess:
    return .inProcess
  case .unknown:
    return .unknown
  @unknown default:
    return .unknown
  }
}

/// Map one EventKit recurrence frequency into one runtime recurrence frequency.
private func runtimeHostRecurrenceFrequency(
  _ frequency: EKRecurrenceFrequency
) -> RuntimeHostCalendarRecurrenceFrequency {
  switch frequency {
  case .daily:
    return .daily
  case .weekly:
    return .weekly
  case .monthly:
    return .monthly
  case .yearly:
    return .yearly
  @unknown default:
    return .daily
  }
}

/// Apply one runtime event draft to one EventKit event.
private func applyEventDraft(
  _ draft: RuntimeHostCalendarEventDraft,
  to event: EKEvent,
  eventStore: EKEventStore,
  operation: CalendarWriteOperation
) -> UInt32 {
  // validate the basic event range and target calendar
  guard draft.endUnixNs >= draft.startUnixNs else {
    return hostStatusInvalidArgument
  }

  guard !draft.calendarId.isEmpty else {
    return hostStatusInvalidArgument
  }

  guard let calendar = eventStore.calendar(withIdentifier: draft.calendarId) else {
    return hostStatusNotFound
  }

  // EventKit does not support attendee mutation through this path
  if draft.attendees != nil {
    return hostStatusNotSupported
  }

  // core event fields
  event.calendar = calendar
  event.title = draft.title
  event.notes = draft.notes
  event.location = draft.location
  event.startDate = dateFromUnixNs(draft.startUnixNs)
  event.endDate = dateFromUnixNs(draft.endUnixNs)
  event.isAllDay = draft.allDay
  event.timeZone = draft.timeZone.flatMap(TimeZone.init(identifier:))
  event.availability = eventKitAvailability(draft.availability)
  event.url = draft.url.flatMap(URL.init(string:))

  // recurrence
  if let rule = draft.recurrenceRule {
    event.recurrenceRules = [eventKitRecurrenceRule(rule)]
  } else {
    event.recurrenceRules = nil
  }

  // reminders
  if let reminders = draft.reminders {
    event.alarms = reminders.map(eventKitAlarm)
  } else if operation == .update {
    event.alarms = nil
  }

  return hostStatusOk
}

/// Decode one attendee email address from one EventKit participant URL.
private func emailFromParticipantURL(
  _ url: URL
) -> String? {
  let absoluteString = url.absoluteString
  let mailtoPrefix = "mailto:"

  if absoluteString.hasPrefix(mailtoPrefix) {
    let emailStartIndex = absoluteString.index(
      absoluteString.startIndex,
      offsetBy: mailtoPrefix.count
    )

    let email = String(absoluteString[emailStartIndex...])
    return email.isEmpty ? nil : email
  }

  return nil
}

/// Map one runtime availability into one EventKit availability.
private func eventKitAvailability(
  _ availability: RuntimeHostCalendarAvailability
) -> EKEventAvailability {
  switch availability {
  case .busy:
    return .busy
  case .free:
    return .free
  case .tentative:
    return .tentative
  case .outOfOffice:
    return .busy
  case .unavailable:
    return .unavailable
  case .unknown:
    return .notSupported
  }
}

/// Map one runtime reminder into one EventKit alarm.
private func eventKitAlarm(
  _ reminder: RuntimeHostCalendarReminder
) -> EKAlarm {
  switch reminder {
  case .absolute(let value):
    return EKAlarm(absoluteDate: dateFromUnixNs(value.absoluteUnixNs))
  case .relative(let value):
    return EKAlarm(relativeOffset: TimeInterval(value.minutesBeforeStart) * 60.0)
  }
}

/// Map one runtime recurrence rule into one EventKit recurrence rule.
private func eventKitRecurrenceRule(
  _ rule: RuntimeHostCalendarRecurrenceRule
) -> EKRecurrenceRule {
  let recurrenceEnd: EKRecurrenceEnd?
  if let count = rule.count {
    recurrenceEnd = EKRecurrenceEnd(occurrenceCount: Int(count))
  } else if let untilUnixNs = rule.untilUnixNs {
    recurrenceEnd = EKRecurrenceEnd(end: dateFromUnixNs(untilUnixNs))
  } else {
    recurrenceEnd = nil
  }

  let daysOfTheWeek = rule.byWeekdayOrdinals.compactMap { value in
    EKRecurrenceDayOfWeek(
      eventKitWeekday(value.day),
      weekNumber: value.weekNumber.map(Int.init) ?? 0
    )
  }
  let fallbackDaysOfWeek = rule.byWeekDays.compactMap { day in
    EKRecurrenceDayOfWeek(eventKitWeekday(day))
  }

  return EKRecurrenceRule(
    recurrenceWith: eventKitRecurrenceFrequency(rule.frequency),
    interval: Int(rule.interval),
    daysOfTheWeek: daysOfTheWeek.isEmpty
      ? (fallbackDaysOfWeek.isEmpty ? nil : fallbackDaysOfWeek) : daysOfTheWeek,
    daysOfTheMonth: rule.byMonthDays.isEmpty ? nil : rule.byMonthDays.map(NSNumber.init(value:)),
    monthsOfTheYear: rule.byMonths.isEmpty ? nil : rule.byMonths.map(NSNumber.init(value:)),
    weeksOfTheYear: rule.byWeekNumbers.isEmpty
      ? nil : rule.byWeekNumbers.map(NSNumber.init(value:)),
    daysOfTheYear: rule.byYearDays.isEmpty ? nil : rule.byYearDays.map(NSNumber.init(value:)),
    setPositions: rule.bySetPositions.isEmpty
      ? nil : rule.bySetPositions.map(NSNumber.init(value:)),
    end: recurrenceEnd
  )
}

/// Map one runtime weekday number into one EventKit weekday.
private func eventKitWeekday(
  _ day: UInt8
) -> EKWeekday {
  switch day {
  case 1:
    return .monday
  case 2:
    return .tuesday
  case 3:
    return .wednesday
  case 4:
    return .thursday
  case 5:
    return .friday
  case 6:
    return .saturday
  case 7:
    return .sunday
  default:
    return .monday
  }
}

/// Map one runtime recurrence frequency into one EventKit recurrence frequency.
private func eventKitRecurrenceFrequency(
  _ frequency: RuntimeHostCalendarRecurrenceFrequency
) -> EKRecurrenceFrequency {
  switch frequency {
  case .daily:
    return .daily
  case .weekly:
    return .weekly
  case .monthly:
    return .monthly
  case .yearly:
    return .yearly
  }
}

/// Convert one native calendar color into one ARGB integer.
private func colorArgb(
  _ color: CGColor
) -> UInt32 {
  guard let components = color.components else {
    return 0
  }

  let red = UInt32((components[safe: 0] ?? 0) * 255.0)
  let green = UInt32((components[safe: 1] ?? components[safe: 0] ?? 0) * 255.0)
  let blue = UInt32(
    (components[safe: 2] ?? components[safe: 1] ?? components[safe: 0] ?? 0) * 255.0)
  let alpha = UInt32((components[safe: 3] ?? 1.0) * 255.0)

  return (alpha << 24) | (red << 16) | (green << 8) | blue
}

extension Array {
  fileprivate subscript(safe index: Int) -> Element? {
    guard indices.contains(index) else {
      return nil
    }

    return self[index]
  }
}
