mod error;
mod warning;

use proc_macro::TokenStream;

/// Define error types for a compiler phase.
///
/// # Example
///
/// ```ignore
/// define_errors!(Resolve, {
///     /// Wait for task dependency.
///     #[error_yield]
///     "ER000" = Yield {
///         dependency: TaskDependency,
///     } => "pending dependency",
///
///     /// Use of undeclared symbol.
///     "ER004" = UndeclaredSymbol {
///         node: GlobalNodeIdAny,
///         scope: GlobalScopeId,
///         key: StaticKey,
///     } => {
///         let key = key.debug_string(&program.strings);
///         format!("missing symbol {key}")
///     },
/// });
/// ```
///
/// # Generated Code
///
/// For each phase, this generates:
/// - The error enum with all variants
/// - `sub_code()` returns the numeric sub-code
/// - `code()` returns the full code string (e.g., "ER004")
/// - `anchor()` extracts the diagnostic anchor from fields
/// - `message()` formats the error message
/// - `ALL` static array of `DiagnosticDef` for all variants
/// - `ALL_CODES` static array of all code strings
/// - `is_valid_code()` checks if a code string is valid
/// - `Display`, `From<X> for TaskError`, `TryFrom` impls
#[proc_macro]
pub fn define_errors(input: TokenStream) -> TokenStream {
    error::define_errors_impl(input)
}

/// Define warning types for a compiler phase.
///
/// # Example
///
/// ```ignore
/// define_warnings!(Analyze, {
///     /// Unreachable code.
///     "WA000" = UnreachableCode {
///         node: GlobalNodeIdAny,
///     } => "unreachable code",
/// });
/// ```
#[proc_macro]
pub fn define_warnings(input: TokenStream) -> TokenStream {
    warning::define_warnings_impl(input)
}
