use std::error::Error;
use std::fmt;

use destack_mir as mir;

use crate::{Continuation, FrameImage};

/// Native frame captured with one continuation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeFrameImage {
    /// The captured frame state.
    pub frame_state: u32,
    /// Whether the caller return frame state is present.
    pub return_state_is_present: u32,
    /// The caller return frame state when present.
    pub return_state: u32,
    /// The captured frame bytes.
    pub bytes: *const u8,
    /// The captured frame byte length.
    pub byte_len: usize,
}

/// Native continuation image passed through the native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeContinuation {
    /// The captured native frames from outermost to innermost.
    pub frames: *const NativeFrameImage,
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

    /// Convert this ABI continuation to owned native continuation state.
    ///
    /// # Safety
    ///
    /// The frame pointer must point at `frame_count` immutable frames for the duration of this call.
    pub unsafe fn to_native(self) -> Result<Continuation, NativeContinuationError> {
        if self.frame_count == 0 {
            return Err(NativeContinuationError::Empty);
        }

        if self.frames.is_null() {
            return Err(NativeContinuationError::NullFrames);
        }

        // SAFETY: guaranteed by the caller and checked for a null pointer above
        let frames = unsafe { std::slice::from_raw_parts(self.frames, self.frame_count) };
        let mut images = Vec::with_capacity(frames.len());

        // copy each native frame into owned bytes
        for frame in frames {
            images.push(unsafe { frame.to_native()? });
        }

        Ok(Continuation::new(images))
    }
}

impl NativeFrameImage {
    /// Return the caller return frame state.
    pub const fn caller_return_state(self) -> Option<mir::FrameStateId> {
        if self.return_state_is_present == 0 {
            None
        } else {
            Some(mir::FrameStateId(self.return_state))
        }
    }

    /// Convert this ABI frame to owned native frame state.
    ///
    /// # Safety
    ///
    /// The byte pointer must point at `byte_len` immutable bytes for the duration of this call.
    pub unsafe fn to_native(self) -> Result<FrameImage, NativeContinuationError> {
        let bytes = if self.byte_len == 0 {
            Vec::new()
        } else if self.bytes.is_null() {
            return Err(NativeContinuationError::NullFrameBytes);
        } else {
            // SAFETY: guaranteed by the caller and checked for a null pointer above
            unsafe { std::slice::from_raw_parts(self.bytes, self.byte_len) }.to_vec()
        };

        Ok(FrameImage {
            frame_state: mir::FrameStateId(self.frame_state),
            return_state: self.caller_return_state(),
            bytes,
        })
    }
}

impl Default for NativeContinuation {
    fn default() -> Self {
        Self::empty()
    }
}

/// Invalid native continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeContinuationError {
    /// No native state was captured.
    Empty,
    /// The native frame pointer was null.
    NullFrames,
    /// One non-empty frame byte pointer was null.
    NullFrameBytes,
}

impl fmt::Display for NativeContinuationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "native continuation has no state"),
            Self::NullFrames => write!(formatter, "native continuation frame pointer is null"),
            Self::NullFrameBytes => write!(formatter, "native continuation frame bytes are null"),
        }
    }
}

impl Error for NativeContinuationError {}
