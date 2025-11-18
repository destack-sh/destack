/// The options for executing.
#[derive(Debug, Clone, Default)]
pub struct ExecuteOptions {
    /// Whether to fold constants.
    pub fold_constants: bool = true,
    /// Whether to execute static expressions.
    pub execute_static: bool = true,
}
