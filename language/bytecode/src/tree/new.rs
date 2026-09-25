use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One exact `new` operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct New {
    /// The allocated value form.
    pub kind: NewKind,
    /// The initial storage state.
    pub initialization: Initialization,
    /// Whether failure follows an explicit branch.
    pub is_fallible: bool,
}

impl New {
    /// The number of stable `new` operation encodings.
    pub(crate) const CODE_COUNT: u16 = 8;

    /// Encode this operation inside the `new` opcode range.
    pub(crate) const fn code(self) -> u16 {
        (self.kind as u16) | ((self.initialization as u16) << 1) | ((self.is_fallible as u16) << 2)
    }

    /// Decode one operation inside the `new` opcode range.
    pub(crate) const fn from_code(code: u16) -> Option<Self> {
        if code >= Self::CODE_COUNT {
            return None;
        }

        let kind = if code & 1 == 0 {
            NewKind::Value
        } else {
            NewKind::Slice
        };
        let initialization = if code & (1 << 1) == 0 {
            Initialization::Zeroed
        } else {
            Initialization::Uninit
        };
        let is_fallible = code & (1 << 2) != 0;

        Some(Self {
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
