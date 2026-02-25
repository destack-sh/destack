use std::path::PathBuf;
use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;

use destack_source::{File, FileContent, FileId};

use crate::{FormatterOptions, LinterOptions, ProfileConfig, ProfileConfigJson, RuntimeOptions};

use super::cache::{DsConfigCacheJson, DsConfigCacheOptions};
use super::compiler::{CompilerOptionsJson, DsConfigCompilerOptions};
use super::daemon::{DsConfigDaemonJson, DsConfigDaemonOptions};
use super::formatter::DsConfigFormatterJson;
use super::linter::DsConfigLinterJson;
use super::runtime::{
    DsConfigRuntimeOptionsJson, runtime_options_from_json, runtime_options_with_base,
};
use super::target::{DsConfigTargetJson, DsConfigTargetOptions};
use super::watch::{DsConfigWatchJson, DsConfigWatchOptions};

/// Destack configuration (from `dsconfig.json`, 1:1 with Package).
#[derive(Debug, Clone)]
pub struct DsConfig {
    /// The id of the `dsconfig.json` file.
    pub file_id: FileId,
    /// Path to the `dsconfig.json` file.
    pub path: PathBuf,
    /// The directory containing the `dsconfig.json` file.
    pub directory: PathBuf,
    /// The normalized/resolved configuration options.
    pub options: DsConfigOptions,
    /// The raw JSON content of the `dsconfig.json` file.
    pub content: DsConfigJson,
}

impl DsConfig {
    /// Parse a dsconfig from a File with JSON content.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
        // extract the JSON value from file content
        let FileContent::Json { value, .. } = &file.content else {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "file is not JSON",
            )));
        };

        // parse the dsconfig from the JSON value
        let dsconfig_json: DsConfigJson = serde_json::from_value(value.clone())?;

        // extract path from file (prefer file.path, fall back to URI conversion)
        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .expect("dsconfig file must have a valid path");
        let directory = path
            .parent()
            .expect("dsconfig.json must have a parent directory")
            .to_path_buf();

        // create initial options from JSON
        let options = DsConfigOptions::from(&dsconfig_json);

        let dsconfig = Self {
            file_id: file.id,
            path,
            directory,
            content: dsconfig_json,
            options,
        };
        Ok(dsconfig)
    }

    /// Inherits settings from the given dsconfig into `self`.
    ///
    /// Type checking options use "most restrictive wins" semantics:
    /// if parent is stricter, child inherits it unless explicitly overridden.
    pub fn extend_from(&mut self, dsconfig: &Self) {
        let parent = &dsconfig.options;

        // extend files/include/exclude (child overrides if non-empty)
        if self.options.files.is_empty() {
            self.options.files = parent.files.clone();
        }
        if self.options.include.is_empty() {
            self.options.include = parent.include.clone();
        }
        if self.options.exclude.is_empty() {
            self.options.exclude = parent.exclude.clone();
        }

        // extend compiler options
        let parent_compiler = &parent.compiler;
        let compiler = &mut self.options.compiler;

        // inherit module resolution (child overrides if set)
        if compiler.base_url.is_none() {
            compiler.base_url = parent_compiler.base_url.clone();
        }
        if compiler.paths.is_none() {
            compiler.paths = parent_compiler.paths.clone();
        }

        // inherit lib (child overrides if set)
        if compiler.lib.is_empty() {
            compiler.lib = parent_compiler.lib.clone();
        }
        if compiler.types.is_empty() {
            compiler.types = parent_compiler.types.clone();
        }
        if compiler.profile.is_none() {
            compiler.profile = parent_compiler.profile.clone();
        }
        if compiler.comptime_env.is_none() {
            compiler.comptime_env = parent_compiler.comptime_env.clone();
        }

        // inherit TypeScript-compatible checking options (stricter wins)
        compiler.strict = compiler.strict || parent_compiler.strict;
        compiler.always_strict = compiler.always_strict || parent_compiler.always_strict;
        if parent_compiler
            .no_implicit_any
            .is_stricter_than(compiler.no_implicit_any)
        {
            compiler.no_implicit_any = parent_compiler.no_implicit_any;
        }
        compiler.strict_null_checks =
            compiler.strict_null_checks || parent_compiler.strict_null_checks;
        if parent_compiler
            .no_implicit_this
            .is_stricter_than(compiler.no_implicit_this)
        {
            compiler.no_implicit_this = parent_compiler.no_implicit_this;
        }
        compiler.strict_function_types =
            compiler.strict_function_types || parent_compiler.strict_function_types;
        compiler.strict_bind_call_apply =
            compiler.strict_bind_call_apply || parent_compiler.strict_bind_call_apply;
        compiler.strict_builtin_iterator_return = compiler.strict_builtin_iterator_return
            || parent_compiler.strict_builtin_iterator_return;
        compiler.strict_property_initialization = compiler.strict_property_initialization
            || parent_compiler.strict_property_initialization;
        compiler.use_unknown_in_catch_variables = compiler.use_unknown_in_catch_variables
            || parent_compiler.use_unknown_in_catch_variables;
        if parent_compiler
            .no_unused_locals
            .is_stricter_than(compiler.no_unused_locals)
        {
            compiler.no_unused_locals = parent_compiler.no_unused_locals;
        }
        if parent_compiler
            .no_unused_parameters
            .is_stricter_than(compiler.no_unused_parameters)
        {
            compiler.no_unused_parameters = parent_compiler.no_unused_parameters;
        }
        if parent_compiler
            .no_implicit_returns
            .is_stricter_than(compiler.no_implicit_returns)
        {
            compiler.no_implicit_returns = parent_compiler.no_implicit_returns;
        }
        if parent_compiler
            .allow_unreachable_code
            .is_stricter_than(compiler.allow_unreachable_code)
        {
            compiler.allow_unreachable_code = parent_compiler.allow_unreachable_code;
        }
        if parent_compiler
            .allow_unused_labels
            .is_stricter_than(compiler.allow_unused_labels)
        {
            compiler.allow_unused_labels = parent_compiler.allow_unused_labels;
        }
        if parent_compiler
            .no_implicit_override
            .is_stricter_than(compiler.no_implicit_override)
        {
            compiler.no_implicit_override = parent_compiler.no_implicit_override;
        }
        if parent_compiler
            .no_fallthrough_cases_in_switch
            .is_stricter_than(compiler.no_fallthrough_cases_in_switch)
        {
            compiler.no_fallthrough_cases_in_switch =
                parent_compiler.no_fallthrough_cases_in_switch;
        }
        compiler.exact_optional_property_types =
            compiler.exact_optional_property_types || parent_compiler.exact_optional_property_types;
        if parent_compiler
            .no_unchecked_indexed_access
            .is_stricter_than(compiler.no_unchecked_indexed_access)
        {
            compiler.no_unchecked_indexed_access = parent_compiler.no_unchecked_indexed_access;
        }
        if parent_compiler
            .no_property_access_from_index_signature
            .is_stricter_than(compiler.no_property_access_from_index_signature)
        {
            compiler.no_property_access_from_index_signature =
                parent_compiler.no_property_access_from_index_signature;
        }

        // inherit Destack-specific checking (stricter wins)
        if parent_compiler.no_any.is_stricter_than(compiler.no_any) {
            compiler.no_any = parent_compiler.no_any;
        }
        if parent_compiler
            .no_unknown
            .is_stricter_than(compiler.no_unknown)
        {
            compiler.no_unknown = parent_compiler.no_unknown;
        }
        if parent_compiler
            .no_imprecise_primitives
            .is_stricter_than(compiler.no_imprecise_primitives)
        {
            compiler.no_imprecise_primitives = parent_compiler.no_imprecise_primitives;
        }
        if parent_compiler
            .no_implicit_conversions
            .is_stricter_than(compiler.no_implicit_conversions)
        {
            compiler.no_implicit_conversions = parent_compiler.no_implicit_conversions;
        }
        if parent_compiler
            .implicit_collection_conversions
            .is_stricter_than(compiler.implicit_collection_conversions)
        {
            compiler.implicit_collection_conversions =
                parent_compiler.implicit_collection_conversions;
        }
        if parent_compiler
            .no_unsafe_type_assertions
            .is_stricter_than(compiler.no_unsafe_type_assertions)
        {
            compiler.no_unsafe_type_assertions = parent_compiler.no_unsafe_type_assertions;
        }
        if parent_compiler
            .no_must_assertions
            .is_stricter_than(compiler.no_must_assertions)
        {
            compiler.no_must_assertions = parent_compiler.no_must_assertions;
        }
        if parent_compiler
            .no_definite_assignment_assertions
            .is_stricter_than(compiler.no_definite_assignment_assertions)
        {
            compiler.no_definite_assignment_assertions =
                parent_compiler.no_definite_assignment_assertions;
        }
        if parent_compiler
            .no_custom_type_guards
            .is_stricter_than(compiler.no_custom_type_guards)
        {
            compiler.no_custom_type_guards = parent_compiler.no_custom_type_guards;
        }
        if parent_compiler
            .no_unsound_variance
            .is_stricter_than(compiler.no_unsound_variance)
        {
            compiler.no_unsound_variance = parent_compiler.no_unsound_variance;
        }
        if parent_compiler
            .no_unsound_narrowing
            .is_stricter_than(compiler.no_unsound_narrowing)
        {
            compiler.no_unsound_narrowing = parent_compiler.no_unsound_narrowing;
        }
        if parent_compiler
            .deep_readonly
            .is_stricter_than(compiler.deep_readonly)
        {
            compiler.deep_readonly = parent_compiler.deep_readonly;
            if !compiler.deep_readonly_explicit {
                compiler.deep_readonly_explicit = parent_compiler.deep_readonly_explicit;
            }
        }
        if parent_compiler
            .no_untrusted_declarations
            .is_stricter_than(compiler.no_untrusted_declarations)
        {
            compiler.no_untrusted_declarations = parent_compiler.no_untrusted_declarations;
        }
        if parent_compiler
            .no_redeclared_locals
            .is_stricter_than(compiler.no_redeclared_locals)
        {
            compiler.no_redeclared_locals = parent_compiler.no_redeclared_locals;
        }
        if parent_compiler
            .no_implicit_managed
            .is_stricter_than(compiler.no_implicit_managed)
        {
            compiler.no_implicit_managed = parent_compiler.no_implicit_managed;
        }
        if parent_compiler
            .no_managed
            .is_stricter_than(compiler.no_managed)
        {
            compiler.no_managed = parent_compiler.no_managed;
        }
        if parent_compiler
            .no_runtime
            .is_stricter_than(compiler.no_runtime)
        {
            compiler.no_runtime = parent_compiler.no_runtime;
        }
        if parent_compiler
            .no_referential_equality
            .is_stricter_than(compiler.no_referential_equality)
        {
            compiler.no_referential_equality = parent_compiler.no_referential_equality;
        }
        if parent_compiler
            .no_dynamic_evaluation
            .is_stricter_than(compiler.no_dynamic_evaluation)
        {
            compiler.no_dynamic_evaluation = parent_compiler.no_dynamic_evaluation;
        }
        if parent_compiler
            .no_global_this
            .is_stricter_than(compiler.no_global_this)
        {
            compiler.no_global_this = parent_compiler.no_global_this;
        }
        if parent_compiler
            .no_dynamic_import
            .is_stricter_than(compiler.no_dynamic_import)
        {
            compiler.no_dynamic_import = parent_compiler.no_dynamic_import;
        }
        if parent_compiler
            .no_internal_import
            .is_stricter_than(compiler.no_internal_import)
        {
            compiler.no_internal_import = parent_compiler.no_internal_import;
        }
        if parent_compiler
            .no_dynamic_shapes
            .is_stricter_than(compiler.no_dynamic_shapes)
        {
            compiler.no_dynamic_shapes = parent_compiler.no_dynamic_shapes;
        }
        if parent_compiler
            .no_computed_property_access
            .is_stricter_than(compiler.no_computed_property_access)
        {
            compiler.no_computed_property_access = parent_compiler.no_computed_property_access;
        }
        if parent_compiler.no_proxy.is_stricter_than(compiler.no_proxy) {
            compiler.no_proxy = parent_compiler.no_proxy;
        }
        if parent_compiler
            .no_implicit_dynamic_dispatch
            .is_stricter_than(compiler.no_implicit_dynamic_dispatch)
        {
            compiler.no_implicit_dynamic_dispatch = parent_compiler.no_implicit_dynamic_dispatch;
        }
        if parent_compiler
            .no_exceptions
            .is_stricter_than(compiler.no_exceptions)
        {
            compiler.no_exceptions = parent_compiler.no_exceptions;
        }
        if parent_compiler
            .borrow_mode
            .is_stricter_than(compiler.borrow_mode)
        {
            compiler.borrow_mode = parent_compiler.borrow_mode;
        }

        // inherit emit settings (child overrides if set)
        if compiler.root_dir.is_none() {
            compiler.root_dir = parent_compiler.root_dir.clone();
        }
        if compiler.out_dir.is_none() {
            compiler.out_dir = parent_compiler.out_dir.clone();
        }
        if compiler.declaration_dir.is_none() {
            compiler.declaration_dir = parent_compiler.declaration_dir.clone();
        }
        if self.content.compiler_options.declaration_map.is_none() {
            compiler.declaration_map = parent_compiler.declaration_map;
        }
        if self.content.compiler_options.no_emit.is_none() {
            compiler.no_emit = parent_compiler.no_emit;
        }

        // inherit interop settings (child overrides if set)
        if compiler.tsconfig.is_none() {
            compiler.tsconfig = parent_compiler.tsconfig.clone();
        }
        if self.content.compiler_options.module_resolution.is_none() {
            compiler.module_resolution = parent_compiler.module_resolution;
        }
        if self
            .content
            .compiler_options
            .allow_arbitrary_extensions
            .is_none()
        {
            compiler.allow_arbitrary_extensions = parent_compiler.allow_arbitrary_extensions;
        }
        if self
            .content
            .compiler_options
            .allow_importing_ts_extensions
            .is_none()
        {
            compiler.allow_importing_ts_extensions = parent_compiler.allow_importing_ts_extensions;
        }
        if self
            .content
            .compiler_options
            .resolve_package_json_exports
            .is_none()
        {
            compiler.resolve_package_json_exports = parent_compiler.resolve_package_json_exports;
        }
        if self
            .content
            .compiler_options
            .resolve_package_json_imports
            .is_none()
        {
            compiler.resolve_package_json_imports = parent_compiler.resolve_package_json_imports;
        }
        if self.content.compiler_options.custom_conditions.is_none() {
            compiler.custom_conditions = parent_compiler.custom_conditions.clone();
        }
        if self.content.compiler_options.node_linker.is_none() {
            compiler.node_linker = parent_compiler.node_linker;
        }
        if self.content.compiler_options.module_detection.is_none() {
            compiler.module_detection = parent_compiler.module_detection;
        }
        if self.content.compiler_options.js_as_jsx.is_none() {
            compiler.js_as_jsx = parent_compiler.js_as_jsx;
        }
        if self.content.compiler_options.es_module_interop.is_none() {
            compiler.es_module_interop = parent_compiler.es_module_interop;
        }
        if self
            .content
            .compiler_options
            .allow_synthetic_default_imports
            .is_none()
        {
            compiler.allow_synthetic_default_imports =
                parent_compiler.allow_synthetic_default_imports;
        }
        if self
            .content
            .compiler_options
            .verbatim_module_syntax
            .is_none()
        {
            compiler.verbatim_module_syntax = parent_compiler.verbatim_module_syntax;
        }
        if self
            .content
            .compiler_options
            .rewrite_relative_import_extensions
            .is_none()
        {
            compiler.rewrite_relative_import_extensions =
                parent_compiler.rewrite_relative_import_extensions;
        }

        // inherit formatter options (child overrides if explicitly set in JSON)
        let child_json = &self.content.formatter;
        // layout
        if child_json.line_ending.is_none() {
            self.options.formatter.line_ending = parent.formatter.line_ending;
        }
        if child_json.indent_style.is_none() && child_json.use_tabs.is_none() {
            self.options.formatter.indent_style = parent.formatter.indent_style;
        }
        if child_json.indent_width.is_none() {
            self.options.formatter.indent_width = parent.formatter.indent_width;
        }
        if child_json.line_width.is_none() {
            self.options.formatter.line_width = parent.formatter.line_width;
        }
        // syntax
        if child_json.quote_style.is_none() && child_json.single_quote.is_none() {
            self.options.formatter.quote_style = parent.formatter.quote_style;
        }
        if child_json.trailing_comma.is_none() {
            self.options.formatter.trailing_comma = parent.formatter.trailing_comma;
        }
        if child_json.bracket_spacing.is_none() {
            self.options.formatter.bracket_spacing = parent.formatter.bracket_spacing;
        }
        if child_json.arrow_parens.is_none() {
            self.options.formatter.arrow_parentheses = parent.formatter.arrow_parentheses;
        }
        if child_json.quote_props.is_none() {
            self.options.formatter.quote_property = parent.formatter.quote_property;
        }
        // tree/jsx
        if child_json.bracket_same_line.is_none() {
            self.options.formatter.bracket_same_line = parent.formatter.bracket_same_line;
        }
        if child_json.single_attribute_per_line.is_none() {
            self.options.formatter.single_attribute_per_line =
                parent.formatter.single_attribute_per_line;
        }

        // inherit linter options (child overrides if explicitly set in JSON)
        let child_linter_json = &self.content.linter;
        if child_linter_json.enabled.is_none() {
            self.options.linter.enabled = parent.linter.enabled;
        }
        if child_linter_json.rules.preset.is_none()
            && child_linter_json.rules.recommended.is_none()
            && child_linter_json.rules.all.is_none()
        {
            self.options.linter.preset = parent.linter.preset;
        }
        // merge category overrides (child takes precedence)
        for (category, severity) in &parent.linter.categories {
            if !self.options.linter.categories.contains_key(category) {
                self.options.linter.categories.insert(*category, *severity);
            }
        }
        // merge rule overrides (child takes precedence)
        for (rule, severity) in &parent.linter.overrides {
            if !self.options.linter.overrides.contains_key(rule) {
                self.options
                    .linter
                    .overrides
                    .insert(rule.clone(), *severity);
            }
        }

        // inherit cache options (child overrides if set)
        let child_cache_json = &self.content.cache;
        if child_cache_json.mode.is_none() {
            self.options.cache.mode = parent.cache.mode;
        }
        if child_cache_json.dir.is_none() {
            self.options.cache.dir = parent.cache.dir.clone();
        }
        if child_cache_json.max_size_mb.is_none() {
            self.options.cache.max_size_mb = parent.cache.max_size_mb;
        }
        if child_cache_json.policy.is_none() {
            self.options.cache.policy = parent.cache.policy;
        }
        if child_cache_json.validate.is_none() {
            self.options.cache.validate = parent.cache.validate;
        }
        if child_cache_json.scope.is_none() {
            self.options.cache.scope = parent.cache.scope;
        }

        // inherit runtime options (child overrides when set)
        self.options.runtime_options =
            runtime_options_with_base(&parent.runtime_options, Some(&self.content.runtime_options));

        // inherit compiler incremental settings (child overrides if explicitly set in JSON)
        if self.content.compiler_options.incremental.is_none() {
            self.options.compiler.incremental = parent.compiler.incremental;
        }

        // inherit watch options (child overrides if set)
        let child_watch_json = &self.content.watch;
        if child_watch_json.debounce_ms.is_none() {
            self.options.watch.debounce_ms = parent.watch.debounce_ms;
        }
        if child_watch_json.poll_interval_ms.is_none() {
            self.options.watch.poll_interval_ms = parent.watch.poll_interval_ms;
        }

        // refresh child targets with merged runtime options
        if let Some(targets) = &self.content.targets {
            for (name, target_json) in targets {
                let options = DsConfigTargetOptions::from_json_with_runtime(
                    target_json,
                    &self.options.runtime_options,
                );
                self.options.targets.insert(name.clone(), options);
            }
        }

        // extend targets (add missing targets from parent)
        for (name, target) in &parent.targets {
            if !self.options.targets.contains_key(name) {
                self.options.targets.insert(name.clone(), target.clone());
            }
        }

        // extend profiles (add missing profiles from parent)
        for (name, profile) in &parent.profiles {
            if !self.options.profiles.contains_key(name) {
                self.options.profiles.insert(name.clone(), profile.clone());
            }
        }
        if self.options.default_target.is_none() {
            self.options.default_target = parent.default_target.clone();
        }
    }

    /// "Build" the root dsconfig in place, finalizing options.
    pub fn build(&mut self) {
        // currently no special build steps needed for dsconfig
        // this is here for symmetry with TsConfig::build()
    }
}

/// Normalized Destack package configuration options (from `dsconfig.json`).
#[derive(Debug, Clone, Default)]
pub struct DsConfigOptions {
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Compiler options.
    pub compiler: DsConfigCompilerOptions,
    /// Runtime options.
    pub runtime_options: RuntimeOptions,
    /// Formatter options.
    pub formatter: FormatterOptions,
    /// Linter options.
    pub linter: LinterOptions,
    /// Cache options.
    pub cache: DsConfigCacheOptions,
    /// Watch options.
    pub watch: DsConfigWatchOptions,
    /// Daemon options.
    pub daemon: DsConfigDaemonOptions,
    /// Build targets.
    pub targets: IndexMap<String, DsConfigTargetOptions>,
    /// Named profiles for semantic configuration.
    pub profiles: IndexMap<String, ProfileConfig>,
    /// Default target for workspace.
    pub default_target: Option<String>,
}

/// DsConfig JSON (usually from `dsconfig.json`)
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DsConfigJson {
    /// Extends other dsconfigs or tsconfigs.
    pub extends: Option<ExtendsFieldJson>,
    /// Specific files to include in the project.
    pub files: Option<Vec<String>>,
    /// Glob patterns for files to include.
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,
    /// Compiler options.
    #[serde(default)]
    pub compiler_options: CompilerOptionsJson,
    /// Runtime options.
    #[serde(default)]
    pub runtime_options: DsConfigRuntimeOptionsJson,
    /// Formatter options.
    #[serde(default)]
    pub formatter: DsConfigFormatterJson,
    /// Linter options.
    #[serde(default)]
    pub linter: DsConfigLinterJson,
    /// Cache options.
    #[serde(default)]
    pub cache: DsConfigCacheJson,
    /// Watch options.
    #[serde(default)]
    #[serde(alias = "watchOptions")]
    pub watch: DsConfigWatchJson,
    /// Daemon options.
    #[serde(default)]
    pub daemon: DsConfigDaemonJson,
    /// Build targets.
    pub targets: Option<IndexMap<String, DsConfigTargetJson>>,
    /// Named profiles for semantic configuration.
    pub profiles: Option<IndexMap<String, ProfileConfigJson>>,
    /// Default target for workspace.
    pub default_target: Option<String>,
}

impl From<&DsConfigJson> for DsConfigOptions {
    fn from(json: &DsConfigJson) -> Self {
        let compiler = DsConfigCompilerOptions::from(&json.compiler_options);
        let runtime_options = runtime_options_from_json(Some(&json.runtime_options));

        let targets = json
            .targets
            .as_ref()
            .map(|t| {
                t.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            DsConfigTargetOptions::from_json_with_runtime(v, &runtime_options),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut formatter = FormatterOptions::default();
        json.formatter.apply(&mut formatter);

        let mut linter = LinterOptions::default();
        json.linter.apply(&mut linter);

        let cache = DsConfigCacheOptions::from(&json.cache);
        let watch = DsConfigWatchOptions::from(&json.watch);
        let daemon = DsConfigDaemonOptions::from(&json.daemon);

        Self {
            files: json.files.clone().unwrap_or_default(),
            include: json.include.clone().unwrap_or_default(),
            exclude: json.exclude.clone().unwrap_or_default(),
            compiler,
            runtime_options,
            formatter,
            linter,
            cache,
            watch,
            daemon,
            targets,
            profiles: json
                .profiles
                .as_ref()
                .map(|profiles| {
                    profiles
                        .iter()
                        .map(|(name, profile)| (name.clone(), ProfileConfig::from_json(profile)))
                        .collect()
                })
                .unwrap_or_default(),
            default_target: json.default_target.clone(),
        }
    }
}

/// Value for the "extends" field of a dsconfig.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ExtendsFieldJson {
    /// Extend a single dsconfig.
    Single(String),
    /// Extend multiple dsconfigs.
    Multiple(Vec<String>),
}
