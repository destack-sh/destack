use std::error::Error;
use std::fmt::{self, Display, Formatter};

use destack_memory::MemoryError;

use crate::allocator::PageId;
use crate::{AccountingRegion, HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};

/// One heap result.
pub type HeapResult<T> = Result<T, HeapError>;

/// Heap operation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapError {
    /// Heap configuration is invalid.
    Configuration {
        /// Configuration failure reason.
        reason: HeapConfigurationError,
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
    /// One heap reference or pointer did not resolve to one live allocation.
    InvalidReference {
        /// The reference space that failed validation.
        kind: HeapReferenceKind,
        /// The raw reference or pointer value.
        value: u64,
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
    /// One allocation request was invalid.
    InvalidAllocation {
        /// Allocation failure reason.
        reason: HeapAllocationError,
    },
    /// One encoded heap value exceeded its representation.
    Representation {
        /// Representation failure reason.
        reason: HeapRepresentationError,
    },
    /// One heap capture request was blocked by live state.
    CaptureBlocked {
        /// Capture blocker reason.
        reason: HeapCaptureBlocker,
    },
    /// One GC operation was requested in the wrong state.
    GcState {
        /// GC state failure reason.
        reason: HeapGcStateError,
    },
    /// One heap operation failed while processing a specific source.
    OperationFailed {
        /// Heap operation that failed.
        operation: HeapOperation,
        /// Heap object or region being processed.
        source: HeapOperationSource,
        /// Underlying heap failure.
        error: Box<HeapError>,
    },
    /// One lower memory operation failed.
    Memory {
        /// The lower memory failure.
        error: MemoryError,
    },
    /// One internal heap invariant failed.
    Internal {
        /// Internal error context.
        context: &'static str,
    },
}

/// Heap configuration failure reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapConfigurationError {
    /// The configured GC trigger percentage is unsupported.
    InvalidGcTriggerPercent { percent: u32 },
    /// The configured minimum GC work is unsupported.
    InvalidGcMinimumWorkBytes { bytes: usize },
    /// The configured heap page width is unsupported.
    InvalidPageSizeBytes { bytes: usize },
    /// The configured allocator chunk width is unsupported.
    InvalidAllocatorChunkSizeBytes { bytes: usize },
    /// The configured virtual heap-space width is unsupported.
    InvalidSpaceSizeBytes { bytes: usize },
    /// The configured allocator chunk width is not aligned to the page width.
    MisalignedAllocatorChunkSize {
        /// The configured page width in bytes.
        page_size_bytes: usize,
        /// The configured allocator chunk width in bytes.
        chunk_size_bytes: usize,
    },
    /// The configured virtual heap-space width is not aligned to the page width.
    MisalignedSpaceSize {
        /// The configured page width in bytes.
        page_size_bytes: usize,
        /// The configured virtual heap-space width in bytes.
        space_size_bytes: usize,
    },
    /// The explicit allocator does not match the configured heap page width.
    AllocatorPageSizeMismatch {
        /// The page width configured through heap options.
        option_page_size_bytes: usize,
        /// The actual page width of the explicit allocator.
        allocator_page_size_bytes: usize,
    },
    /// The explicit allocator does not match the configured heap chunk width.
    AllocatorChunkSizeMismatch {
        /// The chunk width configured through heap options.
        option_chunk_size_bytes: usize,
        /// The actual chunk width of the explicit allocator.
        allocator_chunk_size_bytes: usize,
    },
    /// The configured heap young-allocation threshold exceeds young-space capacity.
    YoungThresholdExceedsCapacity { threshold: usize, capacity: usize },
    /// The configured heap young-space capacity exceeds young metadata capacity.
    YoungCapacityTooLarge { capacity: usize, max: usize },
    /// The configured small-allocation alignment is unsupported.
    InvalidSmallAllocationAlignmentBytes { bytes: usize },
    /// One size class violated the configured small-allocation alignment.
    MisalignedSizeClass {
        /// The required alignment in bytes.
        alignment_bytes: usize,
        /// The misaligned size class in bytes.
        class_bytes: usize,
    },
    /// The configured size-class table is invalid.
    InvalidSizeClassTable {
        /// Size-class table failure reason.
        reason: SizeClassTableError,
    },
    /// The configured size-class generation policy is invalid.
    InvalidSizeClassPolicy {
        /// Size-class policy failure reason.
        reason: SizeClassPolicyError,
    },
    /// One restored or requested size class does not exist in the configured table.
    InvalidSizeClass {
        /// The invalid size class in bytes.
        class_bytes: usize,
    },
    /// The configured small span is too small for the largest size class.
    SmallSpanTooSmall {
        /// The configured span width in bytes.
        span_size_bytes: usize,
        /// The largest configured size class in bytes.
        class_bytes: usize,
    },
}

/// Size-class table validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeClassTableError {
    /// The configured size-class table was empty.
    Empty,
    /// One configured size class was zero.
    ZeroSizeClass,
    /// The configured size classes were not strictly increasing.
    NonMonotonic {
        /// The previous size class in bytes.
        previous: usize,
        /// The current size class in bytes.
        bytes: usize,
    },
}

/// Size-class policy validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeClassPolicyError {
    /// The configured size-class policy range is invalid.
    InvalidRange {
        /// The configured minimum generated size class in bytes.
        min_bytes: usize,
        /// The configured maximum generated size class in bytes.
        max_bytes: usize,
    },
    /// The configured size-class policy alignment is invalid.
    InvalidAlignment {
        /// The configured alignment in bytes.
        alignment_bytes: usize,
    },
    /// The configured size-class policy fragmentation ratio is invalid.
    InvalidFragmentation {
        /// The configured fragmentation ratio numerator.
        numerator: usize,
        /// The configured fragmentation ratio denominator.
        denominator: usize,
    },
}

/// Heap reference space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapReferenceKind {
    /// Local managed heap reference.
    Heap,
    /// Local raw pointer.
    Raw,
    /// Shared managed heap reference.
    SharedHeap,
    /// Shared raw pointer.
    SharedRaw,
}

/// Invalid allocation request reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapAllocationError {
    /// Managed heap allocation with no bytes reached runtime.
    ZeroSize,
    /// One allocation initializer did not match its requested byte length.
    ByteLengthMismatch {
        /// The expected byte length.
        expected: usize,
        /// The actual byte length requested by the caller.
        actual: usize,
    },
}

/// Heap representation failure reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapRepresentationError {
    /// One heap value exceeded its encoded representation.
    LimitExceeded {
        /// The representation that was exceeded.
        context: &'static str,
    },
    /// One allocator image exceeded the encoded chunk range.
    AllocatorChunkLimitExceeded {
        /// The chunks required by the allocator image.
        required_chunks: usize,
        /// The maximum chunks representable by the allocator image.
        max_chunks: usize,
    },
    /// One live large-allocation id cannot be represented.
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
    /// One decoded heap-reference window has an unsupported width.
    InvalidReferenceWindowWidth {
        /// The invalid byte width.
        bytes: usize,
    },
    /// One traced field did not fit inside the provided byte window.
    TruncatedReferenceBytes {
        /// The traced field byte offset.
        start: usize,
        /// The traced field byte width.
        width: usize,
    },
}

/// Heap capture blocker reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapCaptureBlocker {
    /// Active collector work blocks capture.
    GcActive,
    /// Active pins block capture.
    PinsActive,
}

/// GC state failure reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapGcStateError {
    /// One local GC was requested while another local GC was active.
    LocalGcActive,
    /// One shared GC was requested while another shared GC was active.
    SharedGcActive,
    /// One shared GC mark operation was requested while shared mark was inactive.
    SharedGcNotMarking,
    /// One shared GC sweep operation was requested while shared sweep was inactive.
    SharedGcNotSweeping,
    /// One heap reference was unpinned without one active scoped pin.
    PinMissing {
        /// The unpinned heap reference.
        reference: HeapReference,
    },
}

/// Heap operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapOperation {
    /// Scan heap references.
    Scan,
    /// Free one heap allocation.
    Free,
}

/// Heap operation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapOperationSource {
    /// One heap reference payload.
    Reference(HeapReference),
    /// One mature span.
    Span(usize),
    /// One mature large allocation.
    LargeAllocation(u64),
}

impl HeapError {
    /// Return one configuration error.
    pub const fn configuration(reason: HeapConfigurationError) -> Self {
        Self::Configuration { reason }
    }

    /// Return one invalid heap reference error.
    pub const fn invalid_heap_reference(reference: HeapReference) -> Self {
        Self::InvalidReference {
            kind: HeapReferenceKind::Heap,
            value: reference.offset() as u64,
        }
    }

    /// Return one invalid raw pointer error.
    pub const fn invalid_raw_pointer(pointer: RawPointer) -> Self {
        Self::InvalidReference {
            kind: HeapReferenceKind::Raw,
            value: pointer.offset() as u64,
        }
    }

    /// Return one invalid shared heap reference error.
    pub const fn invalid_shared_heap_reference(reference: SharedHeapReference) -> Self {
        Self::InvalidReference {
            kind: HeapReferenceKind::SharedHeap,
            value: reference.offset() as u64,
        }
    }

    /// Return one invalid shared raw pointer error.
    pub const fn invalid_shared_raw_pointer(pointer: SharedRawPointer) -> Self {
        Self::InvalidReference {
            kind: HeapReferenceKind::SharedRaw,
            value: pointer.offset() as u64,
        }
    }

    /// Return one invalid allocation error.
    pub const fn invalid_allocation(reason: HeapAllocationError) -> Self {
        Self::InvalidAllocation { reason }
    }

    /// Return one representation error.
    pub const fn representation(reason: HeapRepresentationError) -> Self {
        Self::Representation { reason }
    }

    /// Return one capture blocker error.
    pub const fn capture_blocked(reason: HeapCaptureBlocker) -> Self {
        Self::CaptureBlocked { reason }
    }

    /// Return one GC state error.
    pub const fn gc_state(reason: HeapGcStateError) -> Self {
        Self::GcState { reason }
    }

    /// Return one operation failure.
    pub fn operation_failed(
        operation: HeapOperation,
        source: HeapOperationSource,
        error: HeapError,
    ) -> Self {
        Self::OperationFailed {
            operation,
            source,
            error: Box::new(error),
        }
    }

    /// Return one scan failure.
    pub fn scan_failed(source: HeapOperationSource, error: HeapError) -> Self {
        Self::operation_failed(HeapOperation::Scan, source, error)
    }

    /// Return one free failure.
    pub fn free_failed(reference: HeapReference, error: HeapError) -> Self {
        Self::operation_failed(
            HeapOperation::Free,
            HeapOperationSource::Reference(reference),
            error,
        )
    }

    /// Return one internal invariant error.
    pub const fn internal(context: &'static str) -> Self {
        Self::Internal { context }
    }
}

impl Display for HeapError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration { reason } => {
                write!(formatter, "invalid heap configuration: {reason}")
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
            Self::InvalidReference { kind, value } => {
                write!(formatter, "invalid {kind} reference: {value}")
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
            Self::InvalidAllocation { reason } => {
                write!(formatter, "invalid heap allocation: {reason}")
            }
            Self::Representation { reason } => {
                write!(formatter, "heap representation error: {reason}")
            }
            Self::CaptureBlocked { reason } => write!(formatter, "heap capture blocked: {reason}"),
            Self::GcState { reason } => write!(formatter, "invalid heap gc state: {reason}"),
            Self::OperationFailed {
                operation,
                source,
                error,
            } => {
                write!(formatter, "heap {operation} failed for {source}: {error}")
            }
            Self::Memory { error } => write!(formatter, "memory operation failed: {error}"),
            Self::Internal { context } => write!(formatter, "internal heap error: {context}"),
        }
    }
}

impl Display for HeapConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGcTriggerPercent { percent } => {
                write!(formatter, "invalid gc trigger percent: {percent}")
            }
            Self::InvalidGcMinimumWorkBytes { bytes } => {
                write!(formatter, "invalid minimum gc work bytes: {bytes}")
            }
            Self::InvalidPageSizeBytes { bytes } => {
                write!(formatter, "invalid heap page width: {bytes}")
            }
            Self::InvalidAllocatorChunkSizeBytes { bytes } => {
                write!(formatter, "invalid allocator chunk width: {bytes}")
            }
            Self::InvalidSpaceSizeBytes { bytes } => {
                write!(formatter, "invalid virtual heap-space width: {bytes}")
            }
            Self::MisalignedAllocatorChunkSize {
                page_size_bytes,
                chunk_size_bytes,
            } => {
                write!(
                    formatter,
                    "allocator chunk width {chunk_size_bytes} is not aligned to page width {page_size_bytes}"
                )
            }
            Self::MisalignedSpaceSize {
                page_size_bytes,
                space_size_bytes,
            } => {
                write!(
                    formatter,
                    "virtual heap-space width {space_size_bytes} is not aligned to page width {page_size_bytes}"
                )
            }
            Self::AllocatorPageSizeMismatch {
                option_page_size_bytes,
                allocator_page_size_bytes,
            } => {
                write!(
                    formatter,
                    "allocator page width mismatch: options {option_page_size_bytes}, allocator {allocator_page_size_bytes}"
                )
            }
            Self::AllocatorChunkSizeMismatch {
                option_chunk_size_bytes,
                allocator_chunk_size_bytes,
            } => {
                write!(
                    formatter,
                    "allocator chunk width mismatch: options {option_chunk_size_bytes}, allocator {allocator_chunk_size_bytes}"
                )
            }
            Self::YoungThresholdExceedsCapacity {
                threshold,
                capacity,
            } => {
                write!(
                    formatter,
                    "young allocation threshold {threshold} exceeds capacity {capacity}"
                )
            }
            Self::YoungCapacityTooLarge { capacity, max } => {
                write!(
                    formatter,
                    "young-space capacity {capacity} exceeds max {max}"
                )
            }
            Self::InvalidSmallAllocationAlignmentBytes { bytes } => {
                write!(formatter, "invalid small-allocation alignment: {bytes}")
            }
            Self::MisalignedSizeClass {
                alignment_bytes,
                class_bytes,
            } => {
                write!(
                    formatter,
                    "size class {class_bytes} is not aligned to {alignment_bytes}"
                )
            }
            Self::InvalidSizeClassTable { reason } => {
                write!(formatter, "invalid size-class table: {reason}")
            }
            Self::InvalidSizeClassPolicy { reason } => {
                write!(formatter, "invalid size-class policy: {reason}")
            }
            Self::InvalidSizeClass { class_bytes } => {
                write!(formatter, "unknown size class: {class_bytes}")
            }
            Self::SmallSpanTooSmall {
                span_size_bytes,
                class_bytes,
            } => {
                write!(
                    formatter,
                    "small span {span_size_bytes} is smaller than size class {class_bytes}"
                )
            }
        }
    }
}

impl Display for SizeClassTableError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "empty table"),
            Self::ZeroSizeClass => write!(formatter, "zero-byte size class"),
            Self::NonMonotonic { previous, bytes } => {
                write!(formatter, "{bytes} follows {previous}")
            }
        }
    }
}

impl Display for SizeClassPolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange {
                min_bytes,
                max_bytes,
            } => {
                write!(formatter, "invalid range: min {min_bytes}, max {max_bytes}")
            }
            Self::InvalidAlignment { alignment_bytes } => {
                write!(formatter, "invalid alignment: {alignment_bytes}")
            }
            Self::InvalidFragmentation {
                numerator,
                denominator,
            } => {
                write!(
                    formatter,
                    "invalid fragmentation ratio: {numerator}/{denominator}"
                )
            }
        }
    }
}

impl Display for HeapReferenceKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Heap => "heap",
            Self::Raw => "raw",
            Self::SharedHeap => "shared heap",
            Self::SharedRaw => "shared raw",
        };

        write!(formatter, "{name}")
    }
}

impl Display for HeapAllocationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSize => write!(formatter, "zero-size managed allocation"),
            Self::ByteLengthMismatch { expected, actual } => {
                write!(formatter, "expected {expected} bytes, got {actual}")
            }
        }
    }
}

impl Display for HeapRepresentationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded { context } => {
                write!(formatter, "representation limit exceeded: {context}")
            }
            Self::AllocatorChunkLimitExceeded {
                required_chunks,
                max_chunks,
            } => {
                write!(
                    formatter,
                    "allocator chunk limit exceeded: required {required_chunks}, max {max_chunks}"
                )
            }
            Self::InvalidLargeAllocationId { id } => {
                write!(formatter, "invalid large-allocation id: {id}")
            }
            Self::InvalidPageId { index } => {
                write!(formatter, "invalid page id index: {index}")
            }
            Self::InvalidPageRun {
                first_page,
                page_count,
            } => {
                write!(
                    formatter,
                    "invalid page run: first page {first_page:?}, page count {page_count}"
                )
            }
            Self::InvalidSmallSlot {
                span_index,
                slot_index,
            } => {
                write!(
                    formatter,
                    "invalid small slot: span {span_index}, slot {slot_index}"
                )
            }
            Self::InvalidReferenceWindowWidth { bytes } => {
                write!(formatter, "invalid reference window width: {bytes}")
            }
            Self::TruncatedReferenceBytes { start, width } => {
                write!(
                    formatter,
                    "truncated reference bytes: start {start}, width {width}"
                )
            }
        }
    }
}

impl Display for HeapCaptureBlocker {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::GcActive => "gc active",
            Self::PinsActive => "pins active",
        };

        write!(formatter, "{reason}")
    }
}

impl Display for HeapGcStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalGcActive => write!(formatter, "local gc already active"),
            Self::SharedGcActive => write!(formatter, "shared gc already active"),
            Self::SharedGcNotMarking => write!(formatter, "shared gc not marking"),
            Self::SharedGcNotSweeping => write!(formatter, "shared gc not sweeping"),
            Self::PinMissing { reference } => write!(formatter, "pin missing for {reference:?}"),
        }
    }
}

impl Display for HeapOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let operation = match self {
            Self::Scan => "scan",
            Self::Free => "free",
        };

        write!(formatter, "{operation}")
    }
}

impl Display for HeapOperationSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reference(reference) => write!(formatter, "reference {reference:?}"),
            Self::Span(span_index) => write!(formatter, "span {span_index}"),
            Self::LargeAllocation(allocation_id) => {
                write!(formatter, "large allocation {allocation_id}")
            }
        }
    }
}

impl Error for HeapError {}

impl From<MemoryError> for HeapError {
    fn from(error: MemoryError) -> Self {
        match error {
            MemoryError::InvalidByteRange {
                start,
                len,
                capacity,
            } => Self::InvalidByteRange {
                start,
                len,
                capacity,
            },
            MemoryError::Internal { context } => Self::Internal { context },
            error => Self::Memory { error },
        }
    }
}
