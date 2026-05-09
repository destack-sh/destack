use crate::BoxedLintRule;

mod no_alert;
mod no_anonymous_default_export;
mod no_banned_import;
mod no_bitwise;
mod no_circular_dependency;
mod no_class;
mod no_console;
mod no_continue;
mod no_default_export;
mod no_enum;
mod no_exceptions;
mod no_implicit_return;
mod no_labels;
mod no_layer_violation;
mod no_magic_numbers;
mod no_namespace;
mod no_parameter_reassignment;
mod no_placeholder_implementation;
mod no_plusplus;
mod no_process_exit;
mod no_re_export_all;
mod no_relative_parent_imports;
mod no_sequences;
mod no_shadow;
mod no_struct;
mod no_ternary;
mod no_unused_modules;
mod no_warning_comments;
mod no_wildcard_imports;

pub use no_alert::*;
pub use no_anonymous_default_export::*;
pub use no_banned_import::*;
pub use no_bitwise::*;
pub use no_circular_dependency::*;
pub use no_class::*;
pub use no_console::*;
pub use no_continue::*;
pub use no_default_export::*;
pub use no_enum::*;
pub use no_exceptions::*;
pub use no_implicit_return::*;
pub use no_labels::*;
pub use no_layer_violation::*;
pub use no_magic_numbers::*;
pub use no_namespace::*;
pub use no_parameter_reassignment::*;
pub use no_placeholder_implementation::*;
pub use no_plusplus::*;
pub use no_process_exit::*;
pub use no_re_export_all::*;
pub use no_relative_parent_imports::*;
pub use no_sequences::*;
pub use no_shadow::*;
pub use no_struct::*;
pub use no_ternary::*;
pub use no_unused_modules::*;
pub use no_warning_comments::*;
pub use no_wildcard_imports::*;

/// Get all restriction rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        Box::new(NoAlert),
        Box::new(NoAnonymousDefaultExport),
        Box::new(NoBannedImport),
        Box::new(NoBitwise),
        Box::new(NoClass),
        Box::new(NoCircularDependency),
        Box::new(NoConsole),
        Box::new(NoContinue),
        Box::new(NoDefaultExport),
        Box::new(NoEnum),
        Box::new(NoExceptions),
        Box::new(NoImplicitReturn),
        Box::new(NoLabels),
        Box::new(NoLayerViolation),
        Box::new(NoMagicNumbers),
        Box::new(NoNamespace),
        Box::new(NoParameterReassignment),
        Box::new(NoPlaceholderImplementation),
        Box::new(NoPlusplus),
        Box::new(NoProcessExit),
        Box::new(NoRelativeParentImports),
        Box::new(NoReExportAll),
        Box::new(NoSequences),
        Box::new(NoShadow),
        Box::new(NoStruct),
        Box::new(NoTernary),
        Box::new(NoUnusedModules),
        Box::new(NoWarningComments),
        Box::new(NoWildcardImports),
    ]
}
