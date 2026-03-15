#[cfg(target_os = "android")]
mod backend;
mod callback;
pub(crate) mod core;

pub(crate) use core::{
    callback_test_lock, register_android_bindings, register_android_bindings_credentials,
    register_android_bindings_crypto, register_android_bindings_midi, register_android_runtime,
};
