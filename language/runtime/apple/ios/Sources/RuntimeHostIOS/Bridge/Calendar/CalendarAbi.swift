import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one calendar-list request.
typealias CalendarListCallback =
  @convention(c) (UInt64, UnsafeMutablePointer<DestackRustCalendarDescriptorSlice>?) -> UInt32

/// One C callback for one calendar event-list request.
typealias CalendarEventListCallback =
  @convention(c) (
    UInt64, DestackRustCalendarQuery, UnsafeMutablePointer<DestackRustCalendarEventSlice>?
  ) -> UInt32

/// One C callback for one calendar event-read request.
typealias CalendarEventReadCallback =
  @convention(c) (UInt64, DestackRustStringRef, UnsafeMutablePointer<DestackRustCalendarEvent>?) ->
  UInt32

/// One C callback for one calendar event-create request.
typealias CalendarEventCreateCallback =
  @convention(c) (
    UInt64, DestackRustCalendarEventDraft, UnsafeMutablePointer<DestackRustStringRef>?
  ) -> UInt32

/// One C callback for one calendar event-update request.
typealias CalendarEventUpdateCallback =
  @convention(c) (UInt64, DestackRustStringRef, DestackRustCalendarEventDraft) -> UInt32

/// One C callback for one calendar event-delete request.
typealias CalendarEventDeleteCallback =
  @convention(c) (UInt64, DestackRustStringRef) -> UInt32

/// The low-level calendar ABI surface for one iOS runtime bridge.
protocol CalendarAbi: Sendable {}
