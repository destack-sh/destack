mod constant;
mod evaluate;
mod literal;
mod reference;

/// The evaluation mode for static expression folding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticEvaluationMode {
    /// Evaluate in parametric form without concrete projection substitutions.
    Parametric,
    /// Evaluate with concrete projection substitutions.
    Instantiated,
}
