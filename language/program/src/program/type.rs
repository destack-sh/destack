use destack_core::{
    EntryRange, EntryStore, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::LayoutId;

/// Durable runtime type id inside one program.
#[repr(C)]
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
    descriptors: SectionSlice<TypeDescriptor>,
    /// Flattened runtime supertype ids.
    supertypes: SectionSlice<TypeId>,
}

impl TypeTable {
    /// Pack one runtime type table.
    pub fn pack(sections: &mut SectionPacker, descriptors: Vec<TypeDescriptorBuilder>) -> Self {
        let mut entries = Vec::with_capacity(descriptors.len());
        let mut supertypes = EntryStore::new();

        // flatten variable descriptor payloads
        for descriptor in descriptors {
            let supertype_range = supertypes.append(descriptor.supertypes);

            entries.push(TypeDescriptor {
                layout: descriptor.layout,
                supertypes: supertype_range,
            });
        }

        let descriptors = sections.insert(entries);
        let supertypes = sections.insert(supertypes.into_entries());

        Self {
            descriptors,
            supertypes,
        }
    }

    /// Return one runtime type descriptor.
    pub fn descriptor<'a>(
        &self,
        sections: SectionImage<'a>,
        ty: TypeId,
    ) -> Option<&'a TypeDescriptor> {
        sections.entries(self.descriptors).get(ty.index())
    }

    /// Return the runtime layout id for one type.
    pub fn layout_id(&self, sections: SectionImage<'_>, ty: TypeId) -> Option<LayoutId> {
        Some(self.descriptor(sections, ty)?.layout)
    }

    /// Return whether one concrete type satisfies one expected runtime type.
    pub fn is_subtype(
        &self,
        sections: SectionImage<'_>,
        concrete: TypeId,
        expected: TypeId,
    ) -> Option<bool> {
        self.descriptor(sections, expected)?;

        if concrete == expected {
            return Some(true);
        }

        let descriptor = self.descriptor(sections, concrete)?;

        Some(self.supertypes(sections, descriptor).contains(&expected))
    }

    /// Return flattened runtime supertypes for one descriptor.
    pub fn supertypes<'a>(
        &self,
        sections: SectionImage<'a>,
        descriptor: &TypeDescriptor,
    ) -> &'a [TypeId] {
        descriptor
            .supertypes
            .slice(sections.entries(self.supertypes))
    }
}

/// Runtime type descriptor required by program code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeDescriptor {
    /// The resolved runtime storage layout.
    pub layout: LayoutId,
    /// Flattened runtime supertypes satisfied by this type.
    pub supertypes: EntryRange<TypeId>,
}

/// Build-time runtime type descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeDescriptorBuilder {
    /// The resolved runtime storage layout.
    pub layout: LayoutId,
    /// Flattened runtime supertypes satisfied by this type.
    pub supertypes: Vec<TypeId>,
}

// SAFETY: type ids and descriptors are fixed-width program entries.
unsafe impl SectionEntry for TypeId {}
unsafe impl SectionEntry for TypeDescriptor {}
