use crate::diagnostic::RuntimeResult;
use crate::platform::core::{NativeBindingCodec, VmDecodeCodec, VmEncodeCodec};
use crate::platform::{NativeAbiCodec, VmAbiCodec};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// One VM runtime decode adapter.
pub(crate) struct VmRuntimeDecode<'a, 'context> {
    /// The active VM binding call.
    context: &'a mut vm::BindingContext<'context>,
}

impl<'a, 'context> VmRuntimeDecode<'a, 'context> {
    /// Create one VM runtime decode adapter.
    pub(crate) fn new(context: &'a mut vm::BindingContext<'context>) -> Self {
        Self { context }
    }

    /// Decode one VM binding value into one materialized Rust value.
    pub(crate) fn decode<T: VmAbiCodec>(&mut self, value: T) -> RuntimeResult<T::Value> {
        let mut codec = VmDecodeCodec::new(self.context);

        codec.decode(value)
    }

    /// Decode one optional VM binding value into one materialized Rust value.
    pub(crate) fn decode_optional<T: VmAbiCodec>(
        &mut self,
        value: Option<T>,
    ) -> RuntimeResult<Option<T::Value>> {
        value.map(|value| self.decode(value)).transpose()
    }
}

/// One VM runtime binding adapter.
pub(crate) struct VmRuntimeBinding<'a, 'context> {
    /// The active VM binding call.
    context: &'a mut vm::BindingContext<'context>,
}

impl<'a, 'context> VmRuntimeBinding<'a, 'context> {
    /// Create one VM runtime binding adapter.
    pub(crate) fn new(context: &'a mut vm::BindingContext<'context>) -> Self {
        Self { context }
    }

    /// Encode one materialized Rust value into one VM binding value.
    pub(crate) fn encode<T: VmAbiCodec>(&mut self, value: T::Value) -> RuntimeResult<T> {
        let mut codec = VmEncodeCodec::new(self.context);

        codec.encode(value)
    }
}

/// One native runtime binding adapter.
pub(crate) struct NativeRuntimeBinding<'a> {
    /// The active native binding call.
    binding: &'a BindingCallContext,
}

impl<'a> NativeRuntimeBinding<'a> {
    /// Create one native runtime binding adapter.
    pub(crate) fn new(binding: &'a BindingCallContext) -> Self {
        Self { binding }
    }

    /// Decode one native binding value into one materialized Rust value.
    pub(crate) unsafe fn decode<T: NativeAbiCodec>(&self, value: T) -> RuntimeResult<T::Value> {
        let codec = NativeBindingCodec::new(self.binding);

        unsafe { codec.decode(value) }
    }

    /// Encode one materialized Rust value into one native binding value.
    pub(crate) fn encode<T: NativeAbiCodec>(&self, value: T::Value) -> T {
        let codec = NativeBindingCodec::new(self.binding);

        codec.encode(value)
    }

    /// Decode one optional native binding value into one materialized Rust value.
    pub(crate) unsafe fn decode_optional<T: NativeAbiCodec>(
        &self,
        value: Option<T>,
    ) -> RuntimeResult<Option<T::Value>> {
        value.map(|value| unsafe { self.decode(value) }).transpose()
    }
}
