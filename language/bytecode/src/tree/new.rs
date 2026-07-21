use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{ReferenceKind, ReferenceType, Space, TypeId, ValueType};

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
    /// Return the reference representation produced by this operation.
    pub const fn reference(self) -> ReferenceType {
        ReferenceType::new(self.ownership, self.space)
    }

    /// Return the value type produced for one allocated element type.
    pub const fn result_type(self, ty: TypeId) -> ValueType {
        match (self.kind, self.initialization) {
            (NewKind::Value, Initialization::Zeroed) => {
                ValueType::reference(self.ownership, self.space)
            }
            (NewKind::Value, Initialization::Uninit) => {
                ValueType::uninit_reference(self.ownership, self.space)
            }
            (NewKind::Slice, Initialization::Zeroed) => {
                ValueType::slice(ty, self.ownership, self.space)
            }
            (NewKind::Slice, Initialization::Uninit) => {
                ValueType::uninit_slice(ty, self.ownership, self.space)
            }
        }
    }

    /// Parse one canonical `new` operation name.
    pub fn from_name(name: &str) -> Option<Self> {
        let mut components = name.split('.');
        if components.next() != Some("new") {
            return None;
        }

        // parse the required operation qualifiers
        let space = components.next().and_then(Space::from_name)?;
        let ownership = components.next().and_then(ReferenceKind::from_name)?;
        let component = components.next()?;
        let (kind, initialization) = if component == "slice" {
            let initialization = components.next().and_then(Initialization::from_name)?;

            (NewKind::Slice, initialization)
        } else {
            let initialization = Initialization::from_name(component)?;

            (NewKind::Value, initialization)
        };

        // accept one optional fallibility suffix and no trailing components
        let is_fallible = match components.next() {
            Some("try") => true,
            None => false,
            Some(_) => return None,
        };
        if components.next().is_some() {
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
