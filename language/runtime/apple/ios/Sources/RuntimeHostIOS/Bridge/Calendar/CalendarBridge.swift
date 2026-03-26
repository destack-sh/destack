import Foundation
import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

let calendarListCallback: CalendarListCallback = {
  sessionHandle,
  outputCalendars in
  handleCalendarList(
    sessionHandle: sessionHandle,
    outputCalendars: outputCalendars
  )
}

let calendarEventListCallback: CalendarEventListCallback = {
  sessionHandle,
  query,
  outputEvents in
  handleCalendarEventList(
    sessionHandle: sessionHandle,
    query: query,
    outputEvents: outputEvents
  )
}

let calendarEventReadCallback: CalendarEventReadCallback = {
  sessionHandle,
  identifier,
  outputEvent in
  handleCalendarEventRead(
    sessionHandle: sessionHandle,
    identifier: identifier,
    outputEvent: outputEvent
  )
}

let calendarEventCreateCallback: CalendarEventCreateCallback = {
  sessionHandle,
  draft,
  outputIdentifier in
  handleCalendarEventCreate(
    sessionHandle: sessionHandle,
    draft: draft,
    outputIdentifier: outputIdentifier
  )
}

let calendarEventUpdateCallback: CalendarEventUpdateCallback = {
  sessionHandle,
  identifier,
  draft in
  handleCalendarEventUpdate(
    sessionHandle: sessionHandle,
    identifier: identifier,
    draft: draft
  )
}

let calendarEventDeleteCallback: CalendarEventDeleteCallback = {
  sessionHandle,
  identifier in
  handleCalendarEventDelete(
    sessionHandle: sessionHandle,
    identifier: identifier
  )
}

/// One temporary native allocation arena for one calendar bridge callback.
private final class CalendarBridgeArena {
  /// The raw deallocation actions recorded for this callback.
  private var deallocations: [() -> Void] = []

  /// Remove every recorded allocation before one new callback payload is encoded.
  func reset() {
    for deallocate in deallocations.reversed() {
      deallocate()
    }

    deallocations.removeAll(keepingCapacity: true)
  }

  /// Release every recorded native allocation.
  deinit {
    for deallocate in deallocations.reversed() {
      deallocate()
    }
  }

  /// Allocate one copied UTF-8 buffer for one Swift string.
  func makeStringRef(
    _ value: String
  ) -> DestackRustStringRef {
    let bytes = Array(value.utf8)
    if bytes.isEmpty {
      return DestackRustStringRef(data: nil, len: 0)
    }

    let storage = UnsafeMutablePointer<UInt8>.allocate(capacity: bytes.count)
    storage.initialize(from: bytes, count: bytes.count)
    deallocations.append {
      storage.deinitialize(count: bytes.count)
      storage.deallocate()
    }

    return DestackRustStringRef(
      data: UnsafePointer(storage),
      len: UInt32(bytes.count)
    )
  }

  /// Allocate one copied native array and fill it with one builder closure.
  func makeArray<Element>(
    count: Int,
    fill: (UnsafeMutableBufferPointer<Element>) -> Void
  ) -> UnsafePointer<Element>? {
    if count == 0 {
      return nil
    }

    let storage = UnsafeMutablePointer<Element>.allocate(capacity: count)
    let buffer = UnsafeMutableBufferPointer(start: storage, count: count)
    fill(buffer)
    deallocations.append {
      storage.deinitialize(count: count)
      storage.deallocate()
    }

    return UnsafePointer(storage)
  }
}

/// Return the thread-local calendar bridge arena for one callback thread.
private func currentCalendarBridgeArena() -> CalendarBridgeArena {
  let dictionary = Thread.current.threadDictionary
  let key = "dev.destack.runtime.apple.calendar-bridge-arena"

  if let arena = dictionary[key] as? CalendarBridgeArena {
    arena.reset()

    return arena
  }

  let arena = CalendarBridgeArena()
  dictionary[key] = arena

  return arena
}

/// One calendar bridge lane for one attached iOS runtime host.
@MainActor
final class CalendarBridge {
  /// List calendars through the attached runtime host.
  func listCalendars(
    runtimeHost: RuntimeHost
  ) -> (status: UInt32, calendars: [RuntimeHostCalendarDescriptor]?) {
    runtimeHost.calendarRequests.listCalendars()
  }

  /// List calendar events through the attached runtime host.
  func listCalendarEvents(
    runtimeHost: RuntimeHost,
    _ query: RuntimeHostCalendarEventQuery
  ) -> (status: UInt32, events: [RuntimeHostCalendarEvent]?) {
    runtimeHost.calendarRequests.listCalendarEvents(query)
  }

  /// Read one calendar event through the attached runtime host.
  func readCalendarEvent(
    runtimeHost: RuntimeHost,
    id: String
  ) -> (status: UInt32, event: RuntimeHostCalendarEvent?) {
    runtimeHost.calendarRequests.readCalendarEvent(id: id)
  }

  /// Create one calendar event through the attached runtime host.
  func createCalendarEvent(
    runtimeHost: RuntimeHost,
    _ draft: RuntimeHostCalendarEventDraft
  ) -> (status: UInt32, id: String?) {
    runtimeHost.calendarRequests.createCalendarEvent(draft)
  }

  /// Update one calendar event through the attached runtime host.
  func updateCalendarEvent(
    runtimeHost: RuntimeHost,
    id: String,
    draft: RuntimeHostCalendarEventDraft
  ) -> UInt32 {
    runtimeHost.calendarRequests.updateCalendarEvent(id: id, draft: draft)
  }

  /// Delete one calendar event through the attached runtime host.
  func deleteCalendarEvent(
    runtimeHost: RuntimeHost,
    id: String
  ) -> UInt32 {
    runtimeHost.calendarRequests.deleteCalendarEvent(id: id)
  }
}

/// Resolve one registered calendar bridge for one runtime session.
func guardCalendarBridge(
  sessionHandle: UInt64
) -> CalendarBridge? {
  RuntimeBridgeRegistry.resolve(sessionHandle: sessionHandle)?.calendarBridge
}

/// Handle one runtime callback asking to list calendars.
private func handleCalendarList(
  sessionHandle: UInt64,
  outputCalendars: UnsafeMutablePointer<DestackRustCalendarDescriptorSlice>?
) -> UInt32 {
  guard let bridge = guardCalendarBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let outputCalendars else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.listCalendars(runtimeHost: runtimeHost)
  }

  guard let calendars = response.calendars else {
    return response.status
  }

  return withEncodedCalendarDescriptors(calendars) { encodedCalendars in
    outputCalendars.pointee = encodedCalendars

    return response.status
  }
}

/// Handle one runtime callback asking to list calendar events.
private func handleCalendarEventList(
  sessionHandle: UInt64,
  query: DestackRustCalendarQuery,
  outputEvents: UnsafeMutablePointer<DestackRustCalendarEventSlice>?
) -> UInt32 {
  guard let bridge = guardCalendarBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let outputEvents,
    let query = decodeCalendarQuery(query)
  else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.listCalendarEvents(
      runtimeHost: runtimeHost,
      query
    )
  }

  guard let events = response.events else {
    return response.status
  }

  return withEncodedCalendarEvents(events) { encodedEvents in
    outputEvents.pointee = encodedEvents

    return response.status
  }
}

/// Handle one runtime callback asking to read one calendar event.
private func handleCalendarEventRead(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef,
  outputEvent: UnsafeMutablePointer<DestackRustCalendarEvent>?
) -> UInt32 {
  guard let bridge = guardCalendarBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let outputEvent,
    let identifier = decodeCalendarString(identifier)
  else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.readCalendarEvent(
      runtimeHost: runtimeHost,
      id: identifier
    )
  }

  guard let event = response.event else {
    return response.status
  }

  return withEncodedCalendarEvent(event) { encodedEvent in
    outputEvent.pointee = encodedEvent

    return response.status
  }
}

/// Handle one runtime callback asking to create one calendar event.
private func handleCalendarEventCreate(
  sessionHandle: UInt64,
  draft: DestackRustCalendarEventDraft,
  outputIdentifier: UnsafeMutablePointer<DestackRustStringRef>?
) -> UInt32 {
  guard let bridge = guardCalendarBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let outputIdentifier,
    let draft = decodeCalendarEventDraft(draft)
  else {
    return hostStatusInvalidArgument
  }

  let response = runOnMainThread {
    bridge.createCalendarEvent(
      runtimeHost: runtimeHost,
      draft
    )
  }

  guard let identifier = response.id else {
    return response.status
  }

  let arena = currentCalendarBridgeArena()
  outputIdentifier.pointee = arena.makeStringRef(identifier)

  return response.status
}

/// Handle one runtime callback asking to update one calendar event.
private func handleCalendarEventUpdate(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef,
  draft: DestackRustCalendarEventDraft
) -> UInt32 {
  guard let bridge = guardCalendarBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard
    let identifier = decodeCalendarString(identifier),
    let draft = decodeCalendarEventDraft(draft)
  else {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.updateCalendarEvent(
      runtimeHost: runtimeHost,
      id: identifier,
      draft: draft
    )
  }
}

/// Handle one runtime callback asking to delete one calendar event.
private func handleCalendarEventDelete(
  sessionHandle: UInt64,
  identifier: DestackRustStringRef
) -> UInt32 {
  guard let bridge = guardCalendarBridge(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }
  guard let runtimeHost = resolveRuntimeHost(sessionHandle: sessionHandle) else {
    return hostStatusNotFound
  }

  guard let identifier = decodeCalendarString(identifier) else {
    return hostStatusInvalidArgument
  }

  return runOnMainThread {
    bridge.deleteCalendarEvent(
      runtimeHost: runtimeHost,
      id: identifier
    )
  }
}

/// Decode one native calendar string into one Swift string.
private func decodeCalendarString(
  _ value: DestackRustStringRef
) -> String? {
  guard let data = value.data else {
    return value.len == 0 ? "" : nil
  }

  let bytes = UnsafeBufferPointer(start: data, count: Int(value.len))

  return String(bytes: bytes, encoding: .utf8)
}

/// Decode one optional native calendar string into one Swift string.
private func decodeOptionalCalendarString(
  _ value: DestackRustOptionalStringRef
) -> String? {
  if !value.has_value {
    return nil
  }

  return decodeCalendarString(value.value)
}

/// Decode one calendar string slice into one Swift array.
private func decodeCalendarStringSlice(
  _ values: DestackRustStringSlice
) -> [String]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))
  var strings: [String] = []
  strings.reserveCapacity(buffer.count)

  for value in buffer {
    guard let string = decodeCalendarString(value) else {
      return nil
    }

    strings.append(string)
  }

  return strings
}

/// Decode one optional `u32` into one Swift optional.
private func decodeOptionalCalendarU32(
  _ value: DestackRustOptionalU32
) -> UInt32? {
  value.has_value ? value.value : nil
}

/// Decode one optional `u64` into one Swift optional.
private func decodeOptionalCalendarU64(
  _ value: DestackRustOptionalU64
) -> UInt64? {
  value.has_value ? value.value : nil
}

/// Decode one optional `i8` into one Swift optional.
private func decodeOptionalCalendarI8(
  _ value: DestackRustOptionalI8
) -> Int8? {
  value.has_value ? value.value : nil
}

/// Decode one `u8` slice into one Swift array.
private func decodeCalendarU8Slice(
  _ values: DestackRustU8Slice
) -> [UInt8]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))
  return Array(buffer)
}

/// Decode one `i8` slice into one Swift array.
private func decodeCalendarI8Slice(
  _ values: DestackRustI8Slice
) -> [Int8]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))
  return Array(buffer)
}

/// Decode one `i16` slice into one Swift array.
private func decodeCalendarI16Slice(
  _ values: DestackRustI16Slice
) -> [Int16]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))
  return Array(buffer)
}

/// Decode one native recurrence weekday slice into one Swift array.
private func decodeCalendarRecurrenceWeekdays(
  _ values: DestackRustCalendarRecurrenceWeekdaySlice
) -> [RuntimeHostCalendarRecurrenceWeekday]? {
  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))
  return buffer.map { value in
    RuntimeHostCalendarRecurrenceWeekday(
      day: value.day,
      weekNumber: decodeOptionalCalendarI8(value.week_number)
    )
  }
}

/// Decode one native recurrence rule into one Swift value.
private func decodeCalendarRecurrenceRule(
  _ value: DestackRustCalendarRecurrenceRule
) -> RuntimeHostCalendarRecurrenceRule? {
  guard
    let byWeekDays = decodeCalendarU8Slice(value.by_week_days),
    let byWeekdayOrdinals = decodeCalendarRecurrenceWeekdays(value.by_weekday_ordinals),
    let byMonthDays = decodeCalendarI8Slice(value.by_month_days),
    let byMonths = decodeCalendarU8Slice(value.by_months),
    let byYearDays = decodeCalendarI16Slice(value.by_year_days),
    let byWeekNumbers = decodeCalendarI8Slice(value.by_week_numbers),
    let bySetPositions = decodeCalendarI16Slice(value.by_set_positions),
    let frequency = RuntimeHostCalendarRecurrenceFrequency(rawValue: value.frequency.rawValue)
  else {
    return nil
  }

  return RuntimeHostCalendarRecurrenceRule(
    frequency: frequency,
    interval: value.interval,
    count: decodeOptionalCalendarU32(value.count),
    untilUnixNs: decodeOptionalCalendarU64(value.until_unix_ns),
    byWeekDays: byWeekDays,
    byWeekdayOrdinals: byWeekdayOrdinals,
    byMonthDays: byMonthDays,
    byMonths: byMonths,
    byYearDays: byYearDays,
    byWeekNumbers: byWeekNumbers,
    bySetPositions: bySetPositions
  )
}

/// Decode one native attendee slice into one Swift array.
private func decodeCalendarAttendees(
  hasValue: Bool,
  values: DestackRustCalendarAttendeeSlice
) -> [RuntimeHostCalendarAttendee]? {
  if !hasValue {
    return nil
  }

  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))
  return buffer.compactMap { value in
    guard
      let responseStatus = RuntimeHostCalendarParticipantStatus(
        rawValue: value.response_status.rawValue)
    else {
      return nil
    }

    return RuntimeHostCalendarAttendee(
      id: decodeOptionalCalendarString(value.id),
      name: decodeOptionalCalendarString(value.name),
      email: decodeOptionalCalendarString(value.email),
      optional: value.optional,
      organizer: value.organizer,
      responseStatus: responseStatus
    )
  }
}

/// Decode one native reminder slice into one Swift array.
private func decodeCalendarReminders(
  hasValue: Bool,
  values: DestackRustCalendarReminderSlice
) -> [RuntimeHostCalendarReminder]? {
  if !hasValue {
    return nil
  }

  guard let data = values.data else {
    return values.len == 0 ? [] : nil
  }

  let buffer = UnsafeBufferPointer(start: data, count: Int(values.len))
  return buffer.compactMap { value in
    switch value.kind {
    case DESTACK_RUST_CALENDAR_REMINDER_KIND_ABSOLUTE:
      return .absolute(
        RuntimeHostCalendarAbsoluteReminder(
          absoluteUnixNs: value.absolute_unix_ns
        )
      )
    case DESTACK_RUST_CALENDAR_REMINDER_KIND_RELATIVE:
      return .relative(
        RuntimeHostCalendarRelativeReminder(
          minutesBeforeStart: value.minutes_before_start
        )
      )
    default:
      return nil
    }
  }
}

/// Decode one native calendar query into one Swift value.
private func decodeCalendarQuery(
  _ value: DestackRustCalendarQuery
) -> RuntimeHostCalendarEventQuery? {
  guard let calendarIDs = decodeCalendarStringSlice(value.calendar_ids) else {
    return nil
  }

  return RuntimeHostCalendarEventQuery(
    calendarIds: calendarIDs,
    startUnixNs: value.start_unix_ns,
    endUnixNs: value.end_unix_ns,
    limit: decodeOptionalCalendarU32(value.limit),
    includeCanceled: value.include_canceled,
    includeDeclined: value.include_declined,
    includeRecurrenceInstances: value.include_recurrence_instances
  )
}

/// Decode one native calendar event draft into one Swift value.
private func decodeCalendarEventDraft(
  _ value: DestackRustCalendarEventDraft
) -> RuntimeHostCalendarEventDraft? {
  let recurrenceRule: RuntimeHostCalendarRecurrenceRule?

  if value.has_recurrence_rule {
    guard let decodedRule = decodeCalendarRecurrenceRule(value.recurrence_rule) else {
      return nil
    }

    recurrenceRule = decodedRule
  } else {
    recurrenceRule = nil
  }

  guard
    let calendarID = decodeCalendarString(value.calendar_id),
    let title = decodeCalendarString(value.title),
    let notes = decodeOptionalCalendarString(value.notes),
    let location = decodeOptionalCalendarString(value.location),
    let timeZone = decodeOptionalCalendarString(value.time_zone),
    let url = decodeOptionalCalendarString(value.url),
    let availability = RuntimeHostCalendarAvailability(rawValue: value.availability.rawValue),
    let attendees = decodeCalendarAttendees(
      hasValue: value.has_attendees,
      values: value.attendees
    ),
    let reminders = decodeCalendarReminders(
      hasValue: value.has_reminders,
      values: value.reminders
    )
  else {
    return nil
  }

  return RuntimeHostCalendarEventDraft(
    calendarId: calendarID,
    title: title,
    notes: notes,
    location: location,
    startUnixNs: value.start_unix_ns,
    endUnixNs: value.end_unix_ns,
    allDay: value.all_day,
    timeZone: timeZone,
    availability: availability,
    url: url,
    recurrenceRule: recurrenceRule,
    attendees: attendees,
    reminders: reminders
  )
}

/// Execute one body with one encoded calendar descriptor slice.
private func withEncodedCalendarDescriptors<T>(
  _ values: [RuntimeHostCalendarDescriptor],
  body: (DestackRustCalendarDescriptorSlice) -> T
) -> T {
  let arena = currentCalendarBridgeArena()
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = encodeCalendarDescriptor(
        value,
        arena: arena
      )
    }
  }

  return body(
    DestackRustCalendarDescriptorSlice(
      data: data,
      len: UInt32(values.count)
    )
  )
}

/// Execute one body with one encoded calendar event slice.
private func withEncodedCalendarEvents<T>(
  _ values: [RuntimeHostCalendarEvent],
  body: (DestackRustCalendarEventSlice) -> T
) -> T {
  let arena = currentCalendarBridgeArena()
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = encodeCalendarEvent(
        value,
        arena: arena
      )
    }
  }

  return body(
    DestackRustCalendarEventSlice(
      data: data,
      len: UInt32(values.count)
    )
  )
}

/// Execute one body with one encoded calendar event.
private func withEncodedCalendarEvent<T>(
  _ value: RuntimeHostCalendarEvent,
  body: (DestackRustCalendarEvent) -> T
) -> T {
  let arena = currentCalendarBridgeArena()
  let encodedValue = encodeCalendarEvent(
    value,
    arena: arena
  )

  return body(encodedValue)
}

/// Encode one optional string into one native optional string.
private func encodeOptionalCalendarString(
  _ value: String?,
  arena: CalendarBridgeArena
) -> DestackRustOptionalStringRef {
  guard let value else {
    return DestackRustOptionalStringRef(
      has_value: false,
      value: DestackRustStringRef(data: nil, len: 0)
    )
  }

  return DestackRustOptionalStringRef(
    has_value: true,
    value: arena.makeStringRef(value)
  )
}

/// Encode one optional `u32` into one native optional.
private func encodeOptionalCalendarU32(
  _ value: UInt32?
) -> DestackRustOptionalU32 {
  DestackRustOptionalU32(
    has_value: value != nil,
    value: value ?? 0
  )
}

/// Encode one optional `u64` into one native optional.
private func encodeOptionalCalendarU64(
  _ value: UInt64?
) -> DestackRustOptionalU64 {
  DestackRustOptionalU64(
    has_value: value != nil,
    value: value ?? 0
  )
}

/// Encode one optional `i8` into one native optional.
private func encodeOptionalCalendarI8(
  _ value: Int8?
) -> DestackRustOptionalI8 {
  DestackRustOptionalI8(
    has_value: value != nil,
    value: value ?? 0
  )
}

/// Encode one `u8` array into one native slice.
private func encodeCalendarU8Slice(
  _ values: [UInt8],
  arena: CalendarBridgeArena
) -> DestackRustU8Slice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = value
    }
  }

  return DestackRustU8Slice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one `i8` array into one native slice.
private func encodeCalendarI8Slice(
  _ values: [Int8],
  arena: CalendarBridgeArena
) -> DestackRustI8Slice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = value
    }
  }

  return DestackRustI8Slice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one `i16` array into one native slice.
private func encodeCalendarI16Slice(
  _ values: [Int16],
  arena: CalendarBridgeArena
) -> DestackRustI16Slice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = value
    }
  }

  return DestackRustI16Slice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one recurrence weekday array into one native slice.
private func encodeCalendarRecurrenceWeekdays(
  _ values: [RuntimeHostCalendarRecurrenceWeekday],
  arena: CalendarBridgeArena
) -> DestackRustCalendarRecurrenceWeekdaySlice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = DestackRustCalendarRecurrenceWeekday(
        day: value.day,
        week_number: encodeOptionalCalendarI8(value.weekNumber)
      )
    }
  }

  return DestackRustCalendarRecurrenceWeekdaySlice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one recurrence rule into one native payload.
private func encodeCalendarRecurrenceRule(
  _ value: RuntimeHostCalendarRecurrenceRule,
  arena: CalendarBridgeArena
) -> DestackRustCalendarRecurrenceRule {
  DestackRustCalendarRecurrenceRule(
    frequency: DestackRustCalendarRecurrenceFrequency(rawValue: value.frequency.rawValue),
    interval: value.interval,
    count: encodeOptionalCalendarU32(value.count),
    until_unix_ns: encodeOptionalCalendarU64(value.untilUnixNs),
    by_week_days: encodeCalendarU8Slice(value.byWeekDays, arena: arena),
    by_weekday_ordinals: encodeCalendarRecurrenceWeekdays(value.byWeekdayOrdinals, arena: arena),
    by_month_days: encodeCalendarI8Slice(value.byMonthDays, arena: arena),
    by_months: encodeCalendarU8Slice(value.byMonths, arena: arena),
    by_year_days: encodeCalendarI16Slice(value.byYearDays, arena: arena),
    by_week_numbers: encodeCalendarI8Slice(value.byWeekNumbers, arena: arena),
    by_set_positions: encodeCalendarI16Slice(value.bySetPositions, arena: arena)
  )
}

/// Encode one attendee array into one native slice.
private func encodeCalendarAttendees(
  _ values: [RuntimeHostCalendarAttendee],
  arena: CalendarBridgeArena
) -> DestackRustCalendarAttendeeSlice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      buffer[index] = DestackRustCalendarAttendee(
        id: encodeOptionalCalendarString(value.id, arena: arena),
        name: encodeOptionalCalendarString(value.name, arena: arena),
        email: encodeOptionalCalendarString(value.email, arena: arena),
        optional: value.optional,
        organizer: value.organizer,
        response_status: DestackRustCalendarParticipantStatus(
          rawValue: value.responseStatus.rawValue)
      )
    }
  }

  return DestackRustCalendarAttendeeSlice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one reminder array into one native slice.
private func encodeCalendarReminders(
  _ values: [RuntimeHostCalendarReminder],
  arena: CalendarBridgeArena
) -> DestackRustCalendarReminderSlice {
  let data = arena.makeArray(count: values.count) { buffer in
    for (index, value) in values.enumerated() {
      switch value {
      case .absolute(let reminder):
        buffer[index] = DestackRustCalendarReminder(
          kind: DESTACK_RUST_CALENDAR_REMINDER_KIND_ABSOLUTE,
          absolute_unix_ns: reminder.absoluteUnixNs,
          minutes_before_start: 0
        )
      case .relative(let reminder):
        buffer[index] = DestackRustCalendarReminder(
          kind: DESTACK_RUST_CALENDAR_REMINDER_KIND_RELATIVE,
          absolute_unix_ns: 0,
          minutes_before_start: reminder.minutesBeforeStart
        )
      }
    }
  }

  return DestackRustCalendarReminderSlice(
    data: data,
    len: UInt32(values.count)
  )
}

/// Encode one calendar descriptor into one native payload.
private func encodeCalendarDescriptor(
  _ value: RuntimeHostCalendarDescriptor,
  arena: CalendarBridgeArena
) -> DestackRustCalendarDescriptor {
  DestackRustCalendarDescriptor(
    id: arena.makeStringRef(value.id),
    title: arena.makeStringRef(value.title),
    source: arena.makeStringRef(value.source),
    owner: encodeOptionalCalendarString(value.owner, arena: arena),
    color_argb: value.colorArgb,
    primary: value.primary,
    access: DestackRustCalendarAccess(rawValue: value.access.rawValue)
  )
}

/// Encode one calendar event into one native payload.
private func encodeCalendarEvent(
  _ value: RuntimeHostCalendarEvent,
  arena: CalendarBridgeArena
) -> DestackRustCalendarEvent {
  let hasRecurrenceRule = value.recurrenceRule != nil
  let recurrenceRule =
    value.recurrenceRule.map { rule in
      encodeCalendarRecurrenceRule(rule, arena: arena)
    } ?? DestackRustCalendarRecurrenceRule()

  let hasAttendees = value.attendees != nil
  let attendees = encodeCalendarAttendees(value.attendees ?? [], arena: arena)

  let hasReminders = value.reminders != nil
  let reminders = encodeCalendarReminders(value.reminders ?? [], arena: arena)

  return DestackRustCalendarEvent(
    id: arena.makeStringRef(value.id),
    calendar_id: arena.makeStringRef(value.calendarId),
    title: arena.makeStringRef(value.title),
    notes: encodeOptionalCalendarString(value.notes, arena: arena),
    location: encodeOptionalCalendarString(value.location, arena: arena),
    start_unix_ns: value.startUnixNs,
    end_unix_ns: value.endUnixNs,
    all_day: value.allDay,
    canceled: value.canceled,
    time_zone: encodeOptionalCalendarString(value.timeZone, arena: arena),
    availability: DestackRustCalendarAvailability(rawValue: value.availability.rawValue),
    url: encodeOptionalCalendarString(value.url, arena: arena),
    organizer_name: encodeOptionalCalendarString(value.organizerName, arena: arena),
    organizer_email: encodeOptionalCalendarString(value.organizerEmail, arena: arena),
    recurring: value.recurring,
    recurrence_master_id: encodeOptionalCalendarString(value.recurrenceMasterId, arena: arena),
    recurrence_id_unix_ns: encodeOptionalCalendarU64(value.recurrenceIdUnixNs),
    has_recurrence_rule: hasRecurrenceRule,
    recurrence_rule: recurrenceRule,
    has_attendees: hasAttendees,
    attendees: attendees,
    has_reminders: hasReminders,
    reminders: reminders
  )
}
