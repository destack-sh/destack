#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]

#[cfg(not(feature = "generate_bindings"))]
pub mod diagnostic;
#[cfg(not(feature = "generate_bindings"))]
pub mod host;
#[cfg(not(feature = "generate_bindings"))]
pub mod platform;
#[cfg(not(feature = "generate_bindings"))]
pub mod runtime;
#[cfg(not(feature = "generate_bindings"))]
pub mod simulation;

#[cfg(test)]
mod tests;
