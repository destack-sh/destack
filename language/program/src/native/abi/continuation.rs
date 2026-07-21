use std::error::Error;
use std::fmt;

use destack_core::Optional;

use crate::{Continuation, ContinuationBuilder, FrameStateId};

/// Native frame captured with one continuation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeFrame {
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The caller normal frame state.
    pub normal_state: Optional<FrameStateId>,
    /// The caller unwind frame state.
    pub unwind_state: Optional<FrameStateId>,
    /// The captured frame bytes in durable continuation encoding.
    pub bytes: *const u8,
    /// The captured frame byte length.
    pub byte_len: usize,
}

/// Native continuation passed through the native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeContinuation {
    /// The captured native frames from outermost to innermost.
    pub frames: *const NativeFrame,
    /// The number of captured native frames.
    pub frame_count: usize,
}

impl NativeContinuation {
    /// Create one empty native continuation.
    pub const fn empty() -> Self {
        Self {
            frames: std::ptr::null(),
            frame_count: 0,
        }
    }

    /// Return whether this continuation contains no native state.
    pub const fn is_empty(self) -> bool {
        self.frame_count == 0
    }

    /// Convert this ABI continuation to durable continuation state.
    ///
    /// # Safety
    ///
    /// The frame pointer must point at `frame_count` immutable frames for the duration of this call.
    pub unsafe fn to_continuation(self) -> Result<Continuation, NativeContinuationError> {
        // reject continuations without captured frames
        if self.frame_count == 0 {
            return Err(NativeContinuationError::Empty);
        }

        // reject non-empty continuations without frame storage
        if self.frames.is_null() {
            return Err(NativeContinuationError::NullFrames);
        }

        // SAFETY: guaranteed by the caller and checked for a null pointer above
        let frames = unsafe { std::slice::from_raw_parts(self.frames, self.frame_count) };
        let mut continuation = ContinuationBuilder::new();

        // copy native frame bytes into the continuation byte store
        for frame in frames {
            unsafe { frame.push(&mut continuation) }.map_err(NativeContinuationError::Frame)?;
        }

        Ok(continuation.build())
    }
}

impl NativeFrame {
    /// Convert this ABI frame to durable frame state.
    ///
    /// # Safety
    ///
    /// The byte pointer must point at `byte_len` immutable bytes for the duration of this call.
    pub unsafe fn push(
        self,
        continuation: &mut ContinuationBuilder,
    ) -> Result<(), NativeFrameError> {
        // accept empty frames without dereferencing their byte pointer
        let bytes = if self.byte_len == 0 {
            &[][..]
        }
        // reject non-empty frames without byte storage
        else if self.bytes.is_null() {
            return Err(NativeFrameError::NullFrameBytes);
        }
        // borrow the frame bytes for immediate copying
        else {
            // SAFETY: guaranteed by the caller and checked for a null pointer above
            unsafe { std::slice::from_raw_parts(self.bytes, self.byte_len) }
        };

        // append this frame to the durable continuation
        continuation.push(
            self.frame_state,
            self.normal_state.get(),
            self.unwind_state.get(),
            bytes,
        );

        Ok(())
    }
}

/// Invalid native frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFrameError {
    /// One non-empty frame byte pointer was null.
    NullFrameBytes,
}

impl fmt::Display for NativeFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullFrameBytes => write!(formatter, "native frame bytes are null"),
        }
    }
}

impl Error for NativeFrameError {}

/// Invalid native continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeContinuationError {
    /// No native state was captured.
    Empty,
    /// The native frame pointer was null.
    NullFrames,
    /// One native frame could not be decoded.
    Frame(NativeFrameError),
}

impl fmt::Display for NativeContinuationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "native continuation has no state"),
            Self::NullFrames => write!(formatter, "native continuation frame pointer is null"),
            Self::Frame(error) => write!(formatter, "native continuation frame error: {error}"),
        }
    }
}

impl Error for NativeContinuationError {}
