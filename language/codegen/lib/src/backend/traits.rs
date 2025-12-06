use destack_workspace::ModuleMir;

/// Trait for code generation backends.
///
/// Each backend (Cranelift, JS, etc.) implements this to produce output from MIR.
/// The trait is intentionally minimal - backends may expose additional methods
/// for target-specific functionality.
pub trait CodegenBackend {
    /// The output type produced by this backend.
    type Output;
    /// The error type for this backend.
    type Error: std::error::Error;

    /// Compile a MIR module to the backend's output format.
    fn compile(&self, module: &ModuleMir) -> Result<Self::Output, Self::Error>;
}
