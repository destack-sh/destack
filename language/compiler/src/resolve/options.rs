/// The options for evaluating.
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// Default integer width.
    pub default_int_width: u16 = 32,
    /// Default float width.
    pub default_float_width: u16 = 32,
    /// Whether to implicitly type unannotated expressions as any.
    pub implicit_any_type: bool = true,
    /// Whether to resolve overimported operators.
    pub overimport_operators: bool = true,
    /// Whether to resolve overimported functions.
    pub overimport_functions: bool = true,
}
