/// The freshness mode for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessMode {
    /// Freshness is preserved.
    Fresh,
    /// Freshness is regularized.
    Regularized,
}

/// The widening mode for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WideningMode {
    /// Apply normal widening behavior.
    Widen,
    /// Preserve literal types without widening.
    Preserve,
}

/// The const context mode for inference.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstContext {
    /// No const context applies.
    None,
    /// A const context applies.
    Const,
    /// An explicit const assertion applies.
    AsConst,
}

/// The contextual typing mode for inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextualTypingMode {
    /// Use normal contextual typing rules.
    Default,
    /// Use satisfies-style contextual typing rules.
    Satisfies,
}
