use serde::Deserialize;

use crate::config::{DiagnosticPolicy, DiagnosticPolicyJson};

use super::{
    BitwiseOperator, BitwiseOperatorJson, LinterOptions, WarningCommentLocation,
    WarningCommentLocationJson,
};

/// Module boundary lint options for module boundary aware rules.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintModuleComponent {
    /// The unique component name.
    pub name: String,
    /// Glob patterns used to match module paths into this component.
    pub path_patterns: Vec<String>,
}

/// One allowed dependency rule between module components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintModuleDependencyRule {
    /// The source component name.
    pub from: String,
    /// Destination components this source component may import.
    pub allow: Vec<String>,
}

/// One module dependency exception for specific module path patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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

/// Restriction-category linter JSON options.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterRestrictionJson {
    /// Bitwise operators allowed by `no-bitwise`.
    pub allowed_bitwise_operators: Option<Vec<BitwiseOperatorJson>>,
    /// Allow `x | 0` int32 cast hints in `no-bitwise`.
    pub allow_bitwise_int32_hint: Option<bool>,
    /// Console methods allowed by `no-console`.
    pub allowed_console_methods: Option<Vec<String>>,
    /// Allow labels on loop statements in `no-labels`.
    pub allow_loop_labels: Option<bool>,
    /// Allow labels on switch statements in `no-labels`.
    pub allow_switch_labels: Option<bool>,
    /// Magic numbers to allow.
    pub allowed_magic_numbers: Option<Vec<f64>>,
    /// Allow `++` and `--` in for-loop afterthoughts for `no-plusplus`.
    pub allow_plusplus_for_loop_afterthoughts: Option<bool>,
    /// Where `no-warning-comments` should match terms.
    pub warning_comment_location: Option<WarningCommentLocationJson>,
    /// Decoration characters to ignore at the start of `no-warning-comments`.
    pub warning_comment_decoration: Option<Vec<String>>,
    /// Globals to restrict.
    pub restricted_globals: Option<Vec<String>>,
    /// Import paths to restrict.
    pub restricted_imports: Option<Vec<String>>,
    /// Comment terms to warn on.
    pub warning_comment_terms: Option<Vec<String>>,
    /// Module boundary constraints for module boundary aware lint rules.
    #[serde(alias = "architecture")]
    pub module_boundaries: Option<LinterModuleBoundariesJson>,
}

impl LinterRestrictionJson {
    /// Validate restriction-category configuration values.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    /// Apply restriction-category options to one linter options struct.
    pub fn apply(&self, options: &mut LinterOptions) {
        if let Some(ref allowed_bitwise_operators) = self.allowed_bitwise_operators {
            options.restriction.allowed_bitwise_operators = allowed_bitwise_operators
                .iter()
                .copied()
                .map(Into::into)
                .collect();
        }

        if let Some(allow_bitwise_int32_hint) = self.allow_bitwise_int32_hint {
            options.restriction.allow_bitwise_int32_hint = allow_bitwise_int32_hint;
        }

        if let Some(ref allowed_console_methods) = self.allowed_console_methods {
            options.restriction.allowed_console_methods = allowed_console_methods.clone();
        }

        if let Some(allow_loop_labels) = self.allow_loop_labels {
            options.restriction.allow_loop_labels = allow_loop_labels;
        }

        if let Some(allow_switch_labels) = self.allow_switch_labels {
            options.restriction.allow_switch_labels = allow_switch_labels;
        }

        if let Some(ref allowed_magic_numbers) = self.allowed_magic_numbers {
            options.restriction.allowed_magic_numbers = allowed_magic_numbers.clone();
        }

        if let Some(allow_plusplus_for_loop_afterthoughts) =
            self.allow_plusplus_for_loop_afterthoughts
        {
            options.restriction.allow_plusplus_for_loop_afterthoughts =
                allow_plusplus_for_loop_afterthoughts;
        }

        if let Some(warning_comment_location) = self.warning_comment_location {
            options.restriction.warning_comment_location = warning_comment_location.into();
        }

        if let Some(ref warning_comment_decoration) = self.warning_comment_decoration {
            options.restriction.warning_comment_decoration = warning_comment_decoration.clone();
        }

        if let Some(ref restricted_globals) = self.restricted_globals {
            options.restriction.restricted_globals = restricted_globals.clone();
        }

        if let Some(ref restricted_imports) = self.restricted_imports {
            options.restriction.restricted_imports = restricted_imports.clone();
        }

        if let Some(ref warning_comment_terms) = self.warning_comment_terms {
            options.restriction.warning_comment_terms = warning_comment_terms.clone();
        }

        if let Some(ref module_boundaries) = self.module_boundaries {
            module_boundaries.apply(&mut options.restriction.module_boundaries);
        }
    }
}

/// Module boundary options for linter configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleBoundariesJson {
    /// Policy for modules that do not match any configured component.
    pub unknown_component_policy: Option<DiagnosticPolicyJson>,
    /// Declared module components.
    pub components: Option<Vec<LinterModuleComponentJson>>,
    /// Allowed component to component dependency rules.
    pub rules: Option<Vec<LinterModuleDependencyRuleJson>>,
    /// Explicit dependency exceptions.
    pub exceptions: Option<Vec<LinterModuleDependencyExceptionJson>>,
}

impl LinterModuleBoundariesJson {
    /// Apply module boundary options to one linter module boundary options struct.
    pub fn apply(&self, options: &mut LintModuleBoundariesOptions) {
        if let Some(unknown_component_policy) = self.unknown_component_policy {
            options.unknown_component_policy = unknown_component_policy.into();
        }

        if let Some(ref components) = self.components {
            options.components = components.iter().map(LintModuleComponent::from).collect();
        }

        if let Some(ref rules) = self.rules {
            options.dependency_rules = rules.iter().map(LintModuleDependencyRule::from).collect();
        }

        if let Some(ref exceptions) = self.exceptions {
            options.exceptions = exceptions
                .iter()
                .map(LintModuleDependencyException::from)
                .collect();
        }
    }
}

/// One module component declaration in linter JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleComponentJson {
    /// The unique component name.
    pub name: String,
    /// Glob patterns used to map modules into this component.
    #[serde(default, rename = "match")]
    pub path_patterns: Vec<String>,
}

impl From<&LinterModuleComponentJson> for LintModuleComponent {
    fn from(value: &LinterModuleComponentJson) -> Self {
        Self {
            name: value.name.clone(),
            path_patterns: value.path_patterns.clone(),
        }
    }
}

/// One module dependency rule in linter JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleDependencyRuleJson {
    /// The source component name.
    pub from: String,
    /// The list of allowed destination component names.
    #[serde(default)]
    pub allow: Vec<String>,
}

impl From<&LinterModuleDependencyRuleJson> for LintModuleDependencyRule {
    fn from(value: &LinterModuleDependencyRuleJson) -> Self {
        Self {
            from: value.from.clone(),
            allow: value.allow.clone(),
        }
    }
}

/// One module dependency exception in linter JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinterModuleDependencyExceptionJson {
    /// The source component name.
    pub from: String,
    /// The destination component name.
    pub to: String,
    /// Module path patterns where this exception is allowed.
    #[serde(default, rename = "match")]
    pub path_patterns: Vec<String>,
    /// Optional human-readable reason for the exception.
    pub reason: Option<String>,
}

impl From<&LinterModuleDependencyExceptionJson> for LintModuleDependencyException {
    fn from(value: &LinterModuleDependencyExceptionJson) -> Self {
        Self {
            from: value.from.clone(),
            to: value.to.clone(),
            path_patterns: value.path_patterns.clone(),
            reason: value.reason.clone(),
        }
    }
}
