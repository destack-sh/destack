use std::error::Error;
use std::fmt::{Display, Formatter};

use tspp_dir as dir;
use tspp_source::ModuleId;

/// A failure while evaluating a pattern against checked DIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextError {
    /// Two module contexts have the same module id.
    DuplicateModule(ModuleId),
    /// A requested module is absent from the program.
    MissingModule(ModuleId),
    /// A checked type is absent from its owning module.
    MissingType(dir::GlobalTypeId),
    /// An applied declaration has no checked generic template.
    MissingGenericTemplate(dir::GlobalSymbolId),
    /// An application disagrees with its checked generic template arity.
    MismatchedGenericArguments(dir::GlobalSymbolId),
    /// Compiled predicate type syntax violates the supported algebra.
    InvalidPredicateType,
    /// A public dependency item has no exact resolved target.
    InvalidReference(dir::GlobalNodeIdAny),
    /// An exported name has no source declaration or dependency item.
    InvalidExport(ModuleId, dir::ExportKey),
    /// Checked DIR artifacts disagree about their module.
    MismatchedModule,
}

impl Display for ContextError {
    /// Format the violated context invariant.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateModule(module) => write!(formatter, "duplicate DIR module {module:?}"),
            Self::MissingModule(module) => write!(formatter, "missing DIR module {module:?}"),
            Self::MissingType(id) => write!(formatter, "missing checked DIR type {id:?}"),
            Self::MissingGenericTemplate(symbol) => {
                write!(formatter, "missing checked DIR generic template {symbol:?}")
            }
            Self::MismatchedGenericArguments(symbol) => {
                write!(
                    formatter,
                    "generic arguments disagree for DIR symbol {symbol:?}"
                )
            }
            Self::InvalidPredicateType => {
                formatter.write_str("compiled predicate contains an unsupported type")
            }
            Self::InvalidReference(node) => {
                write!(formatter, "invalid DIR reference at {node:?}")
            }
            Self::InvalidExport(module, key) => {
                write!(formatter, "invalid DIR export {key:?} in {module:?}")
            }
            Self::MismatchedModule => {
                formatter.write_str("checked DIR artifacts belong to different modules")
            }
        }
    }
}

impl Error for ContextError {}
