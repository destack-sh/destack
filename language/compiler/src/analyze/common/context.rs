/// The freshness state for literal types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralFreshness {
    /// Literal types are still fresh.
    Fresh,
    /// Literal types have been regularized.
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
