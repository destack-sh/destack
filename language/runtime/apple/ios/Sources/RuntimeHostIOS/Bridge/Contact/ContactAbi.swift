import RuntimeHostAppleBridgeC
import RuntimeHostAppleCore

/// One C callback for one contact-list request.
typealias ContactListCallback =
  @convention(c) (UInt64, DestackRustContactQuery, UnsafeMutablePointer<DestackRustContactPage>?) -> UInt32

/// One C callback for one contact-search request.
typealias ContactSearchCallback =
  @convention(c) (
    UInt64,
    DestackRustStringRef,
    DestackRustContactQuery,
    UnsafeMutablePointer<DestackRustContactPage>?
  ) -> UInt32

/// One C callback for one contact-read request.
typealias ContactReadCallback =
  @convention(c) (UInt64, DestackRustStringRef, UnsafeMutablePointer<DestackRustContact>?) -> UInt32

/// One C callback for one contact-create request.
typealias ContactCreateCallback =
  @convention(c) (UInt64, DestackRustContactDraft, UnsafeMutablePointer<DestackRustStringRef>?) -> UInt32

/// One C callback for one contact-update request.
typealias ContactUpdateCallback =
  @convention(c) (UInt64, DestackRustStringRef, DestackRustContactDraft) -> UInt32

/// One C callback for one contact-delete request.
typealias ContactDeleteCallback =
  @convention(c) (UInt64, DestackRustStringRef) -> UInt32

/// The low-level contact ABI surface for one iOS runtime bridge.
protocol ContactAbi: Sendable {}
