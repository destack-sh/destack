use destack_core::StringId;
use destack_mir as mir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One object-local type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Type {
    /// The module-local MIR type identity.
    pub id: mir::TypeId,
    /// The identity used to canonicalize this type across objects.
    pub key: mir::TypeKey,
    /// The complete type definition without its arena identity.
    pub definition: mir::Type,
    /// The persistent symbol when this type has nominal identity.
    pub symbol: Option<mir::Symbol>,
    /// The canonical display name when present.
    pub name: Option<StringId>,
    /// The nominal lineage when present.
    pub lineage: Option<mir::TypeLineage>,
}
