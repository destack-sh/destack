use destack_mir as mir;
use serde::{Deserialize, Serialize};

/// The packed bit mask for the reference kind.
const REF_KIND_MASK: u16 = 0x7;

/// The shift used for packed reference access.
const REF_ACCESS_SHIFT: u8 = 3;

/// The packed bit mask for reference access.
const REF_ACCESS_MASK: u16 = 0x3 << REF_ACCESS_SHIFT;

/// The shift used for packed reference nullability.
const REF_NULLABILITY_SHIFT: u8 = 5;

/// The packed bit mask for reference nullability.
const REF_NULLABILITY_MASK: u16 = 0x3 << REF_NULLABILITY_SHIFT;

/// The shift used for the packed reference space.
const REF_SPACE_SHIFT: u8 = 7;

/// The packed bit mask for the reference space.
const REF_SPACE_MASK: u16 = 0x7 << REF_SPACE_SHIFT;

/// Space class for reference metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceSpace {
    /// Worker-local heap.
    Local,
    /// Runtime-shared heap.
    Shared,
    /// Frame bytes.
    Frame,
    /// Static image memory.
    Static,
}

impl ReferenceSpace {
    /// Map a MIR space into a VM reference space.
    pub fn from_mir(space: mir::Space) -> Self {
        match space {
            mir::Space::Local => ReferenceSpace::Local,
            mir::Space::Shared => ReferenceSpace::Shared,
            mir::Space::Frame => ReferenceSpace::Frame,
            mir::Space::Static => ReferenceSpace::Static,
        }
    }

    /// Decode a reference space from packed bits.
    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0 => ReferenceSpace::Local,
            1 => ReferenceSpace::Frame,
            2 => ReferenceSpace::Static,
            3 => ReferenceSpace::Shared,
            _ => panic!("invalid reference space bits {bits}"),
        }
    }

    /// Encode a reference space as packed bits.
    pub fn to_bits(self) -> u8 {
        match self {
            ReferenceSpace::Local => 0,
            ReferenceSpace::Frame => 1,
            ReferenceSpace::Static => 2,
            ReferenceSpace::Shared => 3,
        }
    }

    /// Return a human-readable label for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            ReferenceSpace::Local => "local",
            ReferenceSpace::Shared => "shared",
            ReferenceSpace::Frame => "frame",
            ReferenceSpace::Static => "static",
        }
    }
}

/// Metadata for reference values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceMeta {
    bits: u16,
}

impl ReferenceMeta {
    /// Empty reference metadata.
    pub const NONE: Self = Self { bits: 0 };

    /// Create reference metadata from raw bits.
    pub fn from_bits(bits: u16) -> Self {
        Self { bits }
    }

    /// Create reference metadata.
    pub fn new(
        kind: mir::ReferenceKind,
        space: mir::Space,
        access: mir::Access,
        nullability: mir::Nullability,
    ) -> Self {
        let kind_bits = match kind {
            mir::ReferenceKind::Managed => 1,
            mir::ReferenceKind::Unique => 2,
            mir::ReferenceKind::Borrowed => 3,
            mir::ReferenceKind::Raw => 4,
        };
        let access_bits = match access {
            mir::Access::Readonly => 0,
            mir::Access::Mutable => 1,
            mir::Access::Exclusive => 2,
        };
        let space_bits = u16::from(ReferenceSpace::from_mir(space).to_bits());
        let nullability_bits = match nullability {
            mir::Nullability::None => 0,
            mir::Nullability::Null => 1,
            mir::Nullability::Undefined => 2,
            mir::Nullability::NullOrUndefined => 3,
        };

        let mut bits = kind_bits | (access_bits << REF_ACCESS_SHIFT);
        bits |= nullability_bits << REF_NULLABILITY_SHIFT;
        bits |= space_bits << REF_SPACE_SHIFT;

        Self { bits }
    }

    /// Get the reference kind when available.
    pub fn kind(self) -> Option<mir::ReferenceKind> {
        match self.bits & REF_KIND_MASK {
            0 => None,
            1 => Some(mir::ReferenceKind::Managed),
            2 => Some(mir::ReferenceKind::Unique),
            3 => Some(mir::ReferenceKind::Borrowed),
            4 => Some(mir::ReferenceKind::Raw),
            _ => None,
        }
    }

    /// Get the reference access when available.
    pub fn access(self) -> Option<mir::Access> {
        self.kind()?;

        match (self.bits & REF_ACCESS_MASK) >> REF_ACCESS_SHIFT {
            0 => Some(mir::Access::Readonly),
            1 => Some(mir::Access::Mutable),
            2 => Some(mir::Access::Exclusive),
            _ => None,
        }
    }

    /// Get the reference nullability.
    pub fn nullability(self) -> mir::Nullability {
        match (self.bits & REF_NULLABILITY_MASK) >> REF_NULLABILITY_SHIFT {
            1 => mir::Nullability::Null,
            2 => mir::Nullability::Undefined,
            3 => mir::Nullability::NullOrUndefined,
            _ => mir::Nullability::None,
        }
    }

    /// Get the reference space.
    pub fn space(self) -> ReferenceSpace {
        let bits = ((self.bits & REF_SPACE_MASK) >> REF_SPACE_SHIFT) as u8;
        ReferenceSpace::from_bits(bits)
    }

    /// Return the raw metadata bits.
    pub fn bits(self) -> u16 {
        self.bits
    }
}
