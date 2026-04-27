use std::error::Error;
use std::fmt::{self, Display, Formatter};

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
    /// One mature large allocation.
    LargeAllocation(u64),
}

/// Heap configuration failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapError {
    /// The configured GC trigger percentage is unsupported.
    InvalidGcTriggerPercent { percent: u32 },
    /// The configured heap page width is unsupported.
    InvalidPageBytes { bytes: usize },
    /// The configured allocator chunk width is unsupported.
    InvalidAllocatorChunkBytes { bytes: usize },
    /// The configured virtual heap-space width is unsupported.
    InvalidSpaceBytes {
        /// The configured virtual heap-space width in bytes.
        bytes: usize,
    },
    /// The configured allocator chunk width is not aligned to the page width.
    MisalignedAllocatorChunkBytes {
        /// The configured page width in bytes.
        page_bytes: usize,
        /// The configured allocator chunk width in bytes.
        chunk_bytes: usize,
    },
    /// The configured virtual heap-space width is not aligned to the page width.
    MisalignedSpaceBytes {
        /// The configured page width in bytes.
        page_bytes: usize,
        /// The configured virtual heap-space width in bytes.
        space_bytes: usize,
    },
    /// The explicit allocator does not match the configured heap page width.
    AllocatorPageBytesMismatch {
        /// The page width configured through heap options.
        option_page_bytes: usize,
        /// The actual page width of the explicit allocator.
        allocator_page_bytes: usize,
    },
    /// The explicit allocator does not match the configured heap chunk width.
    AllocatorChunkBytesMismatch {
        /// The chunk width configured through heap options.
        option_chunk_bytes: usize,
        /// The actual chunk width of the explicit allocator.
        allocator_chunk_bytes: usize,
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
    /// The configured small span is too small for the largest size class.
    SmallSpanTooSmall {
        /// The configured span width in bytes.
        span_bytes: usize,
        /// The largest configured size class in bytes.
        class_bytes: usize,
    },
    /// The allocator chunk count cannot grow far enough for one allocation.
    AllocatorChunkLimitExceeded {
        /// The required chunk count.
        required_chunks: usize,
        /// The maximum configured chunk count.
        max_chunks: usize,
    },
    /// One allocator chunk allocation failed.
    AllocatorChunkAllocationFailed {
        /// The requested chunk byte length.
        byte_len: usize,
    },
    /// One address-space operation failed.
    AddressSpaceFailed {
        /// The requested address space byte length.
        byte_len: usize,
    },
    /// One allocator address was outside the supported chunk map.
    AllocatorAddressUnsupported {
        /// The mapped chunk address.
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
    /// One heap reference was unpinned without one active scoped pin.
    HeapPinMissing {
        /// The unpinned heap reference.
        reference: HeapReference,
    },
    /// One heap byte range was outside the logical allocation.
    InvalidByteRange {
        /// The requested byte offset.
        start: usize,
        /// The requested byte length.
        len: usize,
        /// The logical allocation capacity in bytes.
        capacity: usize,
    },
    /// One heap reference did not resolve to one live allocation.
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
    /// One raw pointer did not resolve to one live allocation.
    InvalidRawPointer {
        /// The invalid raw pointer.
        pointer: RawPointer,
    },
    /// One shared heap reference did not resolve to one live allocation.
    InvalidSharedHeapReference {
        /// The invalid shared heap reference.
        reference: SharedHeapReference,
    },
    /// One shared raw pointer did not resolve to one live allocation.
    InvalidSharedRawPointer {
        /// The invalid shared raw pointer.
        pointer: SharedRawPointer,
    },
    /// One internal heap invariant exceeded representable arithmetic range.
    InvariantOverflow {
        /// The overflowing invariant context.
        context: &'static str,
    },
    /// One internal heap invariant was violated.
    InvariantViolation {
        /// The violated invariant context.
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
    /// One live small slot disappeared from its owning span.
    MissingSmallSlot {
        /// The missing span index.
        span_index: usize,
        /// The missing slot index.
        slot_index: usize,
    },
    /// One live young range disappeared before initialization or access.
    MissingYoungRange {
        /// The missing young range base offset.
        first_offset: usize,
    },
    /// One live large allocation disappeared before access.
    MissingLargeAllocation {
        /// The missing large-allocation identifier.
        allocation_id: u64,
    },
    /// One large-allocation id cannot be represented by large-allocation tables.
    InvalidLargeAllocationId {
        /// The invalid large-allocation id.
        id: u64,
    },
    /// One page identifier exceeded the encoded allocator page range.
    InvalidPageId {
        /// The invalid page index.
        index: usize,
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
    /// One dense copy-on-write table entry was missing unexpectedly.
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
    /// One allocation initializer did not match its requested byte length.
    InvalidAllocationBytes {
        /// The expected byte length.
        expected: usize,
        /// The actual byte length requested by the caller.
        actual: usize,
    },
    /// Managed heap allocation with no bytes reached runtime.
    ZeroSizeAllocation,
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
            Self::InvalidAllocatorChunkBytes { bytes } => {
                write!(
                    formatter,
                    "invalid allocator chunk width for heap options: {bytes}"
                )
            }
            Self::InvalidSpaceBytes { bytes } => {
                write!(
                    formatter,
                    "invalid virtual heap-space width for heap options: {bytes}"
                )
            }
            Self::MisalignedAllocatorChunkBytes {
                page_bytes,
                chunk_bytes,
            } => {
                write!(
                    formatter,
                    "allocator chunk width violates heap page alignment: {chunk_bytes} is not a multiple of {page_bytes}"
                )
            }
            Self::MisalignedSpaceBytes {
                page_bytes,
                space_bytes,
            } => {
                write!(
                    formatter,
                    "virtual heap-space width violates heap page alignment: {space_bytes} is not a multiple of {page_bytes}"
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
            Self::AllocatorChunkBytesMismatch {
                option_chunk_bytes,
                allocator_chunk_bytes,
            } => {
                write!(
                    formatter,
                    "explicit allocator chunk width does not match heap options: options {option_chunk_bytes}, allocator {allocator_chunk_bytes}"
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
            Self::SmallSpanTooSmall {
                span_bytes,
                class_bytes,
            } => {
                write!(
                    formatter,
                    "small span is smaller than the largest size class: {span_bytes} < {class_bytes}"
                )
            }
            Self::AllocatorChunkLimitExceeded {
                required_chunks,
                max_chunks,
            } => {
                write!(
                    formatter,
                    "allocator chunk limit exceeded: required {required_chunks} chunks with maximum {max_chunks}"
                )
            }
            Self::AllocatorChunkAllocationFailed { byte_len } => {
                write!(
                    formatter,
                    "allocator chunk allocation failed: {byte_len} bytes"
                )
            }
            Self::AddressSpaceFailed { byte_len } => {
                write!(
                    formatter,
                    "address space operation failed: {byte_len} bytes"
                )
            }
            Self::AllocatorAddressUnsupported { address } => {
                write!(
                    formatter,
                    "allocator chunk address is outside the supported chunk map: {address:#x}"
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
            Self::HeapPinMissing { reference } => {
                write!(
                    formatter,
                    "heap reference {reference:?} is not currently pinned"
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
                ScanSource::LargeAllocation(allocation_id) => {
                    write!(
                        formatter,
                        "heap scan failed for dirty large allocation {allocation_id}: {error}"
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
            Self::InvalidRawPointer { pointer } => {
                write!(formatter, "invalid raw pointer: {pointer:?}")
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
            Self::InvariantViolation { context } => {
                write!(formatter, "heap invariant violation: {context}")
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
            Self::MissingYoungRange { first_offset } => {
                write!(
                    formatter,
                    "heap lost live young range at byte offset {first_offset}"
                )
            }
            Self::MissingLargeAllocation { allocation_id } => {
                write!(
                    formatter,
                    "heap lost live large allocation with id {allocation_id}"
                )
            }
            Self::InvalidLargeAllocationId { id } => {
                write!(formatter, "invalid large-allocation id: {id}")
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
            Self::InvalidAllocationBytes { expected, actual } => {
                write!(
                    formatter,
                    "allocation bytes do not match requested length: expected {expected}, got {actual}"
                )
            }
            Self::ZeroSizeAllocation => {
                write!(
                    formatter,
                    "zero-size managed heap allocation reached runtime"
                )
            }
        }
    }
}

impl Error for HeapError {}
