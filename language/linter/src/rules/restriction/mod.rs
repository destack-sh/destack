use crate::BoxedLintRule;

mod no_anonymous_default_export;
mod no_bitwise;
mod no_class;
mod no_continue;
mod no_default_export;
mod no_delete;
mod no_enum;
mod no_explicit_any;
mod no_implicit_return;
mod no_labels;
mod no_magic_numbers;
mod no_namespace;
mod no_non_null_assertion;
mod no_null;
mod no_placeholder_implementation;
mod no_plusplus;
mod no_re_export_all;
mod no_sequences;
mod no_struct;
mod no_ternary;
mod no_warning_comments;
mod no_wildcard_imports;
mod strict_boolean_expressions;

pub use no_anonymous_default_export::*;
pub use no_bitwise::*;
pub use no_class::*;
pub use no_continue::*;
pub use no_default_export::*;
pub use no_delete::*;
pub use no_enum::*;
pub use no_explicit_any::*;
pub use no_implicit_return::*;
pub use no_labels::*;
pub use no_magic_numbers::*;
pub use no_namespace::*;
pub use no_non_null_assertion::*;
pub use no_null::*;
pub use no_placeholder_implementation::*;
pub use no_plusplus::*;
pub use no_re_export_all::*;
pub use no_sequences::*;
pub use no_struct::*;
pub use no_ternary::*;
pub use no_warning_comments::*;
pub use no_wildcard_imports::*;
pub use strict_boolean_expressions::*;

/// Get all restriction rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        Box::new(NoAnonymousDefaultExport),
        Box::new(NoExplicitAny),
        Box::new(NoBitwise),
        Box::new(NoClass),
        Box::new(NoContinue),
        Box::new(NoDefaultExport),
        Box::new(NoDelete),
        Box::new(NoEnum),
        Box::new(NoImplicitReturn),
        Box::new(NoLabels),
        Box::new(NoMagicNumbers),
        Box::new(NoNamespace),
        Box::new(NoNonNullAssertion),
        Box::new(NoNull),
        Box::new(NoPlaceholderImplementation),
        Box::new(NoPlusplus),
        Box::new(NoReExportAll),
        Box::new(NoSequences),
        Box::new(NoStruct),
        Box::new(NoTernary),
        Box::new(StrictBooleanExpressions),
        Box::new(NoWarningComments),
        Box::new(NoWildcardImports),
    ]
}
