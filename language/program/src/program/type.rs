use serde::{Deserialize, Serialize};
use tspp_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
};
use tspp_heap::DropId;
use tspp_serde::Reflect;

use super::LayoutId;

/// Durable 32-bit runtime type id inside one program.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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

/// Stable structural type identity across Program versions.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct TypeFingerprint(u128);

impl TypeFingerprint {
    /// Restore one type fingerprint from its persistent bits.
    pub const fn from_raw(raw: u128) -> Self {
        Self(raw)
    }

    /// Return the persistent fingerprint bits.
    pub const fn raw(self) -> u128 {
        self.0
    }
}

/// Runtime type table carried by one durable program.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct TypeTable {
    /// Stable fingerprints keyed by program type id.
    fingerprints: SectionSlice<TypeFingerprint>,
    /// Dense runtime type descriptors keyed by program type id.
    descriptors: SectionSlice<TypeDescriptor>,
    /// Flattened runtime supertype ids.
    supertypes: SectionSlice<TypeId>,
}

impl TypeTable {
    /// Pack one runtime type table.
    pub(crate) fn pack(builder: TypeTableBuilder, sections: &mut SectionBuilder) -> Self {
        let mut entries = Vec::with_capacity(builder.descriptors.len());
        let mut supertypes = EntryStore::new();

        // flatten variable descriptor payloads
        for descriptor in builder.descriptors {
            let supertype_range = supertypes.append(descriptor.supertypes);

            entries.push(TypeDescriptor {
                layout: descriptor.layout,
                supertypes: supertype_range,
                drop: descriptor.drop.into(),
            });
        }

        let descriptors = sections.insert(entries);
        let supertypes = sections.insert(supertypes.into_entries());

        Self {
            fingerprints: sections.insert(builder.fingerprints),
            descriptors,
            supertypes,
        }
    }

    /// Return one stable type fingerprint.
    pub fn fingerprint(&self, sections: SectionImage<'_>, ty: TypeId) -> Option<TypeFingerprint> {
        sections.entries(self.fingerprints).get(ty.index()).copied()
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
    ) -> Result<bool, TypeId> {
        self.descriptor(sections, expected).ok_or(expected)?;

        if concrete == expected {
            return Ok(true);
        }

        let descriptor = self.descriptor(sections, concrete).ok_or(concrete)?;

        Ok(self.supertypes(sections, descriptor).contains(&expected))
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

    /// Return whether every descriptor range fits the supertype column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let supertypes = sections.entries(self.supertypes).len();

        sections
            .entries(self.descriptors)
            .iter()
            .all(|descriptor| descriptor.supertypes.fits(supertypes))
    }
}

/// Mutable runtime type table before section packing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypeTableBuilder {
    /// Stable fingerprints in dense type id order.
    fingerprints: Vec<TypeFingerprint>,
    /// Runtime descriptors in dense type id order.
    descriptors: Vec<TypeDescriptorBuilder>,
}

impl TypeTableBuilder {
    /// Create one empty runtime type table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set runtime types in dense type id order.
    pub fn types(
        mut self,
        types: impl IntoIterator<Item = (TypeFingerprint, TypeDescriptorBuilder)>,
    ) -> Self {
        (self.fingerprints, self.descriptors) = types.into_iter().unzip();

        self
    }
}

/// Runtime type descriptor required by program code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct TypeDescriptor {
    /// The resolved runtime storage layout.
    pub layout: LayoutId,
    /// Flattened runtime supertypes satisfied by this type.
    pub supertypes: EntryRange<TypeId>,
    /// Destructor when this type requires cleanup.
    pub drop: Optional<DropId>,
}

impl TypeDescriptor {
    /// Return the complete drop identity when this type requires cleanup.
    pub const fn drop_id(self) -> Option<DropId> {
        self.drop.get()
    }
}

/// Build-time runtime type descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDescriptorBuilder {
    /// The resolved runtime storage layout.
    layout: LayoutId,
    /// Flattened runtime supertypes satisfied by this type.
    supertypes: Vec<TypeId>,
    /// Complete drop identity when this type requires cleanup.
    drop: Option<DropId>,
}

impl TypeDescriptorBuilder {
    /// Create one runtime type descriptor builder.
    pub fn new(layout: LayoutId) -> Self {
        Self {
            layout,
            supertypes: Vec::new(),
            drop: None,
        }
    }

    /// Set runtime supertypes satisfied by this type.
    pub fn supertypes(mut self, supertypes: impl IntoIterator<Item = TypeId>) -> Self {
        self.supertypes = supertypes.into_iter().collect();

        self
    }

    /// Set the destructor required by this type.
    pub fn drop(mut self, drop: DropId) -> Self {
        self.drop = Some(drop);

        self
    }
}
