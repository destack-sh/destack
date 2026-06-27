use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::LayoutId;

/// Durable runtime type id inside one program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for TypeId {
    /// Convert one raw program type id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<TypeId> for u32 {
    /// Convert one program type id into its raw value.
    fn from(id: TypeId) -> Self {
        id.0
    }
}

/// Runtime type table carried by one durable program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeTable {
    /// Dense runtime type descriptors keyed by program type id.
    descriptors: Vec<TypeDescriptor>,
}

impl TypeTable {
    /// Create one runtime type table.
    pub fn new(descriptors: Vec<TypeDescriptor>) -> Self {
        Self { descriptors }
    }

    /// Return one runtime type descriptor.
    pub fn descriptor(&self, ty: TypeId) -> Option<&TypeDescriptor> {
        self.descriptors.get(ty.index())
    }

    /// Return the transparent representation type.
    pub fn repr_type(&self, mut ty: TypeId) -> Option<TypeId> {
        loop {
            let descriptor = self.descriptor(ty)?;

            if descriptor.repr == ty {
                return Some(ty);
            }

            ty = descriptor.repr;
        }
    }

    /// Return the runtime layout id for one type.
    pub fn layout_id(&self, ty: TypeId) -> Option<LayoutId> {
        let ty = self.repr_type(ty)?;

        Some(self.descriptor(ty)?.layout)
    }

    /// Return whether one concrete type satisfies one expected runtime type.
    pub fn is_subtype(&self, concrete: TypeId, expected: TypeId) -> bool {
        if concrete == expected {
            return true;
        }

        self.descriptor(concrete)
            .map(|descriptor| descriptor.supertypes.contains(&expected))
            .unwrap_or(false)
    }
}

/// Runtime type descriptor required by executable code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeDescriptor {
    /// The transparent representation type.
    pub repr: TypeId,
    /// The runtime layout row.
    pub layout: LayoutId,
    /// Flattened runtime supertypes satisfied by this type.
    pub supertypes: Vec<TypeId>,
}
