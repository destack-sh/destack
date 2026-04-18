use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::arena::PageId;
use crate::value::{ManagedReference, RawPointer, SharedManagedReference, SharedRawPointer};

/// One heap result.
pub type HeapResult<T> = Result<T, HeapError>;

/// One heap domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapDomain {
    /// Managed heap memory.
    Managed,
    /// Raw heap memory.
    Raw,
    /// Shared heap memory.
    Shared,
    /// Combined heap memory.
    Total,
}

impl HeapDomain {
    /// Return the heap usage subject for this domain.
    fn usage_subject(self) -> &'static str {
        match self {
            Self::Managed => "managed heap",
            Self::Raw => "raw heap",
            Self::Shared => "shared heap",
            Self::Total => "total heap",
        }
    }
}

impl Display for HeapDomain {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Managed => "managed",
            Self::Raw => "raw",
            Self::Shared => "shared",
            Self::Total => "total",
        };

        write!(formatter, "{label}")
    }
}

/// One managed trace source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedTraceSource {
    /// One managed reference payload.
    Reference(ManagedReference),
    /// One mature span.
    Span(usize),
    /// One mature large entry.
    LargeEntry(u64),
}

/// Heap configuration failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapError {
    /// The configured managed-reference width is unsupported.
    UnsupportedManagedReferenceWidth { bytes: u8 },
    /// The configured heap page width is unsupported.
    InvalidPageBytes { bytes: usize },
    /// The configured arena segment width is unsupported.
    InvalidArenaSegmentBytes { bytes: usize },
    /// The configured arena segment width is not aligned to the page width.
    MisalignedArenaSegmentBytes {
        /// The configured page width in bytes.
        page_bytes: usize,
        /// The configured arena segment width in bytes.
        segment_bytes: usize,
    },
    /// The explicit arena does not match the configured heap page width.
    ArenaPageBytesMismatch {
        /// The page width configured through heap options.
        option_page_bytes: usize,
        /// The actual page width of the explicit arena.
        arena_page_bytes: usize,
    },
    /// The explicit arena does not match the configured heap segment width.
    ArenaSegmentBytesMismatch {
        /// The segment width configured through heap options.
        option_segment_bytes: usize,
        /// The actual segment width of the explicit arena.
        arena_segment_bytes: usize,
    },
    /// The configured managed young-allocation threshold exceeds young-space capacity.
    ManagedYoungThresholdExceedsCapacity { threshold: usize, capacity: usize },
    /// The configured remembered-card width is unsupported.
    InvalidCardBytes { bytes: usize },
    /// The configured small-allocation alignment is unsupported.
    InvalidSmallAllocationAlignmentBytes { bytes: usize },
    /// The configured table chunk length is unsupported.
    InvalidTableChunkLen { len: usize },
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
    /// The arena segment count cannot grow far enough for one entry.
    ArenaSegmentLimitExceeded {
        /// The required segment count.
        required_segments: usize,
        /// The maximum configured segment count.
        max_segments: usize,
    },
    /// One arena segment entry layout was invalid.
    InvalidArenaSegmentLayout {
        /// The requested segment byte length.
        byte_len: usize,
        /// The requested page alignment in bytes.
        page_bytes: usize,
    },
    /// One arena segment entry failed.
    ArenaSegmentAllocationFailed {
        /// The requested segment byte length.
        byte_len: usize,
        /// The requested page alignment in bytes.
        page_bytes: usize,
    },
    /// One heap-domain hard limit was exceeded.
    LimitExceeded {
        /// The heap domain that exceeded its limit.
        domain: HeapDomain,
        /// The exact bytes in use.
        used_bytes: u64,
        /// The configured limit.
        max_bytes: u64,
    },
    /// One heap capture request found active collector work.
    CaptureGcActive,
    /// One heap capture request found active managed pins.
    CapturePinsActive,
    /// One managed collection was requested while another collection was active.
    ManagedCollectionActive,
    /// One managed collection was requested while scoped pins were active.
    ManagedCollectionPinsActive,
    /// One managed pin count could not represent one additional scoped pin.
    ManagedPinCountOverflow {
        /// The pinned managed reference.
        reference: ManagedReference,
        /// The current pin count before the failed increment.
        count: u32,
    },
    /// One managed pin set could not represent one additional active scoped pin.
    ManagedPinActiveCountOverflow {
        /// The current active pin count before the failed increment.
        active_count: usize,
    },
    /// One managed reference was unpinned without one active scoped pin.
    ManagedPinMissing {
        /// The unpinned managed reference.
        reference: ManagedReference,
    },
    /// One managed pin set lost its active-count invariant while unpinning.
    ManagedPinActiveCountUnderflow {
        /// The unpinned managed reference.
        reference: ManagedReference,
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
    /// One managed reference did not resolve to one live entry.
    InvalidManagedReference {
        /// The invalid managed reference.
        reference: ManagedReference,
    },
    /// One managed collection trace failed.
    ManagedTraceFailed {
        /// The managed trace source.
        source: ManagedTraceSource,
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// One managed collection promotion could not allocate a mature small slot.
    ManagedPromotionUnavailableSmallSlot {
        /// The managed reference being promoted.
        reference: ManagedReference,
        /// The promoted payload length in bytes.
        byte_len: usize,
    },
    /// One managed collection promotion failed.
    ManagedPromotionFailed {
        /// The managed reference being promoted.
        reference: ManagedReference,
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// One managed collection young-space reset failed.
    ManagedYoungResetFailed {
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// One managed collection free failed.
    ManagedFreeFailed {
        /// The managed reference being freed.
        reference: ManagedReference,
        /// The underlying heap failure.
        error: Box<HeapError>,
    },
    /// One managed reference id cannot be represented by managed references.
    InvalidManagedReferenceId {
        /// The invalid managed reference id.
        id: u64,
    },
    /// The heap-domain usage counters cannot service one release.
    InvalidUsage {
        /// The heap domain whose counters were invalid.
        domain: HeapDomain,
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
    /// One raw pointer id cannot be represented by raw pointers.
    InvalidRawPointerId {
        /// The invalid raw pointer id.
        id: u64,
    },
    /// One shared managed reference did not resolve to one live entry.
    InvalidSharedManagedReference {
        /// The invalid shared managed reference.
        reference: SharedManagedReference,
    },
    /// One shared managed reference id cannot be represented by shared managed references.
    InvalidSharedManagedReferenceId {
        /// The invalid shared managed reference id.
        id: u64,
    },
    /// One shared raw pointer did not resolve to one live entry.
    InvalidSharedRawPointer {
        /// The invalid shared raw pointer.
        pointer: SharedRawPointer,
    },
    /// One shared raw pointer id cannot be represented by shared raw pointers.
    InvalidSharedRawPointerId {
        /// The invalid shared raw pointer id.
        id: u64,
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
    /// One validated physical arena page could not be resolved.
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
    /// One page identifier exceeded the encoded arena page range.
    InvalidPageId {
        /// The invalid page index.
        index: usize,
    },
    /// One page run exceeded the encoded arena page range.
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
    /// One page view exhausted its inline patch capacity unexpectedly.
    PagePatchCapacityExceeded {
        /// The logical page count of the view.
        page_count: usize,
        /// The inline patch capacity.
        patch_capacity: usize,
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
    /// One reference-map id cannot be represented by the table.
    InvalidMapId {
        /// The invalid reference-map index.
        index: usize,
    },
    /// One reference-map table does not reserve id 0 for the empty map.
    InvalidMapSentinel {
        /// The invalid sentinel index.
        index: usize,
    },
    /// One decoded managed-reference window has an unsupported width.
    InvalidReferenceWindowWidth {
        /// The invalid byte width.
        bytes: usize,
    },
    /// One traced reference-map field width overflowed its byte offset.
    ReferenceMapOffsetOverflow {
        /// The traced field byte offset.
        start: usize,
        /// The traced field byte width.
        width: usize,
    },
    /// One traced reference-map field extended past the provided payload bytes.
    TruncatedReferenceMapPayload {
        /// The traced field byte offset.
        start: usize,
        /// The traced field byte width.
        width: usize,
        /// The available payload length.
        len: usize,
    },
    /// One traced reference-map field could not be read from one random-access reader.
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
    /// One reference-map table had duplicate maps where stable ids must be unique.
    DuplicateMap {
        /// The duplicate reference-map index.
        index: usize,
    },
    /// One reference-map table cannot be addressed by its reverse lookup.
    InvalidMapTableLen {
        /// The invalid map count.
        len: usize,
    },
}

impl Display for HeapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedManagedReferenceWidth { bytes } => {
                write!(formatter, "unsupported managed reference width: {bytes}")
            }
            Self::InvalidPageBytes { bytes } => {
                write!(
                    formatter,
                    "invalid heap page width for heap options: {bytes}"
                )
            }
            Self::InvalidArenaSegmentBytes { bytes } => {
                write!(
                    formatter,
                    "invalid arena segment width for heap options: {bytes}"
                )
            }
            Self::MisalignedArenaSegmentBytes {
                page_bytes,
                segment_bytes,
            } => {
                write!(
                    formatter,
                    "arena segment width violates heap page alignment: {segment_bytes} is not a multiple of {page_bytes}"
                )
            }
            Self::ArenaPageBytesMismatch {
                option_page_bytes,
                arena_page_bytes,
            } => {
                write!(
                    formatter,
                    "explicit arena page width does not match heap options: options {option_page_bytes}, arena {arena_page_bytes}"
                )
            }
            Self::ArenaSegmentBytesMismatch {
                option_segment_bytes,
                arena_segment_bytes,
            } => {
                write!(
                    formatter,
                    "explicit arena segment width does not match heap options: options {option_segment_bytes}, arena {arena_segment_bytes}"
                )
            }
            Self::ManagedYoungThresholdExceedsCapacity {
                threshold,
                capacity,
            } => {
                write!(
                    formatter,
                    "managed young allocation threshold exceeds young-space capacity: {threshold} > {capacity}"
                )
            }
            Self::InvalidCardBytes { bytes } => {
                write!(
                    formatter,
                    "invalid remembered card width for heap options: {bytes}"
                )
            }
            Self::InvalidSmallAllocationAlignmentBytes { bytes } => {
                write!(
                    formatter,
                    "invalid small-allocation alignment for heap options: {bytes}"
                )
            }
            Self::InvalidTableChunkLen { len } => {
                write!(formatter, "invalid heap table chunk length: {len}")
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
            Self::ArenaSegmentLimitExceeded {
                required_segments,
                max_segments,
            } => {
                write!(
                    formatter,
                    "arena segment limit exceeded: required {required_segments} segments with maximum {max_segments}"
                )
            }
            Self::InvalidArenaSegmentLayout {
                byte_len,
                page_bytes,
            } => {
                write!(
                    formatter,
                    "arena segment entry layout is invalid: {byte_len} bytes with alignment {page_bytes}"
                )
            }
            Self::ArenaSegmentAllocationFailed {
                byte_len,
                page_bytes,
            } => {
                write!(
                    formatter,
                    "arena segment entry failed: {byte_len} bytes with alignment {page_bytes}"
                )
            }
            Self::LimitExceeded {
                domain,
                used_bytes,
                max_bytes,
            } => {
                let subject = domain.usage_subject();

                write!(
                    formatter,
                    "{subject} limit exceeded: using {used_bytes} bytes with limit {max_bytes}"
                )
            }
            Self::CaptureGcActive => {
                write!(formatter, "heap capture requires idle gc state")
            }
            Self::CapturePinsActive => {
                write!(formatter, "heap capture requires no active managed pins")
            }
            Self::ManagedCollectionActive => {
                write!(formatter, "managed heap collection is already active")
            }
            Self::ManagedCollectionPinsActive => {
                write!(
                    formatter,
                    "managed heap collection requires no active scoped pins"
                )
            }
            Self::ManagedPinCountOverflow { reference, count } => {
                write!(
                    formatter,
                    "managed pin count overflow for reference {} at count {count}",
                    reference.id()
                )
            }
            Self::ManagedPinActiveCountOverflow { active_count } => {
                write!(
                    formatter,
                    "managed active pin count overflow at count {active_count}"
                )
            }
            Self::ManagedPinMissing { reference } => {
                write!(
                    formatter,
                    "managed reference {} is not currently pinned",
                    reference.id()
                )
            }
            Self::ManagedPinActiveCountUnderflow {
                reference,
                active_count,
            } => {
                write!(
                    formatter,
                    "managed active pin count underflow while unpinning reference {} at count {active_count}",
                    reference.id()
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
            Self::InvalidManagedReference { reference } => {
                write!(formatter, "invalid managed reference: {reference:?}")
            }
            Self::ManagedTraceFailed { source, error } => match source {
                ManagedTraceSource::Reference(reference) => {
                    write!(
                        formatter,
                        "managed heap tracing failed for reference {}: {error}",
                        reference.id()
                    )
                }
                ManagedTraceSource::Span(span_index) => {
                    write!(
                        formatter,
                        "managed heap tracing failed for dirty span {span_index}: {error}"
                    )
                }
                ManagedTraceSource::LargeEntry(entry_id) => {
                    write!(
                        formatter,
                        "managed heap tracing failed for dirty large entry {entry_id}: {error}"
                    )
                }
            },
            Self::ManagedPromotionUnavailableSmallSlot {
                reference,
                byte_len,
            } => {
                write!(
                    formatter,
                    "managed heap promotion could not allocate one mature small slot for reference {} with {} bytes",
                    reference.id(),
                    byte_len,
                )
            }
            Self::ManagedPromotionFailed { reference, error } => {
                write!(
                    formatter,
                    "managed heap promotion failed for reference {}: {error}",
                    reference.id()
                )
            }
            Self::ManagedYoungResetFailed { error } => {
                write!(formatter, "managed heap young reset failed: {error}")
            }
            Self::ManagedFreeFailed { reference, error } => {
                write!(
                    formatter,
                    "managed heap free failed for reference {}: {error}",
                    reference.id()
                )
            }
            Self::InvalidManagedReferenceId { id } => {
                write!(formatter, "invalid managed reference id: {id}")
            }
            Self::InvalidUsage {
                domain,
                allocated_count,
                allocated_bytes,
                freed_bytes,
            } => {
                let subject = domain.usage_subject();

                write!(
                    formatter,
                    "invalid {subject} usage: count {allocated_count}, bytes {allocated_bytes}, freed bytes {freed_bytes}"
                )
            }
            Self::InvalidRawPointer { pointer } => {
                write!(formatter, "invalid raw pointer: {pointer:?}")
            }
            Self::InvalidRawPointerId { id } => {
                write!(formatter, "invalid raw pointer id: {id}")
            }
            Self::InvalidSharedManagedReference { reference } => {
                write!(formatter, "invalid shared managed reference: {reference:?}")
            }
            Self::InvalidSharedManagedReferenceId { id } => {
                write!(formatter, "invalid shared managed reference id: {id}")
            }
            Self::InvalidSharedRawPointer { pointer } => {
                write!(
                    formatter,
                    "invalid shared raw pointer: id={} offset={}",
                    pointer.id(),
                    pointer.byte_offset(),
                )
            }
            Self::InvalidSharedRawPointerId { id } => {
                write!(formatter, "invalid shared raw pointer id: {id}")
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
            Self::PagePatchCapacityExceeded {
                page_count,
                patch_capacity,
            } => {
                write!(
                    formatter,
                    "heap page view with {page_count} pages exhausted inline patch capacity {patch_capacity}"
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
            Self::InvalidMapId { index } => {
                write!(formatter, "invalid reference-map id: {index}")
            }
            Self::InvalidMapSentinel { index } => {
                write!(
                    formatter,
                    "reference-map table must reserve index {index} for the empty map"
                )
            }
            Self::InvalidReferenceWindowWidth { bytes } => {
                write!(
                    formatter,
                    "unsupported managed reference width for tracing window: {bytes}"
                )
            }
            Self::ReferenceMapOffsetOverflow { start, width } => {
                write!(
                    formatter,
                    "managed reference offset overflow while tracing: start={start}, width={width}"
                )
            }
            Self::TruncatedReferenceMapPayload { start, width, len } => {
                write!(
                    formatter,
                    "truncated managed reference payload while tracing: start={start}, width={width}, len={len}"
                )
            }
            Self::TruncatedReferenceReaderWindow { start, width } => {
                write!(
                    formatter,
                    "truncated managed reference payload while tracing: start={start}, width={width}"
                )
            }
            Self::InvalidReferenceValuePayload { start } => {
                write!(
                    formatter,
                    "invalid value payload while tracing managed references: start={start}"
                )
            }
            Self::DuplicateMap { index } => {
                write!(formatter, "duplicate reference map at index {index}")
            }
            Self::InvalidMapTableLen { len } => {
                write!(formatter, "invalid reference-map table length: {len}")
            }
        }
    }
}

impl Error for HeapError {}
