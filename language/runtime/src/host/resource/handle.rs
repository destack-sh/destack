use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::worker::WorkerId;

use super::ResourceKind;
use super::kind::for_each_resource_handle_kind;

/// The global identifier for one runtime-managed resource.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
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

        )+
    };
}

for_each_resource_handle_kind!(define_resource_handle_types);
