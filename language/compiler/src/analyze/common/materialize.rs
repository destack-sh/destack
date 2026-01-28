/// The mode used when materializing types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaterializationMode {
    /// Materialize a shape without validating bounds.
    Shape,
    /// Materialize types for validation and assignability checks.
    Validation,
    /// Materialize surface types for exports and substitutions.
    Surface,
}

/// Placeholder to keep the shape mode reachable during refactors.
const _SHAPE_MODE: MaterializationMode = MaterializationMode::Shape;
