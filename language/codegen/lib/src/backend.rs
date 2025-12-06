use destack_mir::NodeTree;
use destack_source::ImmutableStringPool;

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
    fn compile_module(
        &self,
        tree: &NodeTree,
        strings: &ImmutableStringPool,
    ) -> Result<Self::Output, Self::Error>;
}
