//! core/encoding/kompakt@2025.08.15.1

#![destack::partial(core/encoding/kompakt, file)]

pub use encoder::*;
pub use core::*;
pub use generate::*;

mod encoder;
mod core;
mod generate;