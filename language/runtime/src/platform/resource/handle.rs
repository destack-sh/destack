use crate::diagnostic::RuntimeResult;
use crate::platform::VmValueCodec;
use destack_vm as vm;
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
    fn decode(value: vm::Value) -> RuntimeResult<Self> {
        Ok(Self(<u64 as VmValueCodec>::decode(value)?))
    }

    fn encode(self) -> vm::Value {
        <u64 as VmValueCodec>::encode(self.0)
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
pub type ResourceKindVm = vm::StringHandle;

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
                fn decode(value: vm::Value) -> RuntimeResult<Self> {
                    Ok(Self(<ResourceId as VmValueCodec>::decode(value)?))
                }

                fn encode(self) -> vm::Value {
                    <ResourceId as VmValueCodec>::encode(self.0)
                }
            }
        )+
    };
}

for_each_resource_handle_kind!(define_resource_handle_types);
