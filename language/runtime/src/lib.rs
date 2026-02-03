#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(clippy::missing_safety_doc)]

#[cfg(not(feature = "generate_bindings"))]
pub mod diagnostic;
#[cfg(not(feature = "generate_bindings"))]
pub mod engine;
#[cfg(not(feature = "generate_bindings"))]
pub mod memory;
#[cfg(not(feature = "generate_bindings"))]
pub mod platform;
#[cfg(not(feature = "generate_bindings"))]
pub mod random;
#[cfg(not(feature = "generate_bindings"))]
pub mod replay;
#[cfg(not(feature = "generate_bindings"))]
pub mod runtime;
#[cfg(not(feature = "generate_bindings"))]
pub mod scheduler;
#[cfg(not(feature = "generate_bindings"))]
pub mod snapshot;
#[cfg(not(feature = "generate_bindings"))]
pub mod time;
