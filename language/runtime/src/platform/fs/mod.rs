#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;
pub(crate) mod core;
pub mod native;
mod os;
#[cfg(test)]
mod tests;
pub mod vm;

use crate::platform::abi::{NativeAbi, VmAbi};

pub use crate::platform::resource::{DirectoryHandle, FileHandle};
pub use abi_generated::*;
pub use bindings_generated::*;

/// ABI alias for UTF-16 paths represented with the same byte layout.
///
/// This is a semantic alias only: UTF-16 and byte paths share the same ABI
/// container and are differentiated by `PathEncoding`.
pub use abi_generated::PathBytesAbi as PathUtf16Abi;

/// Native alias for UTF-16 paths represented as path bytes.
pub type PathUtf16 = PathUtf16Abi<NativeAbi>;

/// VM alias for UTF-16 paths represented as path bytes.
pub type PathUtf16Vm = PathUtf16Abi<VmAbi>;
