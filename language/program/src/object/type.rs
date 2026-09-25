use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_mir as mir;
use tspp_serde::Reflect;

/// One object-local type declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Type {
    /// The module-local MIR type identity.
    pub id: mir::TypeId,
    /// The stable canonical fingerprint used to canonicalize this type across objects.
    pub fingerprint: mir::TypeFingerprint,
    /// The complete type definition without its arena identity.
    pub definition: mir::Type,
    /// The persistent symbol when this type has nominal identity.
    pub symbol: Option<mir::Symbol>,
    /// The source-facing declaration name when present.
    pub name: Option<StringId>,
    /// The direct nominal heritage when present.
    pub heritage: Option<mir::TypeHeritage>,
    /// The language item key this declaration binds, mirroring the source decorator.
    pub language_item: Option<StringId>,
}
