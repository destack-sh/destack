use std::error::Error;
use std::fmt;

use destack_mir as mir;
use destack_program::{ContinuationImage, FrameImage, StackImage};

use crate::NativeFrameImage;

impl NativeFrameImage {
    /// Convert this frame to the durable frame image form.
    ///
    /// # Safety
    ///
    /// The byte pointer must point at `byte_len` immutable bytes for the duration of this call.
    pub unsafe fn to_frame_image(
        self,
        stack: &mut StackImage,
    ) -> Result<FrameImage, NativeMaterializationError> {
        let bytes = if self.byte_len == 0 {
            &[][..]
        } else if self.bytes.is_null() {
            return Err(NativeMaterializationError::NullFrameBytes);
        } else {
            // SAFETY: guaranteed by the caller and checked for a null pointer above
            unsafe { std::slice::from_raw_parts(self.bytes, self.byte_len) }
        };
        let stack_offset = stack.push_frame(bytes);

        Ok(FrameImage {
            frame_state: mir::FrameStateId(self.frame_state),
            return_state: self.caller_return_state(),
            stack_offset,
            byte_len: self.byte_len,
        })
    }
}

/// Native materialization passed through the native ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeMaterialization {
    /// The captured frames from outermost to innermost.
    pub frames: *const NativeFrameImage,
    /// The number of captured frames.
    pub frame_count: usize,
}

impl NativeMaterialization {
    /// Create one empty materialization.
    pub const fn empty() -> Self {
        Self {
            frames: std::ptr::null(),
            frame_count: 0,
        }
    }

    /// Return whether this materialization contains no frames.
    pub const fn is_empty(self) -> bool {
        self.frame_count == 0
    }

    /// Convert this materialization to the durable continuation image form.
    ///
    /// # Safety
    ///
    /// The frame pointer must point at `frame_count` immutable frames for the duration of this call.
    pub unsafe fn to_continuation_image(
        self,
    ) -> Result<ContinuationImage, NativeMaterializationError> {
        if self.frame_count == 0 {
            return Err(NativeMaterializationError::Empty);
        }

        if self.frames.is_null() {
            return Err(NativeMaterializationError::NullFrames);
        }

        // SAFETY: guaranteed by the caller and checked for a null pointer above
        let frames = unsafe { std::slice::from_raw_parts(self.frames, self.frame_count) };
        let mut stack = StackImage::empty();
        let mut images = Vec::with_capacity(frames.len());

        // copy each native frame into one durable stack image
        for frame in frames {
            // SAFETY: guaranteed by the caller for each frame byte range
            images.push(unsafe { frame.to_frame_image(&mut stack)? });
        }

        Ok(ContinuationImage {
            stack,
            frames: images,
        })
    }
}

impl Default for NativeMaterialization {
    fn default() -> Self {
        Self::empty()
    }
}

/// Invalid native materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeMaterializationError {
    /// No frames were captured.
    Empty,
    /// The frame pointer was null.
    NullFrames,
    /// One non-empty frame byte pointer was null.
    NullFrameBytes,
}

impl fmt::Display for NativeMaterializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "native materialization has no frames"),
            Self::NullFrames => write!(formatter, "native materialization frame pointer is null"),
            Self::NullFrameBytes => write!(formatter, "native materialized frame bytes are null"),
        }
    }
}

impl Error for NativeMaterializationError {}
