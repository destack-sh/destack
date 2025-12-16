/// The options for type/symbol resolution.
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Default integer width (if not specified).
    pub default_int_width: u16,
    /// Default float width (if not specified).
    pub default_float_width: u16,
    /// Whether to make prelude items (Add, Type, deprecated, etc.) available.
    /// When true, prelude items resolve without explicit imports.
    pub inject_prelude: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            default_int_width: 32,
            default_float_width: 64,
            inject_prelude: true,
        }
    }
}
