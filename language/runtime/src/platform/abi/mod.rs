mod array;
mod binding;
mod slice;
mod string;

pub use crate::diagnostic::RuntimeStatus;
pub use array::{PlatformArray, VmArray};
pub use binding::{BindingAbi, NativeAbi, VmAbi};
pub use slice::{PlatformSlice, VmSlice};
pub use string::{PlatformStringRef, PlatformStringSlice};

/// Native ABI string reference alias.
pub type NativeStringRef = PlatformStringRef;
/// Native ABI string slice alias.
pub type NativeStringSlice = PlatformStringSlice;
/// Native ABI slice alias.
pub type NativeSlice<T> = PlatformSlice<T>;
/// Native ABI array alias.
pub type NativeArray<T> = PlatformArray<T>;
