use super::ModulePath;

/// One generated schema module.
pub(crate) struct Module {
    /// Generated module path.
    pub(crate) path: ModulePath,
    /// Type keys in source order.
    pub(crate) keys: Vec<String>,
}
