/// The options for evaluating.
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// Default integer width (if not specified).
    pub default_int_width: u16 = 32,
    /// Default float width (if not specified).
    pub default_float_width: u16 = 64,
}
