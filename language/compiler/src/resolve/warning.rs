use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::define_warnings;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

define_warnings!(Resolve, {
    /// Unknown import.
    "WR001" = UnknownImport {
        node: GlobalNodeIdAny,
    } => "unknown import",

    /// Unused imports or unused re-exports.
    "WR002" = UnusedImport {
        node: GlobalNodeIdAny,
    } => "unused import",

    /// Import that resolves but is only used for side effects.
    "WR003" = SideEffectOnlyImport {
        node: GlobalNodeIdAny,
    } => "side effect only import",
});
