mod constant;
mod evaluate;
mod literal;
mod reference;

/// The evaluation mode for static expression folding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticEvaluationMode {
    /// Evaluate without receiver specialization.
    Generic,
    /// Evaluate with receiver specialization.
    Specialized,
}
