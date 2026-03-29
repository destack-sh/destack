use destack_vm as vm;

use super::{NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, VmArray, VmSlice};

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

/// Native ABI configuration for platform bindings.
impl BindingAbi for NativeAbi {
    /// String representation for native ABI.
    type String = NativeStringRef;
    /// String slice representation for native ABI.
    type StringSlice = NativeStringSlice;
    /// Slice representation for native ABI.
    type Slice<T> = NativeSlice<T>;
    /// Array representation for native ABI.
    type Array<T> = NativeArray<T>;
}

/// VM ABI configuration for platform bindings.
#[derive(Debug, Clone, Copy)]
pub struct VmAbi;

/// VM ABI configuration for platform bindings.
impl BindingAbi for VmAbi {
    /// String representation for VM ABI.
    type String = vm::StringHandle;
    /// String slice representation for VM ABI.
    type StringSlice = VmSlice<vm::StringHandle>;
    /// Slice representation for VM ABI.
    type Slice<T> = VmSlice<T>;
    /// Array representation for VM ABI.
    type Array<T> = VmArray<T>;
}
