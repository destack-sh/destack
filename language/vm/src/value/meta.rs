use destack_mir as mir;
use serde::{Deserialize, Serialize};

/// Address space class for reference metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceAddressSpace {
    /// Local runtime storage.
    Local,
    /// Shared runtime storage.
    Shared,
    /// Stack or function-local memory.
    Stack,
    /// Frame-slot storage inside one activation.
    Frame,
    /// Global or module-static memory.
    Global,
    /// Named backend-specific storage space.
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
            mir::AddressSpace::Global => ReferenceAddressSpace::Global,
            mir::AddressSpace::Named(_) => ReferenceAddressSpace::Named,
        }
    }

    /// Decode a reference address space from packed bits.
    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0 => ReferenceAddressSpace::Local,
            1 => ReferenceAddressSpace::Stack,
            2 => ReferenceAddressSpace::Global,
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
            ReferenceAddressSpace::Global => 2,
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
                | ReferenceAddressSpace::Global
        )
    }

    /// Return a human-readable label for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            ReferenceAddressSpace::Local => "local",
            ReferenceAddressSpace::Shared => "shared",
            ReferenceAddressSpace::Stack => "stack",
            ReferenceAddressSpace::Frame => "frame",
            ReferenceAddressSpace::Global => "global",
            ReferenceAddressSpace::Named => "named",
        }
    }
}

/// Metadata for reference values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceMeta {
    bits: u8,
}

/// The packed bit mask for the reference kind.
const REF_KIND_MASK: u8 = 0x7;

/// The packed bit that marks mutable references.
const REF_MUTABLE_BIT: u8 = 1 << 3;

/// The packed bit that marks nullable references.
const REF_NULLABLE_BIT: u8 = 1 << 4;

/// The shift used for the packed reference address space.
const REF_ADDRESS_SPACE_SHIFT: u8 = 5;

/// The packed bit mask for the reference address space.
const REF_ADDRESS_SPACE_MASK: u8 = 0x7 << REF_ADDRESS_SPACE_SHIFT;

impl ReferenceMeta {
    /// Empty reference metadata.
    pub const NONE: Self = Self { bits: 0 };

    /// Create reference metadata from raw bits.
    pub(crate) const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    /// Create reference metadata.
    pub fn new(
        kind: mir::ReferenceKind,
        address_space: mir::AddressSpace,
        mutability: mir::Mutability,
        is_nullable: bool,
    ) -> Self {
        let kind_bits = match kind {
            mir::ReferenceKind::Managed => 1,
            mir::ReferenceKind::Owned => 2,
            mir::ReferenceKind::Borrowed => 3,
            mir::ReferenceKind::Raw => 4,
        };
        let address_space_bits = ReferenceAddressSpace::from_mir(address_space).to_bits();

        let mut bits = kind_bits | (address_space_bits << REF_ADDRESS_SPACE_SHIFT);
        if matches!(mutability, mir::Mutability::Mutable) {
            bits |= REF_MUTABLE_BIT;
        }
        if is_nullable {
            bits |= REF_NULLABLE_BIT;
        }

        Self { bits }
    }

    /// Get the reference kind when available.
    pub fn kind(self) -> Option<mir::ReferenceKind> {
        match self.bits & REF_KIND_MASK {
            0 => None,
            1 => Some(mir::ReferenceKind::Managed),
            2 => Some(mir::ReferenceKind::Owned),
            3 => Some(mir::ReferenceKind::Borrowed),
            4 => Some(mir::ReferenceKind::Raw),
            _ => None,
        }
    }

    /// Get the reference mutability when available.
    pub fn mutability(self) -> Option<mir::Mutability> {
        self.kind()?;

        if self.bits & REF_MUTABLE_BIT != 0 {
            Some(mir::Mutability::Mutable)
        } else {
            Some(mir::Mutability::Immutable)
        }
    }

    /// Check whether this reference is nullable.
    pub fn is_nullable(self) -> bool {
        self.bits & REF_NULLABLE_BIT != 0
    }

    /// Get the reference address space.
    pub fn address_space(self) -> ReferenceAddressSpace {
        let bits = (self.bits & REF_ADDRESS_SPACE_MASK) >> REF_ADDRESS_SPACE_SHIFT;
        ReferenceAddressSpace::from_bits(bits)
    }

    /// Return the raw metadata bits.
    pub fn bits(self) -> u8 {
        self.bits
    }
}
