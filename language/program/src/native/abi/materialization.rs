use std::error::Error;
use std::fmt;

use super::{NativeFrameImage, NativeFrameImageError};
use crate::{ContinuationImage, StackImage};

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
            let image = unsafe { frame.to_frame_image(&mut stack) }
                .map_err(NativeMaterializationError::Frame)?;

            images.push(image);
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
    /// One native frame image could not be decoded.
    Frame(NativeFrameImageError),
}

impl fmt::Display for NativeMaterializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "native materialization has no frames"),
            Self::NullFrames => write!(formatter, "native materialization frame pointer is null"),
            Self::Frame(error) => write!(formatter, "native materialization frame error: {error}"),
        }
    }
}

impl Error for NativeMaterializationError {}
