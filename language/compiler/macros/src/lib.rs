mod error;
mod pass;
mod warning;

use proc_macro::TokenStream;

/// Derive macro for defining error types for a compiler phase.
///
/// # Example
///
/// ```ignore
/// use destack_compiler_macros::DefineError;
///
/// #[derive(Debug, Clone, PartialEq, DefineError)]
/// #[phase(Resolve)]
/// pub enum ResolveError {
///     /// Wait for a build requirement.
///     #[error(code = "ER000", r#yield)]
///     Yield { requirement: ArtifactRequirementSet },
///
///     /// Use of undeclared symbol.
///     #[error(code = "ER004", message = "missing symbol {key}")]
///     UndeclaredSymbol {
///         node: GlobalNodeIdAny,
///         scope: GlobalScopeId,
///         key: StaticKey,
///     },
/// }
/// ```
///
/// # Format Strings
///
/// The `message` attribute supports format strings with automatic type conversion:
/// - `{field}` with `StringId` → `program.strings.get(*field)`
/// - `{field}` with `StaticKey` → `field.debug_string(&program.strings)`
/// - `{field}` with `ModuleId` → `program.modules.get(*field).uri`
/// - `{field}` with `GlobalNodeIdAny` → `field.local_id.ty.name()`
/// - Other types use their Display impl
///
/// # Generated Code
///
/// For each phase, this generates:
/// - `sub_code()` returns the numeric sub-code
/// - `code()` returns the full code string (e.g., "ER004")
/// - `anchor()` extracts the diagnostic anchor from fields
/// - `message()` formats the error message
/// - `ALL` static array of `DiagnosticDefinition` for all variants
/// - `ALL_CODES` static array of all code strings
/// - `is_valid_code()` checks if a code string is valid
/// - `Display`, `From<X> for TaskError`, `TryFrom` impls
#[proc_macro_derive(DefineError, attributes(phase, error))]
pub fn define_error(input: TokenStream) -> TokenStream {
    error::define_error_impl(input)
}

/// Derive macro for defining warning types for a compiler phase.
///
/// # Example
///
/// ```ignore
/// use destack_compiler_macros::DefineWarning;
///
/// #[derive(Debug, Clone, PartialEq, DefineWarning)]
/// #[phase(Resolve)]
/// pub enum ResolveWarning {
///     /// Unused import.
///     #[warning(code = "WR001", message = "unused import")]
///     UnusedImport { node: GlobalNodeIdAny },
/// }
/// ```
#[proc_macro_derive(DefineWarning, attributes(phase, warning, standalone))]
pub fn define_warning(input: TokenStream) -> TokenStream {
    warning::define_warning_impl(input)
}

/// Declare an optimization pass.
///
/// # Example
///
/// ```ignore
/// declare_pass! {
///     /// Fold constant expressions at compile time.
///     ///
///     /// Evaluates operations on constant values and replaces them with
///     /// the computed result.
///     #[pass(
///         id = "constant-fold",
///         requires = [],
///         invalidates = [],
///     )]
///     pub ConstantFold,
///     "Fold constant expressions"
/// }
/// ```
///
/// # Generated Code
///
/// This generates:
/// - `pub struct ConstantFold;`
/// - `pub static CONSTANT_FOLD: &PassMeta = &PassMeta { ... };`
/// - `impl ConstantFold { pub const fn meta() -> &'static PassMeta { CONSTANT_FOLD } }`
#[proc_macro]
pub fn declare_pass(input: TokenStream) -> TokenStream {
    pass::declare_pass_impl(input)
}
