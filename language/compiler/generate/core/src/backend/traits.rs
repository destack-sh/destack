use destack_repository::Target;

/// Marker trait for code generation backends.
/// Backends implement this to provide metadata about their capabilities.
/// This trait is intentionally minimal:
/// - JS backend: generates from DIR (elaborated), doesn't need middle-end
/// - Cranelift backend: generates from MIR (optimized), needs full pipeline
pub trait CodegenBackend {
    /// Name of this backend (e.g., "js", "cranelift").
    fn name(&self) -> &'static str;

    /// Check if this backend supports the given target configuration.
    fn supports_target(&self, target: &Target) -> bool;
}
