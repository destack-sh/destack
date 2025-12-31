mod lint;

use proc_macro::TokenStream;

/// Declare a single lint rule.
///
/// # Example
///
/// ```ignore
/// declare_lint! {
///     /// Disallow debugger statements in production code.
///     ///
///     /// Debugger statements should not be committed to production code
///     /// as they can cause the program to pause unexpectedly.
///     #[lint(
///         id = "no-debugger",
///         code = "LC002",
///         category = Correctness,
///         level = Dir,
///         fixable = true,
///     )]
///     pub NoDebugger,
///     "Disallow debugger statements"
/// }
/// ```
///
/// # Generated Code
///
/// This generates:
/// - `pub struct NoDebugger;`
/// - `pub static NO_DEBUGGER: &LintMeta = &LintMeta { ... };`
/// - `impl LintRule for NoDebugger { fn meta(&self) -> &'static LintMeta { NO_DEBUGGER } }`
#[proc_macro]
pub fn declare_lint(input: TokenStream) -> TokenStream {
    lint::declare_lint_impl(input)
}
