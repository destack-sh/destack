use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{Signature, SourceReference};

/// One checked public declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationReference {
    /// The declared name when one exists.
    pub name: Option<String>,
    /// The declaration kind.
    pub kind: DeclarationKind,
    /// The canonical checked declaration signature.
    pub signature: Signature,
    /// The rendered authored documentation.
    pub documentation: Option<String>,
    /// The public members declared directly by this item.
    pub members: Vec<DeclarationReference>,
    /// The authored source location.
    pub source: SourceReference,
}

/// A stable public declaration category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "snake_case")]
pub enum DeclarationKind {
    /// Associated constant.
    AssociatedConst,
    /// Associated type.
    AssociatedType,
    /// Class declaration.
    Class,
    /// Immutable value declaration.
    Constant,
    /// Constructor declaration.
    Constructor,
    /// Enum declaration.
    Enum,
    /// Enum member.
    EnumMember,
    /// Extension declaration.
    Extension,
    /// Field declaration.
    Field,
    /// Function declaration.
    Function,
    /// Interface declaration.
    Interface,
    /// Method declaration.
    Method,
    /// Module declaration.
    Module,
    /// Namespace declaration.
    Namespace,
    /// Nominal type declaration.
    Newtype,
    /// Nominal interface declaration.
    NewtypeInterface,
    /// Property declaration.
    Property,
    /// Struct declaration.
    Struct,
    /// Type alias declaration.
    TypeAlias,
    /// Mutable value declaration.
    Variable,
}
