/// The options for importing.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Whether to follow imports automatically.
    pub follow_imports: bool = true,
    /// Options for resolving imports.
    pub resolve: dyst_resolver::ResolveOptions,
}
