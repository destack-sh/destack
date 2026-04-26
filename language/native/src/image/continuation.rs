/// Native continuation materialized at a managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Continuation {
    /// The logical continuation image reconstructed from native state.
    pub image: destack_engine::ContinuationImage,
}

impl Continuation {
    /// Create one continuation from a logical image.
    pub const fn new(image: destack_engine::ContinuationImage) -> Self {
        Self { image }
    }
}
