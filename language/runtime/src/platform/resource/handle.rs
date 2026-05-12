use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{
    NativeAbiCodec, PlatformError, VmAbiCodec, VmCollectionElement, VmValueCodec,
};
use crate::runtime::{BindingCallContext, WorkerId};
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::ResourceKind;
use super::kind::for_each_resource_handle_kind;

/// The global identifier for one runtime-managed resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResourceId {
    /// Worker that owns this resource.
    pub worker_id: WorkerId,
    /// Worker-local resource identifier.
    pub local_id: u64,
}

impl ResourceId {
    /// Build a resource identifier from its owning worker and local sequence.
    pub const fn new(worker_id: WorkerId, local_id: u64) -> Self {
        Self {
            worker_id,
            local_id,
        }
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

impl VmValueCodec for ResourceId {
    fn decode(_value: vm::Word) -> RuntimeResult<Self> {
        Err(RuntimeError::from(PlatformError::invalid_argument_type(
            "resource",
            "binding-scoped resource id",
        ))
        .boxed())
    }

    fn encode(self) -> vm::Word {
        vm::Word::uint(self.local_id, 64)
    }
}

impl VmCollectionElement for ResourceId {}

impl VmAbiCodec for ResourceId {
    type Value = Self;

    fn into_value(self, _context: &vm::BindingRead<'_, '_>) -> RuntimeResult<Self::Value> {
        Ok(self)
    }

    fn from_value(
        _context: &mut vm::BindingWrite<'_, '_>,
        value: Self::Value,
    ) -> RuntimeResult<Self> {
        Ok(value)
    }
}

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
                    _context: &vm::BindingRead<'_, '_>,
                ) -> RuntimeResult<Self::Value> {
                    Ok(self)
                }

                fn from_value(
                    _context: &mut vm::BindingWrite<'_, '_>,
                    value: Self::Value,
                ) -> RuntimeResult<Self> {
                    Ok(value)
                }
            }

            impl VmValueCodec for $handle {
                fn decode(value: vm::Word) -> RuntimeResult<Self> {
                    Ok(Self(ResourceId::decode(value)?))
                }

                fn encode(self) -> vm::Word {
                    self.0.encode()
                }
            }

            impl VmCollectionElement for $handle {}
        )+
    };
}

for_each_resource_handle_kind!(define_resource_handle_types);
