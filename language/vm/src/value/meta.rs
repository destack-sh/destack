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

/// The shift used for the packed reference address space.
const REF_ADDRESS_SPACE_SHIFT: u8 = 7;

/// The packed bit mask for the reference address space.
const REF_ADDRESS_SPACE_MASK: u16 = 0x7 << REF_ADDRESS_SPACE_SHIFT;

/// Address space class for reference metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceAddressSpace {
    /// Worker-local heap.
    Local,
    /// World-shared heap.
    Shared,
    /// Stack allocation.
    Stack,
    /// Frame bytes.
    Frame,
    /// Static image memory.
    Static,
    /// Named backend space.
    Named,
}

impl ReferenceAddressSpace {
    /// Map a MIR address space into a VM reference address space.
    pub fn from_mir(address_space: mir::AddressSpace) -> Self {
        match address_space {
            mir::AddressSpace::Local => ReferenceAddressSpace::Local,
            mir::AddressSpace::Shared => ReferenceAddressSpace::Shared,
            mir::AddressSpace::Stack => ReferenceAddressSpace::Stack,
            mir::AddressSpace::Frame => ReferenceAddressSpace::Frame,
            mir::AddressSpace::Static => ReferenceAddressSpace::Static,
            mir::AddressSpace::Named(_) => ReferenceAddressSpace::Named,
        }
    }

    /// Decode a reference address space from packed bits.
    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0 => ReferenceAddressSpace::Local,
            1 => ReferenceAddressSpace::Stack,
            2 => ReferenceAddressSpace::Static,
            3 => ReferenceAddressSpace::Shared,
            4 => ReferenceAddressSpace::Frame,
            _ => ReferenceAddressSpace::Named,
        }
    }

    /// Encode a reference address space as packed bits.
    pub fn to_bits(self) -> u8 {
        match self {
            ReferenceAddressSpace::Local => 0,
            ReferenceAddressSpace::Stack => 1,
            ReferenceAddressSpace::Static => 2,
            ReferenceAddressSpace::Shared => 3,
            ReferenceAddressSpace::Frame => 4,
            ReferenceAddressSpace::Named => 5,
        }
    }

    /// Check whether this address space is supported by the VM.
    pub fn is_supported_by_vm(self) -> bool {
        matches!(
            self,
            ReferenceAddressSpace::Local
                | ReferenceAddressSpace::Shared
                | ReferenceAddressSpace::Stack
                | ReferenceAddressSpace::Frame
                | ReferenceAddressSpace::Static
        )
    }

    /// Return a human-readable label for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            ReferenceAddressSpace::Local => "local",
            ReferenceAddressSpace::Shared => "shared",
            ReferenceAddressSpace::Stack => "stack",
            ReferenceAddressSpace::Frame => "frame",
            ReferenceAddressSpace::Static => "static",
            ReferenceAddressSpace::Named => "named",
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
        address_space: mir::AddressSpace,
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
        let address_space_bits =
            u16::from(ReferenceAddressSpace::from_mir(address_space).to_bits());
        let nullability_bits = match nullability {
            mir::Nullability::None => 0,
            mir::Nullability::Null => 1,
            mir::Nullability::Undefined => 2,
            mir::Nullability::NullOrUndefined => 3,
        };

        let mut bits = kind_bits | (access_bits << REF_ACCESS_SHIFT);
        bits |= nullability_bits << REF_NULLABILITY_SHIFT;
        bits |= address_space_bits << REF_ADDRESS_SPACE_SHIFT;

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

    /// Get the reference address space.
    pub fn address_space(self) -> ReferenceAddressSpace {
        let bits = ((self.bits & REF_ADDRESS_SPACE_MASK) >> REF_ADDRESS_SPACE_SHIFT) as u8;
        ReferenceAddressSpace::from_bits(bits)
    }

    /// Return the raw metadata bits.
    pub fn bits(self) -> u16 {
        self.bits
    }
}
