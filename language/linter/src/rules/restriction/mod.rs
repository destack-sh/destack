use crate::BoxedLintRule;

mod no_anonymous_default_export;
mod no_bitwise;
mod no_class;
mod no_continue;
mod no_explicit_any;
mod no_labels;
mod no_magic_numbers;
mod no_namespace;
mod no_non_null_assertion;
mod no_plusplus;
mod no_sequences;
mod no_struct;
mod no_ternary;

pub use no_anonymous_default_export::*;
pub use no_bitwise::*;
pub use no_class::*;
pub use no_continue::*;
pub use no_explicit_any::*;
pub use no_labels::*;
pub use no_magic_numbers::*;
pub use no_namespace::*;
pub use no_non_null_assertion::*;
pub use no_plusplus::*;
pub use no_sequences::*;
pub use no_struct::*;
pub use no_ternary::*;

/// Get all restriction rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        Box::new(NoAnonymousDefaultExport),
        Box::new(NoExplicitAny),
        Box::new(NoBitwise),
        Box::new(NoClass),
        Box::new(NoContinue),
        Box::new(NoLabels),
        Box::new(NoMagicNumbers),
        Box::new(NoNamespace),
        Box::new(NoNonNullAssertion),
        Box::new(NoPlusplus),
        Box::new(NoSequences),
        Box::new(NoStruct),
        Box::new(NoTernary),
    ]
}
