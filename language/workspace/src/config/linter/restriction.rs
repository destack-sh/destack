use serde::{Deserialize, Serialize};

use crate::config::DiagnosticPolicy;

use super::{BitwiseOperator, WarningCommentLocation};

/// Module boundary lint options for module boundary aware rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LintModuleBoundariesOptions {
    /// Policy for modules that do not match any configured component.
    pub unknown_component_policy: DiagnosticPolicy,
    /// Declared components and their path match patterns.
    pub components: Vec<LintModuleComponent>,
    /// Allowed component to component dependency rules.
    pub dependency_rules: Vec<LintModuleDependencyRule>,
    /// Explicit dependency exceptions.
    pub exceptions: Vec<LintModuleDependencyException>,
}

impl Default for LintModuleBoundariesOptions {
    fn default() -> Self {
        Self {
            unknown_component_policy: DiagnosticPolicy::Allow,
            components: Vec::new(),
            dependency_rules: Vec::new(),
            exceptions: Vec::new(),
        }
    }
}

/// One module component declaration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LintModuleComponent {
    /// The unique component name.
    pub name: String,
    /// Glob patterns used to match module paths into this component.
    pub path_patterns: Vec<String>,
}

/// One allowed dependency rule between module components.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LintModuleDependencyRule {
    /// The source component name.
    pub from: String,
    /// Destination components this source component may import.
    pub allow: Vec<String>,
}

/// One module dependency exception for specific module path patterns.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LintModuleDependencyException {
    /// The source component name for this exception.
    pub from: String,
    /// The destination component name for this exception.
    pub to: String,
    /// Module path patterns where this exception is allowed.
    pub path_patterns: Vec<String>,
    /// Optional human-readable reason for this exception.
    pub reason: Option<String>,
}

/// Restriction-category linter options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LinterRestrictionOptions {
    /// Bitwise operators allowed by `no-bitwise`.
    pub allowed_bitwise_operators: Vec<BitwiseOperator>,
    /// Allow `x | 0` int32 cast hints in `no-bitwise`.
    pub allow_bitwise_int32_hint: bool,
    /// Console methods allowed by `no-console`.
    pub allowed_console_methods: Vec<String>,
    /// Allow labels on loop statements in `no-labels`.
    pub allow_loop_labels: bool,
    /// Allow labels on switch statements in `no-labels`.
    pub allow_switch_labels: bool,
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Vec<f64>,
    /// Allow `++` and `--` in for-loop afterthoughts for `no-plusplus`.
    pub allow_plusplus_for_loop_afterthoughts: bool,
    /// Where `no-warning-comments` should match terms.
    pub warning_comment_location: WarningCommentLocation,
    /// Decoration characters to ignore at the start of `no-warning-comments`.
    pub warning_comment_decoration: Vec<String>,
    /// Globals to restrict.
    pub restricted_globals: Vec<String>,
    /// Import paths to restrict.
    pub restricted_imports: Vec<String>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Vec<String>,
    /// Module boundary constraints for module boundary aware lints.
    pub module_boundaries: LintModuleBoundariesOptions,
}

impl Default for LinterRestrictionOptions {
    fn default() -> Self {
        Self {
            allowed_bitwise_operators: Vec::new(),
            allow_bitwise_int32_hint: false,
            allowed_console_methods: Vec::new(),
            allow_loop_labels: false,
            allow_switch_labels: false,
            allowed_magic_numbers: vec![-1.0, 0.0, 1.0, 2.0],
            allow_plusplus_for_loop_afterthoughts: false,
            warning_comment_location: WarningCommentLocation::Start,
            warning_comment_decoration: Vec::new(),
            restricted_globals: Vec::new(),
            restricted_imports: Vec::new(),
            warning_comment_terms: vec![
                "TODO".to_string(),
                "FIXME".to_string(),
                "HACK".to_string(),
            ],
            module_boundaries: LintModuleBoundariesOptions::default(),
        }
    }
}
