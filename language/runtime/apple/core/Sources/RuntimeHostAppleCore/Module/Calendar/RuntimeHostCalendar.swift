import Foundation

/// The calendar access level for one host calendar.
public enum RuntimeHostCalendarAccess: UInt32, Sendable, Hashable, Codable {
  /// Read-only access.
  case read = 1
  /// Read and write access.
  case write = 2
}

/// The availability class for one host calendar event.
public enum RuntimeHostCalendarAvailability: UInt32, Sendable, Hashable, Codable {
  /// Busy slot.
  case busy = 1
  /// Free slot.
  case free = 2
  /// Tentative slot.
  case tentative = 3
  /// Out-of-office slot.
  case outOfOffice = 4
  /// Unavailable slot.
  case unavailable = 5
  /// Unknown or host-specific availability.
  case unknown = 6
}

/// The response status for one calendar attendee.
public enum RuntimeHostCalendarParticipantStatus: UInt32, Sendable, Hashable, Codable {
  /// The status is unknown.
  case unknown = 1
  /// The attendee has not responded.
  case pending = 2
  /// The attendee accepted the invitation.
  case accepted = 3
  /// The attendee tentatively accepted the invitation.
  case tentative = 4
  /// The attendee declined the invitation.
  case declined = 5
  /// The attendee delegated the invitation.
  case delegated = 6
  /// The attendee completed the request.
  case completed = 7
  /// The attendee is processing the request.
  case inProcess = 8
}

/// The frequency for one calendar recurrence rule.
public enum RuntimeHostCalendarRecurrenceFrequency: UInt32, Sendable, Hashable, Codable {
  /// Daily recurrence.
  case daily = 1
  /// Weekly recurrence.
  case weekly = 2
  /// Monthly recurrence.
  case monthly = 3
  /// Yearly recurrence.
  case yearly = 4
}

/// One calendar attendee payload.
public struct RuntimeHostCalendarAttendee: Sendable, Hashable, Codable {
  /// The stable attendee identifier when available.
  public let id: String?
  /// The attendee display name when available.
  public let name: String?
  /// The attendee email address when available.
  public let email: String?
  /// Whether this attendee is optional.
  public let optional: Bool
  /// Whether this attendee is the organizer.
  public let organizer: Bool
  /// The attendee response status.
  public let responseStatus: RuntimeHostCalendarParticipantStatus

  /// Create one calendar attendee payload.
  public init(
    id: String? = nil,
    name: String? = nil,
    email: String? = nil,
    optional: Bool = false,
    organizer: Bool = false,
    responseStatus: RuntimeHostCalendarParticipantStatus = .unknown
  ) {
    self.id = id
    self.name = name
    self.email = email
    self.optional = optional
    self.organizer = organizer
    self.responseStatus = responseStatus
  }
}

/// One recurrence weekday selector payload.
public struct RuntimeHostCalendarRecurrenceWeekday: Sendable, Hashable, Codable {
  /// The ISO-8601 weekday number, 1 through 7.
  public let day: UInt8
  /// The optional ordinal week number.
  public let weekNumber: Int8?

  /// Create one recurrence weekday payload.
  public init(
    day: UInt8,
    weekNumber: Int8? = nil
  ) {
    self.day = day
    self.weekNumber = weekNumber
  }
}

/// One absolute calendar reminder payload.
public struct RuntimeHostCalendarAbsoluteReminder: Sendable, Hashable, Codable {
  /// The absolute reminder timestamp in UTC nanoseconds.
  public let absoluteUnixNs: UInt64

  /// Create one absolute reminder payload.
  public init(
    absoluteUnixNs: UInt64
  ) {
    self.absoluteUnixNs = absoluteUnixNs
  }
}

/// One relative calendar reminder payload.
public struct RuntimeHostCalendarRelativeReminder: Sendable, Hashable, Codable {
  /// The offset in minutes before the event start.
  public let minutesBeforeStart: Int32

  /// Create one relative reminder payload.
  public init(
    minutesBeforeStart: Int32
  ) {
    self.minutesBeforeStart = minutesBeforeStart
  }
}

/// One calendar reminder payload.
public enum RuntimeHostCalendarReminder: Sendable, Hashable, Codable {
  /// One absolute reminder.
  case absolute(RuntimeHostCalendarAbsoluteReminder)
  /// One relative reminder.
  case relative(RuntimeHostCalendarRelativeReminder)

  /// Create one calendar reminder payload from one bridge shape.
  public init(
    kind: RuntimeHostCalendarReminderKind,
    absoluteUnixNs: UInt64 = 0,
    minutesBeforeStart: Int32 = 0
  ) {
    switch kind {
    case .absolute:
      self = .absolute(
        RuntimeHostCalendarAbsoluteReminder(
          absoluteUnixNs: absoluteUnixNs
        )
      )
    case .relative:
      self = .relative(
        RuntimeHostCalendarRelativeReminder(
          minutesBeforeStart: minutesBeforeStart
        )
      )
    }
  }

  /// The bridge-facing reminder kind.
  public var kind: RuntimeHostCalendarReminderKind {
    switch self {
    case .absolute:
      return .absolute
    case .relative:
      return .relative
    }
  }

  /// The bridge-facing absolute timestamp.
  public var absoluteUnixNs: UInt64 {
    switch self {
    case .absolute(let value):
      return value.absoluteUnixNs
    case .relative:
      return 0
    }
  }

  /// The bridge-facing relative offset.
  public var minutesBeforeStart: Int32 {
    switch self {
    case .absolute:
      return 0
    case .relative(let value):
      return value.minutesBeforeStart
    }
  }

  /// Create one calendar reminder payload from one bridge shape.
  public init(
    kind: RuntimeHostCalendarReminderKind,
    absoluteUnixNs: UInt64 = 0,
    minutesBeforeStart: Int = 0
  ) {
    self.init(
      kind: kind,
      absoluteUnixNs: absoluteUnixNs,
      minutesBeforeStart: Int32(minutesBeforeStart)
    )
  }
}

/// The calendar reminder kind for one bridge payload.
public enum RuntimeHostCalendarReminderKind: UInt32, Sendable, Hashable, Codable {
  /// One absolute reminder.
  case absolute = 1
  /// One relative reminder.
  case relative = 2
}

/// One recurrence rule payload.
public struct RuntimeHostCalendarRecurrenceRule: Sendable, Hashable, Codable {
  /// The recurrence frequency.
  public let frequency: RuntimeHostCalendarRecurrenceFrequency
  /// The recurrence interval.
  public let interval: UInt32
  /// The optional occurrence limit.
  public let count: UInt32?
  /// The optional end timestamp in UTC nanoseconds.
  public let untilUnixNs: UInt64?
  /// The weekday numbers in ISO-8601 encoding.
  public let byWeekDays: [UInt8]
  /// The structured weekday selectors with optional ordinals.
  public let byWeekdayOrdinals: [RuntimeHostCalendarRecurrenceWeekday]
  /// The day-of-month set.
  public let byMonthDays: [Int8]
  /// The month set.
  public let byMonths: [UInt8]
  /// The day-of-year set.
  public let byYearDays: [Int16]
  /// The week-of-year set.
  public let byWeekNumbers: [Int8]
  /// The set-position filters.
  public let bySetPositions: [Int16]

  /// Create one recurrence rule payload.
  public init(
    frequency: RuntimeHostCalendarRecurrenceFrequency,
    interval: UInt32,
    count: UInt32? = nil,
    untilUnixNs: UInt64? = nil,
    byWeekDays: [UInt8] = [],
    byWeekdayOrdinals: [RuntimeHostCalendarRecurrenceWeekday] = [],
    byMonthDays: [Int8] = [],
    byMonths: [UInt8] = [],
    byYearDays: [Int16] = [],
    byWeekNumbers: [Int8] = [],
    bySetPositions: [Int16] = []
  ) {
    self.frequency = frequency
    self.interval = interval
    self.count = count
    self.untilUnixNs = untilUnixNs
    self.byWeekDays = byWeekDays
    self.byWeekdayOrdinals = byWeekdayOrdinals
    self.byMonthDays = byMonthDays
    self.byMonths = byMonths
    self.byYearDays = byYearDays
    self.byWeekNumbers = byWeekNumbers
    self.bySetPositions = bySetPositions
  }
}

/// One calendar descriptor payload.
public struct RuntimeHostCalendarDescriptor: Sendable, Hashable, Codable {
  /// The stable calendar identifier.
  public let id: String
  /// The host-visible calendar title.
  public let title: String
  /// The host-visible source or account label.
  public let source: String
  /// The optional owner account label.
  public let owner: String?
  /// The ARGB color value.
  public let colorArgb: UInt32
  /// Whether this is the primary write target.
  public let primary: Bool
  /// The calendar access level.
  public let access: RuntimeHostCalendarAccess

  /// Create one calendar descriptor payload.
  public init(
    id: String,
    title: String,
    source: String,
    owner: String? = nil,
    colorArgb: UInt32 = 0,
    primary: Bool = false,
    access: RuntimeHostCalendarAccess = .read
  ) {
    self.id = id
    self.title = title
    self.source = source
    self.owner = owner
    self.colorArgb = colorArgb
    self.primary = primary
    self.access = access
  }
}

/// One calendar event payload.
public struct RuntimeHostCalendarEvent: Sendable, Hashable, Codable {
  /// The stable event identifier.
  public let id: String
  /// The calendar identifier.
  public let calendarId: String
  /// The event title.
  public let title: String
  /// The optional event notes.
  public let notes: String?
  /// The optional location text.
  public let location: String?
  /// The start timestamp in UTC nanoseconds.
  public let startUnixNs: UInt64
  /// The end timestamp in UTC nanoseconds.
  public let endUnixNs: UInt64
  /// Whether the event is all-day.
  public let allDay: Bool
  /// Whether the event is canceled.
  public let canceled: Bool
  /// The optional timezone identifier.
  public let timeZone: String?
  /// The availability class.
  public let availability: RuntimeHostCalendarAvailability
  /// The optional event URL.
  public let url: String?
  /// The optional organizer display name.
  public let organizerName: String?
  /// The optional organizer email address.
  public let organizerEmail: String?
  /// Whether this event is recurring.
  public let recurring: Bool
  /// The optional recurrence master identifier.
  public let recurrenceMasterId: String?
  /// The optional recurrence instance identifier timestamp.
  public let recurrenceIdUnixNs: UInt64?
  /// The optional recurrence rule.
  public let recurrenceRule: RuntimeHostCalendarRecurrenceRule?
  /// The optional attendees.
  public let attendees: [RuntimeHostCalendarAttendee]?
  /// The optional reminders.
  public let reminders: [RuntimeHostCalendarReminder]?

  /// Create one calendar event payload.
  public init(
    id: String,
    calendarId: String,
    title: String,
    notes: String? = nil,
    location: String? = nil,
    startUnixNs: UInt64,
    endUnixNs: UInt64,
    allDay: Bool = false,
    canceled: Bool = false,
    timeZone: String? = nil,
    availability: RuntimeHostCalendarAvailability = .unknown,
    url: String? = nil,
    organizerName: String? = nil,
    organizerEmail: String? = nil,
    recurring: Bool = false,
    recurrenceMasterId: String? = nil,
    recurrenceIdUnixNs: UInt64? = nil,
    recurrenceRule: RuntimeHostCalendarRecurrenceRule? = nil,
    attendees: [RuntimeHostCalendarAttendee]? = nil,
    reminders: [RuntimeHostCalendarReminder]? = nil
  ) {
    self.id = id
    self.calendarId = calendarId
    self.title = title
    self.notes = notes
    self.location = location
    self.startUnixNs = startUnixNs
    self.endUnixNs = endUnixNs
    self.allDay = allDay
    self.canceled = canceled
    self.timeZone = timeZone
    self.availability = availability
    self.url = url
    self.organizerName = organizerName
    self.organizerEmail = organizerEmail
    self.recurring = recurring
    self.recurrenceMasterId = recurrenceMasterId
    self.recurrenceIdUnixNs = recurrenceIdUnixNs
    self.recurrenceRule = recurrenceRule
    self.attendees = attendees
    self.reminders = reminders
  }

  /// The bridge-facing calendar identifier.
  public var calendarID: String {
    calendarId
  }

  /// The bridge-facing recurrence master identifier.
  public var recurrenceMasterID: String? {
    recurrenceMasterId
  }

  /// The bridge-facing recurrence instance timestamp.
  public var recurrenceIDUnixNs: UInt64? {
    recurrenceIdUnixNs
  }

  /// Create one calendar event payload from one bridge shape.
  public init(
    id: String,
    calendarID: String,
    title: String,
    notes: String? = nil,
    location: String? = nil,
    startUnixNs: UInt64,
    endUnixNs: UInt64,
    allDay: Bool = false,
    canceled: Bool = false,
    timeZone: String? = nil,
    availability: RuntimeHostCalendarAvailability = .unknown,
    url: String? = nil,
    organizerName: String? = nil,
    organizerEmail: String? = nil,
    recurring: Bool = false,
    recurrenceMasterID: String? = nil,
    recurrenceIDUnixNs: UInt64? = nil,
    recurrenceRule: RuntimeHostCalendarRecurrenceRule? = nil,
    attendees: [RuntimeHostCalendarAttendee]? = nil,
    reminders: [RuntimeHostCalendarReminder]? = nil
  ) {
    self.init(
      id: id,
      calendarId: calendarID,
      title: title,
      notes: notes,
      location: location,
      startUnixNs: startUnixNs,
      endUnixNs: endUnixNs,
      allDay: allDay,
      canceled: canceled,
      timeZone: timeZone,
      availability: availability,
      url: url,
      organizerName: organizerName,
      organizerEmail: organizerEmail,
      recurring: recurring,
      recurrenceMasterId: recurrenceMasterID,
      recurrenceIdUnixNs: recurrenceIDUnixNs,
      recurrenceRule: recurrenceRule,
      attendees: attendees,
      reminders: reminders
    )
  }
}

/// One calendar event draft payload.
public struct RuntimeHostCalendarEventDraft: Sendable, Hashable, Codable {
  /// The calendar identifier.
  public let calendarId: String
  /// The event title.
  public let title: String
  /// The optional event notes.
  public let notes: String?
  /// The optional location text.
  public let location: String?
  /// The start timestamp in UTC nanoseconds.
  public let startUnixNs: UInt64
  /// The end timestamp in UTC nanoseconds.
  public let endUnixNs: UInt64
  /// Whether the event is all-day.
  public let allDay: Bool
  /// The optional timezone identifier.
  public let timeZone: String?
  /// The availability class.
  public let availability: RuntimeHostCalendarAvailability
  /// The optional event URL.
  public let url: String?
  /// The optional recurrence rule.
  public let recurrenceRule: RuntimeHostCalendarRecurrenceRule?
  /// The optional attendees.
  public let attendees: [RuntimeHostCalendarAttendee]?
  /// The optional reminders.
  public let reminders: [RuntimeHostCalendarReminder]?

  /// Create one calendar event draft payload.
  public init(
    calendarId: String,
    title: String,
    notes: String? = nil,
    location: String? = nil,
    startUnixNs: UInt64,
    endUnixNs: UInt64,
    allDay: Bool = false,
    timeZone: String? = nil,
    availability: RuntimeHostCalendarAvailability = .unknown,
    url: String? = nil,
    recurrenceRule: RuntimeHostCalendarRecurrenceRule? = nil,
    attendees: [RuntimeHostCalendarAttendee]? = nil,
    reminders: [RuntimeHostCalendarReminder]? = nil
  ) {
    self.calendarId = calendarId
    self.title = title
    self.notes = notes
    self.location = location
    self.startUnixNs = startUnixNs
    self.endUnixNs = endUnixNs
    self.allDay = allDay
    self.timeZone = timeZone
    self.availability = availability
    self.url = url
    self.recurrenceRule = recurrenceRule
    self.attendees = attendees
    self.reminders = reminders
  }

  /// The bridge-facing calendar identifier.
  public var calendarID: String {
    calendarId
  }

  /// Create one calendar event draft payload from one bridge shape.
  public init(
    calendarID: String,
    title: String,
    notes: String? = nil,
    location: String? = nil,
    startUnixNs: UInt64,
    endUnixNs: UInt64,
    allDay: Bool = false,
    timeZone: String? = nil,
    availability: RuntimeHostCalendarAvailability = .unknown,
    url: String? = nil,
    recurrenceRule: RuntimeHostCalendarRecurrenceRule? = nil,
    attendees: [RuntimeHostCalendarAttendee]? = nil,
    reminders: [RuntimeHostCalendarReminder]? = nil
  ) {
    self.init(
      calendarId: calendarID,
      title: title,
      notes: notes,
      location: location,
      startUnixNs: startUnixNs,
      endUnixNs: endUnixNs,
      allDay: allDay,
      timeZone: timeZone,
      availability: availability,
      url: url,
      recurrenceRule: recurrenceRule,
      attendees: attendees,
      reminders: reminders
    )
  }
}

/// One calendar-list response.
public struct RuntimeHostCalendarListResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The listed calendars.
  public let calendars: [RuntimeHostCalendarDescriptor]

  /// Create one calendar-list response.
  public init(
    status: UInt32,
    calendars: [RuntimeHostCalendarDescriptor] = []
  ) {
    self.status = status
    self.calendars = calendars
  }
}

/// One calendar event-list response.
public struct RuntimeHostCalendarEventListResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The listed events.
  public let events: [RuntimeHostCalendarEvent]

  /// Create one calendar event-list response.
  public init(
    status: UInt32,
    events: [RuntimeHostCalendarEvent] = []
  ) {
    self.status = status
    self.events = events
  }
}

/// One calendar event-read response.
public struct RuntimeHostCalendarEventReadResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The returned event when available.
  public let event: RuntimeHostCalendarEvent?

  /// Create one calendar event-read response.
  public init(
    status: UInt32,
    event: RuntimeHostCalendarEvent? = nil
  ) {
    self.status = status
    self.event = event
  }
}

/// One calendar event-create response.
public struct RuntimeHostCalendarEventCreateResponse: Sendable, Hashable, Codable {
  /// The host status code.
  public let status: UInt32
  /// The created event identifier when available.
  public let id: String?

  /// Create one calendar event-create response.
  public init(
    status: UInt32,
    id: String? = nil
  ) {
    self.status = status
    self.id = id
  }
}

/// One calendar event query payload.
public struct RuntimeHostCalendarEventQuery: Sendable, Hashable, Codable {
  /// The selected calendar identifiers, empty means all readable calendars.
  public let calendarIds: [String]
  /// The query start timestamp in UTC nanoseconds.
  public let startUnixNs: UInt64
  /// The query end timestamp in UTC nanoseconds.
  public let endUnixNs: UInt64
  /// The optional page limit.
  public let limit: UInt32?
  /// Whether canceled events should be included.
  public let includeCanceled: Bool
  /// Whether declined events should be included.
  public let includeDeclined: Bool
  /// Whether expanded recurrence instances should be included.
  public let includeRecurrenceInstances: Bool

  /// Create one calendar event query payload.
  public init(
    calendarIds: [String] = [],
    startUnixNs: UInt64,
    endUnixNs: UInt64,
    limit: UInt32? = nil,
    includeCanceled: Bool = false,
    includeDeclined: Bool = false,
    includeRecurrenceInstances: Bool = false
  ) {
    self.calendarIds = calendarIds
    self.startUnixNs = startUnixNs
    self.endUnixNs = endUnixNs
    self.limit = limit
    self.includeCanceled = includeCanceled
    self.includeDeclined = includeDeclined
    self.includeRecurrenceInstances = includeRecurrenceInstances
  }
}
