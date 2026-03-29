#[cfg(target_os = "android")]
mod backend;
mod callback;
pub(crate) mod core;

#[allow(unused_imports)]
pub(crate) use core::{callback_test_lock, register_android_bindings, register_android_runtime};
