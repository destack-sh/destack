#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]

#[cfg(not(feature = "generator"))]
pub mod diagnostic;
#[cfg(not(feature = "generator"))]
pub mod host;
#[cfg(not(feature = "generator"))]
pub mod platform;
#[cfg(not(feature = "generator"))]
pub mod runtime;
#[cfg(not(feature = "generator"))]
pub mod simulation;

#[cfg(all(not(feature = "generator"), feature = "execution"))]
#[doc(hidden)]
pub mod tests;

#[cfg(all(not(feature = "generator"), test, not(feature = "execution")))]
mod tests;
