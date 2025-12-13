//! Procedural macros for defining linter rules.
//!
//! This crate provides macros for defining lint rules with auto-generated codes,
//! static metadata, and boilerplate implementations.

mod lint;

use proc_macro::TokenStream;

/// Define lint rules for a category.
///
/// # Example
///
/// ```ignore
/// define_lints! {
///     /// Correctness lints detect likely bugs.
///     category Correctness {
///         /// Detects Promises that are not awaited or handled.
///         #[id = "no-floating-promise"]
///         #[default_severity = Error]
///         #[docs = "https://destack.sh/lints/no-floating-promise"]
///         NoFloatingPromise {
///             node: GlobalNodeIdAny,
///             promise_type: GlobalTypeId,
///         } => "Promise must be awaited or handled",
///
///         /// Detects unused variables.
///         #[id = "no-unused-vars"]
///         #[default_severity = Warn]
///         #[fixable]
///         NoUnusedVars {
///             node: GlobalNodeIdAny,
///             name: StringId,
///         } => |self, program| {
///             format!("'{}' is declared but never used", program.strings.get(self.name))
///         },
///     }
/// }
/// ```
///
/// # Generated Code
///
/// For each category, this generates:
/// - Lint struct for each rule with metadata
/// - `LintDef` static metadata for all variants
/// - `ALL` - static array of all lint definitions
/// - `ALL_CODES` - static array of all code strings
/// - `is_valid_code()` - checks if a code string is valid
#[proc_macro]
pub fn define_lints(input: TokenStream) -> TokenStream {
    lint::define_lints_impl(input)
}
