use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ReferenceKind, ReferenceType, Space, Storage, TypeId, ValueType};

/// One exact `new` operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct New {
    /// The destination memory space.
    pub space: Space,
    /// The allocated reference ownership.
    pub ownership: ReferenceKind,
    /// The allocated value form.
    pub kind: NewKind,
    /// The initial storage state.
    pub initialization: Initialization,
    /// Whether failure follows an explicit branch.
    pub is_fallible: bool,
}

impl New {
    /// The number of stable `new` operation encodings.
    pub(crate) const CODE_COUNT: u16 = 32;

    /// Select one exact allocation operation.
    pub const fn select(
        reference: ReferenceType,
        kind: NewKind,
        initialization: Initialization,
        is_fallible: bool,
    ) -> Option<Self> {
        let Some(space) = reference.storage().heap_space() else {
            return None;
        };
        let ownership = reference.kind();
        if !matches!(ownership, ReferenceKind::MANAGED | ReferenceKind::UNIQUE) {
            return None;
        }

        Some(Self {
            space,
            ownership,
            kind,
            initialization,
            is_fallible,
        })
    }

    /// Return the reference representation produced by this operation.
    pub const fn reference(self) -> ReferenceType {
        ReferenceType::new(self.ownership, Storage::heap(self.space))
    }

    /// Return the value type produced for one allocated element type.
    pub const fn result_type(self, ty: TypeId) -> ValueType {
        match (self.kind, self.initialization) {
            (NewKind::Value, Initialization::Zeroed) => {
                ValueType::reference(self.ownership, Storage::heap(self.space))
            }
            (NewKind::Value, Initialization::Uninit) => {
                ValueType::uninit_reference(self.ownership, Storage::heap(self.space))
            }
            (NewKind::Slice, Initialization::Zeroed) => {
                ValueType::slice(ty, self.ownership, Storage::heap(self.space))
            }
            (NewKind::Slice, Initialization::Uninit) => {
                ValueType::uninit_slice(ty, self.ownership, Storage::heap(self.space))
            }
        }
    }

    /// Encode this operation inside the `new` opcode range.
    pub(crate) const fn code(self) -> Option<u16> {
        let space = match self.space {
            Space::LOCAL => 0,
            Space::SHARED => 1,
            _ => return None,
        };
        let ownership = match self.ownership {
            ReferenceKind::MANAGED => 0,
            ReferenceKind::UNIQUE => 1,
            _ => return None,
        };
        let code = space
            | (ownership << 1)
            | ((self.kind as u16) << 2)
            | ((self.initialization as u16) << 3)
            | ((self.is_fallible as u16) << 4);

        Some(code)
    }

    /// Decode one operation inside the `new` opcode range.
    pub(crate) const fn from_code(code: u16) -> Option<Self> {
        if code >= Self::CODE_COUNT {
            return None;
        }

        let space = if code & 1 == 0 {
            Space::LOCAL
        } else {
            Space::SHARED
        };
        let ownership = if code & (1 << 1) == 0 {
            ReferenceKind::MANAGED
        } else {
            ReferenceKind::UNIQUE
        };
        let kind = if code & (1 << 2) == 0 {
            NewKind::Value
        } else {
            NewKind::Slice
        };
        let initialization = if code & (1 << 3) == 0 {
            Initialization::Zeroed
        } else {
            Initialization::Uninit
        };
        let is_fallible = code & (1 << 4) != 0;

        Some(Self {
            space,
            ownership,
            kind,
            initialization,
            is_fallible,
        })
    }
}

/// One `new` operation form.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum NewKind {
    /// One fixed-size value.
    Value = 0,
    /// One variable-length slice.
    Slice = 1,
}

/// One `new` storage initialization mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Initialization {
    /// Storage initialized with the zero bit pattern.
    Zeroed = 0,
    /// Storage whose bytes are not initialized yet.
    Uninit = 1,
}

impl Initialization {
    /// Parse one canonical initialization name.
    pub const fn from_name(name: &str) -> Option<Self> {
        match name.as_bytes() {
            b"zeroed" => Some(Self::Zeroed),
            b"uninit" => Some(Self::Uninit),
            _ => None,
        }
    }

    /// Return the canonical initialization name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Zeroed => "zeroed",
            Self::Uninit => "uninit",
        }
    }
}
