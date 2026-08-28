use destack_core::StringId;
use destack_serde::Reflect;
use destack_source::ProvenanceId;
use serde::{Deserialize, Serialize};

use crate::{Expression, Identifier, IdentifierName, LocalNodeId, StringLiteral};

/// One ECMAScript property name.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum PropertyName {
    /// One fixed identifier name.
    Identifier(IdentifierName),
    /// One string literal name.
    String(StringLiteral),
    /// One computed property name.
    Computed(LocalNodeId<Expression>),
}

/// One ECMAScript class element name.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ClassElementName {
    /// One public property name.
    Public(PropertyName),
    /// One private identifier.
    Private(Identifier),
}

/// One ECMAScript module export name.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ModuleExportName {
    /// One identifier name.
    Identifier(IdentifierName),
    /// One string literal name.
    String(StringLiteral),
}

impl ModuleExportName {
    /// Return the ECMAScript string value of this name.
    pub const fn value(self) -> StringId {
        match self {
            Self::Identifier(name) => name.text,
            Self::String(name) => name.value,
        }
    }

    /// Return the provenance of this name.
    pub const fn provenance(self) -> ProvenanceId {
        match self {
            Self::Identifier(name) => name.provenance,
            Self::String(name) => name.provenance,
        }
    }
}
