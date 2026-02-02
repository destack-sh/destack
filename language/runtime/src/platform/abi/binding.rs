use destack_vm as vm;

use crate::platform::abi::{
    PlatformArray, PlatformSlice, PlatformStringRef, PlatformStringSlice, VmArray, VmSlice,
};

/// ABI configuration for platform bindings.
pub trait BindingAbi {
    /// String representation for the ABI.
    type String;
    /// String slice representation for the ABI.
    type StringSlice;
    /// Slice representation for the ABI.
    type Slice<T>;
    /// Array representation for the ABI.
    type Array<T>;
}

/// Native ABI configuration for platform bindings.
#[derive(Debug, Clone, Copy)]
pub struct NativeAbi;

impl BindingAbi for NativeAbi {
    type String = PlatformStringRef;
    type StringSlice = PlatformStringSlice;
    type Slice<T> = PlatformSlice<T>;
    type Array<T> = PlatformArray<T>;
}

/// VM ABI configuration for platform bindings.
#[derive(Debug, Clone, Copy)]
pub struct VmAbi;

impl BindingAbi for VmAbi {
    type String = vm::StringHandle;
    type StringSlice = VmSlice<vm::StringHandle>;
    type Slice<T> = VmSlice<T>;
    type Array<T> = VmArray<T>;
}
