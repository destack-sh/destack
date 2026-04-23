use std::error::Error;
use std::fmt::{self, Display, Formatter};

use destack_mir::LayoutId;

use crate::allocator::PageId;
use crate::{AccountingRegion, HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};

/// One heap result.
pub type HeapResult<T> = Result<T, HeapError>;

/// One heap scan source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanSource {
    /// One heap reference payload.
    Reference(HeapReference),
    /// One mature span.
    Span(usize),
    /// One mature large entry.
    LargeEntry(u64),
}

/// Heap configuration failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapError {
    /// The configured GC trigger percentage is unsupported.
    InvalidGcTriggerPercent { percent: u32 },
    /// The configured heap page width is unsupported.
    InvalidPageBytes { bytes: usize },
    /// The configured allocator arena width is unsupported.
    InvalidAllocatorArenaBytes { bytes: usize },
    /// The configured allocator arena width is not aligned to the page width.
    MisalignedAllocatorArenaBytes {
        /// The configured page width in bytes.
        page_bytes: usize,
        /// The configured allocator arena width in bytes.
        arena_bytes: usize,
    },
    /// The explicit allocator does not match the configured heap page width.
    AllocatorPageBytesMismatch {
        /// The page width configured through heap options.
        option_page_bytes: usize,
        /// The actual page width of the explicit allocator.
        allocator_page_bytes: usize,
    },
    /// The explicit allocator does not match the configured heap arena width.
    AllocatorArenaBytesMismatch {
        /// The arena width configured through heap options.
        option_arena_bytes: usize,
        /// The actual arena width of the explicit allocator.
        allocator_arena_bytes: usize,
    },
    /// The configured heap young-allocation threshold exceeds young-space capacity.
    HeapYoungThresholdExceedsCapacity { threshold: usize, capacity: usize },
    /// The configured small-allocation alignment is unsupported.
    InvalidSmallAllocationAlignmentBytes { bytes: usize },
    /// One size class violated the configured small-allocation alignment.
    MisalignedSizeClass {
        /// The required alignment in bytes.
        alignment_bytes: usize,
        /// The misaligned size class in bytes.
        class_bytes: usize,
    },
    /// The configured size-class table was empty.
    EmptySizeClassTable,
    /// One configured size class was zero.
    ZeroSizeClass,
    /// The configured size classes were not strictly increasing.
    NonMonotonicSizeClass {
        /// The previous size class in bytes.
        previous: usize,
        /// The current size class in bytes.
        bytes: usize,
    },
    /// The configured size-class policy range is invalid.
    InvalidSizeClassPolicyRange {
        /// The configured minimum generated size class in bytes.
        min_bytes: usize,
        /// The configured maximum generated size class in bytes.
        max_bytes: usize,
    },
    /// The configured size-class policy alignment is invalid.
    InvalidSizeClassPolicyAlignment {
        /// The configured alignment in bytes.
        alignment_bytes: usize,
    },
    /// The configured size-class policy waste ratio is invalid.
    InvalidSizeClassPolicyWaste {
        /// The configured waste ratio numerator.
        numerator: usize,
        /// The configured waste ratio denominator.
        denominator: usize,
    },
    /// One restored or requested size class does not exist in the configured table.
    InvalidSizeClass {
        /// The invalid size class in bytes.
        class_bytes: usize,
    },
    /// The allocator arena count cannot grow far enough for one entry.
    AllocatorArenaLimitExceeded {
        /// The required arena count.
        required_arenas: usize,
        /// The maximum configured arena count.
        max_arenas: usize,
    },
    /// One allocator arena allocation failed.
    AllocatorArenaAllocationFailed {
        /// The requested arena byte length.
        byte_len: usize,
    },
    /// One allocator address was outside the supported arena map.
    AllocatorAddressUnsupported {
        /// The mapped arena address.
        address: usize,
    },
    /// One heap-space hard limit was exceeded.
    LimitExceeded {
        /// The heap space that exceeded its limit.
        region: AccountingRegion,
        /// The exact bytes in use.
        used_bytes: u64,
        /// The configured limit.
        max_bytes: u64,
    },
    /// One combined heap hard limit was exceeded.
    TotalLimitExceeded {
        /// The exact total bytes in use.
        used_bytes: u64,
        /// The configured total limit.
        max_bytes: u64,
    },
    /// One heap capture request found active collector work.
    CaptureGcActive,
    /// One heap capture request found active heap pins.
    CapturePinsActive,
    /// One heap collection was requested while another collection was active.
    HeapCollectionActive,
    /// One shared heap collection was requested while another collection was active.
    SharedCollectionActive,
    /// One shared heap mark operation was requested while shared mark was inactive.
    SharedCollectionNotMarking,
    /// One heap pin count could not represent one additional scoped pin.
    HeapPinCountOverflow {
        /// The pinned heap reference.
        reference: HeapReference,
        /// The current pin count before the failed increment.
        count: u32,
    },
    /// One heap pin set could not represent one additional active scoped pin.
    HeapPinActiveCountOverflow {
        /// The current active pin count before the failed increment.
        active_count: usize,
    },
    /// One heap reference was unpinned without one active scoped pin.
    HeapPinMissing {
        /// The unpinned heap reference.
        reference: HeapReference,
    },
    /// One heap pin set lost its active-count invariant while unpinning.
    HeapPinActiveCountUnderflow {
        /// The unpinned heap reference.
        reference: HeapReference,
        /// The current active pin count before the failed decrement.
        active_count: usize,
    },
    /// One heap byte range was outside the logical entry.
    InvalidByteRange {
        /// The requested byte offset.
        start: usize,
        /// The requested byte length.
        len: usize,
        /// The logical entry capacity in bytes.
        capacity: usize,
    },
    /// One heap reference did not resolve to one live entry.
    InvalidHeapReference {
        /// The invalid heap reference.
        reference: HeapReference,
    },
    /// One heap collection scan failed.
    HeapScanFailed {
        /// The heap scan source.
        source: ScanSource,
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// One heap collection promotion could not allocate a mature small slot.
    HeapPromotionUnavailableSmallSlot {
        /// The heap reference being promoted.
        reference: HeapReference,
        /// The promoted payload length in bytes.
        byte_len: usize,
    },
    /// One heap collection promotion failed.
    HeapPromotionFailed {
        /// The heap reference being promoted.
        reference: HeapReference,
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// One heap collection young-space reset failed.
    HeapYoungResetFailed {
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// One heap collection free failed.
    HeapFreeFailed {
        /// The heap reference being freed.
        reference: HeapReference,
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// The heap-space usage counters cannot service one release.
    InvalidUsage {
        /// The heap space whose counters were invalid.
        region: AccountingRegion,
        /// The live entry count before the release.
        allocated_count: usize,
        /// The live byte count before the release.
        allocated_bytes: u64,
        /// The bytes being released.
        freed_bytes: u64,
    },
    /// One raw pointer did not resolve to one live entry.
    InvalidRawPointer {
        /// The invalid raw pointer.
        pointer: RawPointer,
    },
    /// One direct page-view write expected unique allocator pages.
    BorrowedPageWrite {
        /// The first borrowed logical page index in the write window.
        page_index: usize,
    },
    /// One shared heap reference did not resolve to one live entry.
    InvalidSharedHeapReference {
        /// The invalid shared heap reference.
        reference: SharedHeapReference,
    },
    /// One shared raw pointer did not resolve to one live entry.
    InvalidSharedRawPointer {
        /// The invalid shared raw pointer.
        pointer: SharedRawPointer,
    },
    /// One internal heap invariant exceeded representable arithmetic range.
    InvariantOverflow {
        /// The overflowing invariant context.
        context: &'static str,
    },
    /// One serialized image page could not be resolved.
    ImageMissingPage {
        /// The missing page identifier.
        page_id: PageId,
    },
    /// One serialized image page had the wrong byte length.
    ImageInvalidPageBytes {
        /// The invalid page identifier.
        page_id: PageId,
        /// The expected byte length.
        expected: usize,
        /// The actual byte length.
        actual: usize,
    },
    /// One validated logical page map lost one visible page slot.
    MissingLogicalPage {
        /// The missing logical page index.
        page_index: usize,
    },
    /// One validated physical allocator page could not be resolved.
    MissingPage {
        /// The missing page identifier.
        page_id: PageId,
    },
    /// One reserved span disappeared before initialization or access.
    MissingSpan {
        /// The missing span index.
        span_index: usize,
    },
    /// One live small entry disappeared from its owning span.
    MissingSmallSlot {
        /// The missing span index.
        span_index: usize,
        /// The missing slot index.
        slot_index: usize,
    },
    /// One live young entry disappeared before initialization or access.
    MissingYoungEntry {
        /// The missing young-space generation.
        generation: u32,
        /// The missing young-entry index.
        entry_index: u32,
    },
    /// One live large entry disappeared before access.
    MissingLargeEntry {
        /// The missing large-entry identifier.
        entry_id: u64,
    },
    /// One large-entry id cannot be represented by large-entry tables.
    InvalidLargeEntryId {
        /// The invalid large-entry id.
        id: u64,
    },
    /// One run refcount entry was missing for one live run.
    MissingRunRefcount {
        /// The first page of the live run.
        first_page: PageId,
    },
    /// One run refcount cannot service one retain or release.
    InvalidRunRefcount {
        /// The first page of the live run.
        first_page: PageId,
        /// The invalid refcount value.
        refcount: u32,
    },
    /// One page identifier exceeded the encoded allocator page range.
    InvalidPageId {
        /// The invalid page index.
        index: usize,
    },
    /// One managed layout id did not resolve in the heap layout table.
    InvalidLayoutId {
        /// The invalid layout index.
        index: usize,
    },
    /// One managed layout was missing from one derived heap index.
    MissingLayout {
        /// The missing layout identifier.
        layout_id: LayoutId,
    },
    /// One page run exceeded the encoded allocator page range.
    InvalidPageRun {
        /// The first page of the run.
        first_page: PageId,
        /// The requested page count.
        page_count: usize,
    },
    /// One span slot exceeded the encoded small-space slot range.
    InvalidSmallSlot {
        /// The invalid span index.
        span_index: usize,
        /// The invalid slot index.
        slot_index: usize,
    },
    /// One patched run violated the single-page patch invariant.
    InvalidPatchedRun {
        /// The patched page index.
        page_index: usize,
        /// The patched run page count.
        page_count: usize,
    },
    /// One dense copy on write table entry was missing unexpectedly.
    MissingTableEntry {
        /// The missing dense entry index.
        index: usize,
    },
    /// One decoded heap-reference window has an unsupported width.
    InvalidReferenceWindowWidth {
        /// The invalid byte width.
        bytes: usize,
    },
    /// One traced field width overflowed its byte offset.
    TraceOffsetOverflow {
        /// The traced field byte offset.
        start: usize,
        /// The traced field byte width.
        width: usize,
    },
    /// One traced field extended past the provided payload bytes.
    TruncatedTracePayload {
        /// The traced field byte offset.
        start: usize,
        /// The traced field byte width.
        width: usize,
        /// The available payload length.
        len: usize,
    },
    /// One traced field could not be read from one random-access reader.
    TruncatedReferenceReaderWindow {
        /// The traced field byte offset.
        start: usize,
        /// The traced field byte width.
        width: usize,
    },
    /// One traced value payload was invalid.
    InvalidReferenceValuePayload {
        /// The traced value byte offset.
        start: usize,
    },
    /// One managed allocation did not match its declared layout width.
    InvalidLayoutBytes {
        /// The layout identifier used for the allocation.
        layout_id: LayoutId,
        /// The expected byte length from the layout table.
        expected: usize,
        /// The actual byte length requested by the caller.
        actual: usize,
    },
}

impl Display for HeapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGcTriggerPercent { percent } => {
                write!(formatter, "invalid GC trigger percent: {percent}")
            }
            Self::InvalidPageBytes { bytes } => {
                write!(
                    formatter,
                    "invalid heap page width for heap options: {bytes}"
                )
            }
            Self::InvalidAllocatorArenaBytes { bytes } => {
                write!(
                    formatter,
                    "invalid allocator arena width for heap options: {bytes}"
                )
            }
            Self::MisalignedAllocatorArenaBytes {
                page_bytes,
                arena_bytes,
            } => {
                write!(
                    formatter,
                    "allocator arena width violates heap page alignment: {arena_bytes} is not a multiple of {page_bytes}"
                )
            }
            Self::AllocatorPageBytesMismatch {
                option_page_bytes,
                allocator_page_bytes,
            } => {
                write!(
                    formatter,
                    "explicit allocator page width does not match heap options: options {option_page_bytes}, allocator {allocator_page_bytes}"
                )
            }
            Self::AllocatorArenaBytesMismatch {
                option_arena_bytes,
                allocator_arena_bytes,
            } => {
                write!(
                    formatter,
                    "explicit allocator arena width does not match heap options: options {option_arena_bytes}, allocator {allocator_arena_bytes}"
                )
            }
            Self::HeapYoungThresholdExceedsCapacity {
                threshold,
                capacity,
            } => {
                write!(
                    formatter,
                    "heap young allocation threshold exceeds young-space capacity: {threshold} > {capacity}"
                )
            }
            Self::InvalidSmallAllocationAlignmentBytes { bytes } => {
                write!(
                    formatter,
                    "invalid small-allocation alignment for heap options: {bytes}"
                )
            }
            Self::MisalignedSizeClass {
                alignment_bytes,
                class_bytes,
            } => {
                write!(
                    formatter,
                    "size class violates heap small-allocation alignment: {class_bytes} is not a multiple of {alignment_bytes}"
                )
            }
            Self::EmptySizeClassTable => {
                write!(
                    formatter,
                    "size-class table must contain at least one class"
                )
            }
            Self::ZeroSizeClass => {
                write!(
                    formatter,
                    "size-class table must not contain zero-byte classes"
                )
            }
            Self::NonMonotonicSizeClass { previous, bytes } => {
                write!(
                    formatter,
                    "size-class table must be strictly increasing: {bytes} follows {previous}"
                )
            }
            Self::InvalidSizeClassPolicyRange {
                min_bytes,
                max_bytes,
            } => {
                write!(
                    formatter,
                    "size-class policy range is invalid: min {min_bytes}, max {max_bytes}"
                )
            }
            Self::InvalidSizeClassPolicyAlignment { alignment_bytes } => {
                write!(
                    formatter,
                    "size-class policy alignment is invalid: {alignment_bytes}"
                )
            }
            Self::InvalidSizeClassPolicyWaste {
                numerator,
                denominator,
            } => {
                write!(
                    formatter,
                    "size-class policy waste ratio is invalid: {numerator}/{denominator}"
                )
            }
            Self::InvalidSizeClass { class_bytes } => {
                write!(
                    formatter,
                    "size class is not present in the configured heap table: {class_bytes}"
                )
            }
            Self::AllocatorArenaLimitExceeded {
                required_arenas,
                max_arenas,
            } => {
                write!(
                    formatter,
                    "allocator arena limit exceeded: required {required_arenas} arenas with maximum {max_arenas}"
                )
            }
            Self::AllocatorArenaAllocationFailed { byte_len } => {
                write!(
                    formatter,
                    "allocator arena allocation failed: {byte_len} bytes"
                )
            }
            Self::AllocatorAddressUnsupported { address } => {
                write!(
                    formatter,
                    "allocator arena address is outside the supported arena map: {address:#x}"
                )
            }
            Self::LimitExceeded {
                region,
                used_bytes,
                max_bytes,
            } => {
                let subject = region.subject();

                write!(
                    formatter,
                    "{subject} limit exceeded: using {used_bytes} bytes with limit {max_bytes}"
                )
            }
            Self::TotalLimitExceeded {
                used_bytes,
                max_bytes,
            } => {
                write!(
                    formatter,
                    "total heap limit exceeded: using {used_bytes} bytes with limit {max_bytes}"
                )
            }
            Self::CaptureGcActive => {
                write!(formatter, "heap capture requires idle gc state")
            }
            Self::CapturePinsActive => {
                write!(formatter, "heap capture requires no active heap pins")
            }
            Self::HeapCollectionActive => {
                write!(formatter, "heap collection is already active")
            }
            Self::SharedCollectionActive => {
                write!(formatter, "shared heap collection is already active")
            }
            Self::SharedCollectionNotMarking => {
                write!(formatter, "shared heap is not currently marking")
            }
            Self::HeapPinCountOverflow { reference, count } => {
                write!(
                    formatter,
                    "heap pin count overflow for reference {reference:?} at count {count}"
                )
            }
            Self::HeapPinActiveCountOverflow { active_count } => {
                write!(
                    formatter,
                    "heap active pin count overflow at count {active_count}"
                )
            }
            Self::HeapPinMissing { reference } => {
                write!(
                    formatter,
                    "heap reference {reference:?} is not currently pinned"
                )
            }
            Self::HeapPinActiveCountUnderflow {
                reference,
                active_count,
            } => {
                write!(
                    formatter,
                    "heap active pin count underflow while unpinning reference {reference:?} at count {active_count}"
                )
            }
            Self::InvalidByteRange {
                start,
                len,
                capacity,
            } => {
                write!(
                    formatter,
                    "invalid heap byte range: start {start}, len {len}, capacity {capacity}"
                )
            }
            Self::InvalidHeapReference { reference } => {
                write!(formatter, "invalid heap reference: {reference:?}")
            }
            Self::HeapScanFailed { source, error } => match source {
                ScanSource::Reference(reference) => {
                    write!(
                        formatter,
                        "heap scan failed for reference {reference:?}: {error}"
                    )
                }
                ScanSource::Span(span_index) => {
                    write!(
                        formatter,
                        "heap scan failed for dirty span {span_index}: {error}"
                    )
                }
                ScanSource::LargeEntry(entry_id) => {
                    write!(
                        formatter,
                        "heap scan failed for dirty large entry {entry_id}: {error}"
                    )
                }
            },
            Self::HeapPromotionUnavailableSmallSlot {
                reference,
                byte_len,
            } => {
                write!(
                    formatter,
                    "heap promotion could not allocate one mature small slot for reference {reference:?} with {byte_len} bytes"
                )
            }
            Self::HeapPromotionFailed { reference, error } => {
                write!(
                    formatter,
                    "heap promotion failed for reference {reference:?}: {error}"
                )
            }
            Self::HeapYoungResetFailed { error } => {
                write!(formatter, "heap young reset failed: {error}")
            }
            Self::HeapFreeFailed { reference, error } => {
                write!(
                    formatter,
                    "heap free failed for reference {reference:?}: {error}"
                )
            }
            Self::InvalidUsage {
                region,
                allocated_count,
                allocated_bytes,
                freed_bytes,
            } => {
                let subject = region.subject();

                write!(
                    formatter,
                    "invalid {subject} usage: count {allocated_count}, bytes {allocated_bytes}, freed bytes {freed_bytes}"
                )
            }
            Self::InvalidRawPointer { pointer } => {
                write!(formatter, "invalid raw pointer: {pointer:?}")
            }
            Self::BorrowedPageWrite { page_index } => {
                write!(
                    formatter,
                    "direct page-view write requires unique pages: page {page_index} is still borrowed"
                )
            }
            Self::InvalidSharedHeapReference { reference } => {
                write!(formatter, "invalid shared heap reference: {reference:?}")
            }
            Self::InvalidSharedRawPointer { pointer } => {
                write!(formatter, "invalid shared raw pointer: {pointer:?}")
            }
            Self::InvariantOverflow { context } => {
                write!(formatter, "heap invariant overflow: {context}")
            }
            Self::ImageMissingPage { page_id } => {
                write!(formatter, "heap image is missing page {page_id:?}")
            }
            Self::ImageInvalidPageBytes {
                page_id,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "heap image page {page_id:?} has invalid byte length: expected {expected}, got {actual}"
                )
            }
            Self::MissingLogicalPage { page_index } => {
                write!(formatter, "heap lost logical page at index {page_index}")
            }
            Self::MissingPage { page_id } => {
                write!(formatter, "heap lost live page {page_id:?}")
            }
            Self::MissingSpan { span_index } => {
                write!(formatter, "heap lost live span at index {span_index}")
            }
            Self::MissingSmallSlot {
                span_index,
                slot_index,
            } => {
                write!(
                    formatter,
                    "heap lost live span slot at span {span_index}, slot {slot_index}"
                )
            }
            Self::MissingYoungEntry {
                generation,
                entry_index,
            } => {
                write!(
                    formatter,
                    "heap lost live young entry in generation {generation} at index {entry_index}"
                )
            }
            Self::MissingLargeEntry { entry_id } => {
                write!(formatter, "heap lost live large entry with id {entry_id}")
            }
            Self::InvalidLargeEntryId { id } => {
                write!(formatter, "invalid large-entry id: {id}")
            }
            Self::MissingRunRefcount { first_page } => {
                write!(
                    formatter,
                    "heap lost run refcount for live run starting at {first_page:?}"
                )
            }
            Self::InvalidRunRefcount {
                first_page,
                refcount,
            } => {
                write!(
                    formatter,
                    "invalid run refcount for run starting at {first_page:?}: {refcount}"
                )
            }
            Self::InvalidPageId { index } => {
                write!(
                    formatter,
                    "page identifier exceeds encoded page range: {index}"
                )
            }
            Self::InvalidLayoutId { index } => {
                write!(formatter, "invalid managed layout id: {index}")
            }
            Self::MissingLayout { layout_id } => {
                write!(formatter, "managed layout {layout_id:?} is missing")
            }
            Self::InvalidPageRun {
                first_page,
                page_count,
            } => {
                write!(
                    formatter,
                    "page run exceeds encoded page range: first page {first_page:?}, page count {page_count}"
                )
            }
            Self::InvalidSmallSlot {
                span_index,
                slot_index,
            } => {
                write!(
                    formatter,
                    "span slot exceeds encoded slot range: span {span_index}, slot {slot_index}"
                )
            }
            Self::InvalidPatchedRun {
                page_index,
                page_count,
            } => {
                write!(
                    formatter,
                    "heap patched run at page index {page_index} has invalid page count {page_count}"
                )
            }
            Self::MissingTableEntry { index } => {
                write!(formatter, "heap lost dense table entry at index {index}")
            }
            Self::InvalidReferenceWindowWidth { bytes } => {
                write!(
                    formatter,
                    "unsupported heap reference width for tracing window: {bytes}"
                )
            }
            Self::TraceOffsetOverflow { start, width } => {
                write!(
                    formatter,
                    "heap reference offset overflow while tracing: start={start}, width={width}"
                )
            }
            Self::TruncatedTracePayload { start, width, len } => {
                write!(
                    formatter,
                    "truncated heap reference payload while tracing: start={start}, width={width}, len={len}"
                )
            }
            Self::TruncatedReferenceReaderWindow { start, width } => {
                write!(
                    formatter,
                    "truncated heap reference payload while tracing: start={start}, width={width}"
                )
            }
            Self::InvalidReferenceValuePayload { start } => {
                write!(
                    formatter,
                    "invalid value payload while tracing heap references: start={start}"
                )
            }
            Self::InvalidLayoutBytes {
                layout_id,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "managed allocation bytes do not match layout {layout_id:?}: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl Error for HeapError {}
