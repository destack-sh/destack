use crate::diagnostic::RuntimeResult;
use crate::platform::{NativeAbiCodec, VmAbiCodec, VmCollectionElement, VmValueCodec};
use crate::runtime::BindingCallContext;
use destack_vm;
use serde::{Deserialize, Serialize};

use super::ResourceKind;
use super::kind::for_each_resource_handle_kind;

/// The identifier for one runtime-managed resource table entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(
    /// The inner identifier value.
    pub u64,
);

impl VmValueCodec for ResourceId {
    fn decode(value: destack_vm::Word) -> RuntimeResult<Self> {
        Ok(Self(<u64 as VmValueCodec>::decode(value)?))
    }

    fn encode(self) -> destack_vm::Word {
        <u64 as VmValueCodec>::encode(self.0)
    }
}

impl NativeAbiCodec for ResourceId {
    type Value = Self;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(self)
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        value
    }
}

impl VmAbiCodec for ResourceId {
    type Value = Self;

    fn into_value(self, _context: &destack_vm::BindingRead<'_, '_>) -> RuntimeResult<Self::Value> {
        Ok(self)
    }

    fn from_value(
        _context: &mut destack_vm::BindingWrite<'_, '_>,
        value: Self::Value,
    ) -> RuntimeResult<Self> {
        Ok(value)
    }
}

impl VmCollectionElement for ResourceId {}

/// The ownership mode for one transferred resource.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceOwnership {
    /// The sender retains ownership.
    Borrowed = 1,
    /// The receiver takes ownership.
    Owned = 2,
}

/// VM transport alias for resource kind labels.
pub type ResourceKindVm = destack_vm::StringHandle;

/// Typed resource-handle contract bound to one canonical resource kind.
pub trait ResourceHandle: Copy {
    /// Canonical resource kind for this handle type.
    const KIND: ResourceKind;

    /// Return the raw resource identifier.
    fn resource_id(self) -> ResourceId;

    /// Build one typed handle from a raw resource identifier.
    fn from_resource_id(resource_id: ResourceId) -> Self;
}

macro_rules! define_resource_handle_types {
    ($(($handle:ident, $kind:ident, $kind_id:literal, $label:literal, $doc:literal),)+) => {
        $(
            #[doc = $doc]
            #[repr(transparent)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
            pub struct $handle(
                /// The inner resource identifier.
                pub ResourceId,
            );

            impl ResourceHandle for $handle {
                const KIND: ResourceKind = ResourceKind::$kind;

                fn resource_id(self) -> ResourceId {
                    self.0
                }

                fn from_resource_id(resource_id: ResourceId) -> Self {
                    Self(resource_id)
                }
            }

            impl From<ResourceId> for $handle {
                fn from(resource_id: ResourceId) -> Self {
                    Self::from_resource_id(resource_id)
                }
            }

            impl From<$handle> for ResourceId {
                fn from(handle: $handle) -> Self {
                    handle.resource_id()
                }
            }

            impl VmValueCodec for $handle {
                fn decode(value: destack_vm::Word) -> RuntimeResult<Self> {
                    Ok(Self(<ResourceId as VmValueCodec>::decode(value)?))
                }

                fn encode(self) -> destack_vm::Word {
                    <ResourceId as VmValueCodec>::encode(self.0)
                }
            }

            impl NativeAbiCodec for $handle {
                type Value = Self;

                unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
                    Ok(self)
                }

                fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
                    value
                }
            }

            impl VmAbiCodec for $handle {
                type Value = Self;

                fn into_value(
                    self,
                    _context: &destack_vm::BindingRead<'_, '_>,
                ) -> RuntimeResult<Self::Value> {
                    Ok(self)
                }

                fn from_value(
                    _context: &mut destack_vm::BindingWrite<'_, '_>,
                    value: Self::Value,
                ) -> RuntimeResult<Self> {
                    Ok(value)
                }
            }

            impl VmCollectionElement for $handle {}
        )+
    };
}

for_each_resource_handle_kind!(define_resource_handle_types);
