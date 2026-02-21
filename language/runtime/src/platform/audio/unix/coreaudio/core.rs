use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::{
    ffi::{CStr, c_void},
    mem::{MaybeUninit, size_of},
    ptr, thread,
    time::Duration,
};

#[cfg(target_os = "macos")]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::{AudioStreamFlags, core as audio_core};
#[cfg(target_os = "macos")]
use crate::platform::{PlatformError, diagnostic::PlatformErrorCode};

include!("types.rs");
include!("constants.rs");
include!("ffi.rs");
include!("format.rs");
include!("property.rs");
include!("sample.rs");
include!("callback.rs");
include!("queue.rs");
include!("probe.rs");
include!("runtime.rs");
