/// The options for evaluating.
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// Default integer width.
    pub default_int_width: u16 = 32,
    /// Default float width.
    pub default_float_width: u16 = 32,
    /// Whether to implicitly type unannotated expressions as any.
    pub implicit_any_type: bool = false,
    /// Whether to resolve overloaded operators.
    pub overload_operators: bool = true,
    /// Whether to resolve overloaded functions.
    pub overload_functions: bool = true,
}
