mod artifact;
mod blob;
mod constants;
mod content;
mod error;
mod layout;
mod memory;
mod null;
mod record;
mod segment;
mod string;

#[cfg(all(not(target_os = "wasi"), not(target_arch = "wasm32")))]
mod disk;
#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
mod unsupported;
#[cfg(target_os = "wasi")]
mod wasi;

pub use artifact::*;
pub use blob::*;
pub use constants::*;
pub use content::*;
pub use error::*;
pub use layout::*;
pub use memory::*;
pub use null::*;
pub use record::*;
pub use segment::*;

#[cfg(all(not(target_os = "wasi"), not(target_arch = "wasm32")))]
pub use disk::*;
#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
pub use unsupported::*;
#[cfg(target_os = "wasi")]
pub use wasi::*;
