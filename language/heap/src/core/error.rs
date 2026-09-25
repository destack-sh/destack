use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use tspp_memory::MemoryError;

use crate::{AccountingRegion, HeapReference, SharedHeapReference, TraceTableError};

/// One heap result.
pub type HeapResult<T> = Result<T, HeapError>;

/// Heap operation failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// One heap reference or pointer did not resolve to one live block.
    InvalidReference {
        /// The reference space that failed validation.
        kind: HeapReferenceKind,
        /// The raw reference or pointer value.
        value: u64,
    },
    /// One heap byte range was outside the logical block.
    InvalidByteRange {
        /// The requested byte offset.
        start: usize,
        /// The requested byte length.
        len: usize,
        /// The logical block capacity in bytes.
        capacity: usize,
    },
    /// One block request was invalid.
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
    /// One heap trace table operation failed.
    Trace {
        /// The trace table failure.
        error: TraceTableError,
    },
    /// One lower memory operation failed.
    Memory {
        /// The lower memory failure.
        error: MemoryError,
    },
    /// One internal heap invariant failed.
    Internal {
        /// Internal error context.
        context: String,
    },
}

/// Heap configuration failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapConfigurationError {
    /// The configured GC trigger percentage is unsupported.
    InvalidGcTriggerPercent { percent: u32 },
    /// The configured minimum GC work is unsupported.
    InvalidGcMinimumWorkBytes { bytes: usize },
    /// The configured heap page width is unsupported.
    InvalidPageSizeBytes { bytes: usize },
    /// The configured small-block alignment is unsupported.
    InvalidSmallAllocationAlignmentBytes { bytes: usize },
    /// One size class violated the configured small-block alignment.
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapReferenceKind {
    /// Local managed heap reference.
    Heap,
    /// Shared managed heap reference.
    SharedHeap,
}

/// Invalid block request reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapAllocationError {
    /// Managed heap block with no bytes reached runtime.
    ZeroSize,
    /// One block initializer did not match its requested byte length.
    ByteLengthMismatch {
        /// The expected byte length.
        expected: usize,
        /// The actual byte length requested by the caller.
        actual: usize,
    },
}

/// Heap representation failure reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapRepresentationError {
    /// One heap value exceeded its encoded representation.
    LimitExceeded {
        /// The representation that was exceeded.
        context: String,
    },
    /// One live large-block id cannot be represented.
    InvalidLargeBlockId {
        /// The invalid large-block id.
        id: u64,
    },
    /// One span slot exceeded the encoded small-space slot range.
    InvalidSlot {
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
    /// One Drop plan does not fit its allocation byte length.
    InvalidDropLayout {
        /// The allocation byte length.
        byte_len: usize,
        /// The planned value stride.
        stride: usize,
    },
}

/// Heap capture blocker reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapCaptureBlocker {
    /// Active collector work blocks capture.
    GcActive,
}

/// GC state failure reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapGcStateError {
    /// One local GC was requested while another local GC was active.
    LocalGcActive,
    /// One shared GC was requested while another shared GC was active.
    SharedGcActive,
    /// One shared GC mark operation was requested while shared mark was inactive.
    SharedGcNotMarking,
    /// One shared GC Drop operation was requested while shared Drop was inactive.
    SharedGcNotDropping,
    /// One shared GC sweep operation was requested while shared sweep was inactive.
    SharedGcNotSweeping,
}

/// Heap operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapOperation {
    /// Scan heap references.
    Scan,
    /// Free one heap block.
    Free,
}

/// Heap operation source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapOperationSource {
    /// One heap reference payload.
    Reference(HeapReference),
    /// One mature span.
    Span(usize),
    /// One mature large block.
    LargeBlock(u64),
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

    /// Return one invalid shared heap reference error.
    pub const fn invalid_shared_heap_reference(reference: SharedHeapReference) -> Self {
        Self::InvalidReference {
            kind: HeapReferenceKind::SharedHeap,
            value: reference.offset() as u64,
        }
    }

    /// Return one invalid block error.
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
    pub fn internal(context: impl Into<String>) -> Self {
        Self::Internal {
            context: context.into(),
        }
    }
}

impl HeapRepresentationError {
    /// Return one representation limit error.
    pub fn limit_exceeded(context: impl Into<String>) -> Self {
        Self::LimitExceeded {
            context: context.into(),
        }
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
                write!(formatter, "invalid heap block: {reason}")
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
            Self::Trace { error } => write!(formatter, "invalid heap trace table: {error}"),
            Self::Memory { error } => write!(formatter, "memory operation failed: {error}"),
            Self::Internal { context } => write!(formatter, "internal heap error: {context}"),
        }
    }
}

impl From<TraceTableError> for HeapError {
    /// Convert one trace table error.
    fn from(error: TraceTableError) -> Self {
        Self::Trace { error }
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
            Self::InvalidSmallAllocationAlignmentBytes { bytes } => {
                write!(formatter, "invalid small-block alignment: {bytes}")
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
            Self::SharedHeap => "shared heap",
        };

        write!(formatter, "{name}")
    }
}

impl Display for HeapAllocationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSize => write!(formatter, "zero-size managed block"),
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
            Self::InvalidLargeBlockId { id } => {
                write!(formatter, "invalid large-block id: {id}")
            }
            Self::InvalidSlot {
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
            Self::InvalidDropLayout { byte_len, stride } => {
                write!(
                    formatter,
                    "invalid Drop layout: byte length {byte_len}, stride {stride}"
                )
            }
        }
    }
}

impl Display for HeapCaptureBlocker {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let reason = match self {
            Self::GcActive => "gc active",
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
            Self::SharedGcNotDropping => write!(formatter, "shared gc not dropping"),
            Self::SharedGcNotSweeping => write!(formatter, "shared gc not sweeping"),
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
            Self::LargeBlock(block_id) => {
                write!(formatter, "large block {block_id}")
            }
        }
    }
}

impl Error for HeapError {
    /// Return the underlying subsystem failure when present.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OperationFailed { error, .. } => Some(error.as_ref()),
            Self::Trace { error } => Some(error),
            Self::Memory { error } => Some(error),
            _ => None,
        }
    }
}

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
