#![feature(default_field_values)]
#![feature(if_let_guard)]

pub mod machine;
pub use machine::*;

#[cfg(feature = "bun")]
pub mod bun;
#[cfg(feature = "deno")]
pub mod deno;
#[cfg(feature = "javascript_compiler")]
pub mod javascript_compiler;
#[cfg(feature = "node")]
pub mod node;
#[cfg(feature = "v8")]
pub mod v8;

#[allow(unused_imports)]
#[cfg(feature = "bun")]
pub use bun::*;
#[allow(unused_imports)]
#[cfg(feature = "deno")]
pub use deno::*;
#[allow(unused_imports)]
#[cfg(feature = "javascript_compiler")]
pub use javascript_compiler::*;
#[allow(unused_imports)]
#[cfg(feature = "node")]
pub use node::*;
#[allow(unused_imports)]
#[cfg(feature = "v8")]
pub use v8::*;
