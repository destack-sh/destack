/// The options for executing.
#[derive(Debug, Clone, Default)]
pub struct ExecuteOptions {
    /// Whether to fold constants.
    pub fold_constants: bool = true,
    /// Whether to resolve static expressions.
    pub resolve_static_expressions: bool = true,
}

