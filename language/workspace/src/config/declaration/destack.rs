use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileId};
use serde_json::Value;

use crate::config::{
    DestackJson, PackageOptions, ProfileConfig, ProfileConfigJson, TargetJson, TargetOptions,
    WorkspaceOptions, environment_options_from_json, extend_environment_options, parse_jsonc_file,
    runtime_options_with_base,
};

/// Parsed `destack.json` declaration.
#[derive(Debug, Clone)]
pub struct DestackDeclaration {
    /// The id of the `destack.json` file.
    pub file_id: FileId,
    /// Path to the `destack.json` file.
    pub path: PathBuf,
    /// The directory containing the `destack.json` file.
    pub directory: PathBuf,
    /// The raw JSON content of the `destack.json` file.
    pub json: DestackJson,
    /// The raw declaration JSON for exact child over parent merging.
    raw_json: Value,
    /// The effective package options after declaration inheritance.
    package_options: PackageOptions,
    /// The effective workspace options after declaration inheritance.
    workspace_options: WorkspaceOptions,
}

impl DestackDeclaration {
    /// Parse one `destack.json` declaration from one file.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
        let raw_json = parse_jsonc_file(file)?;
        let json: DestackJson = serde_json::from_value(raw_json.clone())?;

        json.linter.validate().map_err(|error| {
            serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;

        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .expect("destack.json must have a valid path");
        let directory = path
            .parent()
            .expect("destack.json must have a parent directory")
            .to_path_buf();
        let package_options = PackageOptions::from(&json);
        let workspace_options = WorkspaceOptions::from(&json);

        Ok(Self {
            file_id: file.id,
            path,
            directory,
            json,
            raw_json,
            package_options,
            workspace_options,
        })
    }

    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        self.json.extends()
    }

    /// Inherit settings from one parent declaration.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub fn extend_from(&mut self, parent: &Self) {
        let parent_package = &parent.package_options;
        let parent_workspace = &parent.workspace_options;

        // package metadata
        if self.json.name.is_none() {
            self.package_options.name = parent_package.name.clone();
        }
        if self.json.version.is_none() {
            self.package_options.version = parent_package.version.clone();
        }
        if self.json.r#private.is_none() {
            self.package_options.is_private = parent_package.is_private;
        }
        if self.json.description.is_none() {
            self.package_options.description = parent_package.description.clone();
        }
        if self.json.license.is_none() {
            self.package_options.license = parent_package.license.clone();
        }
        if self.json.repository.is_none() {
            self.package_options.repository = parent_package.repository.clone();
        }
        if self.json.homepage.is_none() {
            self.package_options.homepage = parent_package.homepage.clone();
        }
        if self.json.keywords.is_none() {
            self.package_options.keywords = parent_package.keywords.clone();
        }
        if self.json.package_manager.is_none() {
            self.package_options.package_manager = parent_package.package_manager.clone();
        }
        if self.json.module_type.is_none() {
            self.package_options.module_type = parent_package.module_type.clone();
        }
        if self.json.engines.is_none() {
            self.package_options.engines = parent_package.engines.clone();
        }
        if self.json.exports.is_none() {
            self.package_options.exports = parent_package.exports.clone();
        }
        if self.json.imports.is_none() {
            self.package_options.imports = parent_package.imports.clone();
        }
        if self.json.dependencies.is_none() {
            self.package_options.dependencies = parent_package.dependencies.clone();
        }
        if self.json.dev_dependencies.is_none() {
            self.package_options.dev_dependencies = parent_package.dev_dependencies.clone();
        }
        if self.json.peer_dependencies.is_none() {
            self.package_options.peer_dependencies = parent_package.peer_dependencies.clone();
        }
        if self.json.optional_dependencies.is_none() {
            self.package_options.optional_dependencies =
                parent_package.optional_dependencies.clone();
        }

        // workspace membership
        if let Some(workspace) = self.json.workspace.as_ref() {
            let mut membership = self.workspace_options.membership.clone();
            membership.extend_from(&parent_workspace.membership);
            self.workspace_options.membership = membership;

            if workspace.members.is_none() {
                self.workspace_options.membership.members =
                    parent_workspace.membership.members.clone();
            }
            if workspace.groups.is_none() {
                self.workspace_options.membership.groups =
                    parent_workspace.membership.groups.clone();
            }
        } else {
            self.workspace_options.membership = parent_workspace.membership.clone();
        }

        // source selection
        if self.package_options.files.is_empty() {
            self.package_options.files = parent_package.files.clone();
        }
        if self.package_options.include.is_empty() {
            self.package_options.include = parent_package.include.clone();
        }
        if self.package_options.exclude.is_empty() {
            self.package_options.exclude = parent_package.exclude.clone();
        }

        // compiler
        let parent_compiler = &parent_package.compiler;
        let compiler = &mut self.package_options.compiler;

        if compiler.base_url.is_none() {
            compiler.base_url = parent_compiler.base_url.clone();
        }
        if compiler.paths.is_none() {
            compiler.paths = parent_compiler.paths.clone();
        }
        if compiler.lib.is_empty() {
            compiler.lib = parent_compiler.lib.clone();
        }
        if compiler.types.is_empty() {
            compiler.types = parent_compiler.types.clone();
        }
        if compiler.environment.is_none() {
            compiler.environment = parent_compiler.environment.clone();
        }
        if compiler.profile.is_none() {
            compiler.profile = parent_compiler.profile.clone();
        }
        if compiler.mode.is_none() {
            compiler.mode = parent_compiler.mode.clone();
        }
        if compiler.comptime_env.is_none() {
            compiler.comptime_env = parent_compiler.comptime_env.clone();
        }

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

        if compiler.root_dir.is_none() {
            compiler.root_dir = parent_compiler.root_dir.clone();
        }
        if compiler.out_dir.is_none() {
            compiler.out_dir = parent_compiler.out_dir.clone();
        }
        if compiler.declaration_dir.is_none() {
            compiler.declaration_dir = parent_compiler.declaration_dir.clone();
        }
        if self.json.compiler.declaration_map.is_none() {
            compiler.declaration_map = parent_compiler.declaration_map;
        }
        if self.json.compiler.no_emit.is_none() {
            compiler.no_emit = parent_compiler.no_emit;
        }
        if compiler.tsconfig.is_none() {
            compiler.tsconfig = parent_compiler.tsconfig.clone();
        }
        if self.json.compiler.module_resolution.is_none() {
            compiler.module_resolution = parent_compiler.module_resolution;
        }
        if self.json.compiler.allow_arbitrary_extensions.is_none() {
            compiler.allow_arbitrary_extensions = parent_compiler.allow_arbitrary_extensions;
        }
        if self.json.compiler.allow_importing_ts_extensions.is_none() {
            compiler.allow_importing_ts_extensions = parent_compiler.allow_importing_ts_extensions;
        }
        if self.json.compiler.resolve_package_json_exports.is_none() {
            compiler.resolve_package_json_exports = parent_compiler.resolve_package_json_exports;
        }
        if self.json.compiler.resolve_package_json_imports.is_none() {
            compiler.resolve_package_json_imports = parent_compiler.resolve_package_json_imports;
        }
        if self.json.compiler.custom_conditions.is_none() {
            compiler.custom_conditions = parent_compiler.custom_conditions.clone();
        }
        if self.json.compiler.node_linker.is_none() {
            compiler.node_linker = parent_compiler.node_linker;
        }
        if self.json.compiler.module_detection.is_none() {
            compiler.module_detection = parent_compiler.module_detection;
        }
        if self.json.compiler.js_as_jsx.is_none() {
            compiler.js_as_jsx = parent_compiler.js_as_jsx;
        }
        if self.json.compiler.verbatim_module_syntax.is_none() {
            compiler.verbatim_module_syntax = parent_compiler.verbatim_module_syntax;
        }
        if self
            .json
            .compiler
            .rewrite_relative_import_extensions
            .is_none()
        {
            compiler.rewrite_relative_import_extensions =
                parent_compiler.rewrite_relative_import_extensions;
        }

        // formatter
        let child_formatter = &self.json.formatter;
        let formatter = &mut self.package_options.formatter;
        let parent_formatter = &parent_package.formatter;

        if child_formatter.line_ending.is_none() {
            formatter.line_ending = parent_formatter.line_ending;
        }
        if child_formatter.indent_style.is_none() && child_formatter.use_tabs.is_none() {
            formatter.indent_style = parent_formatter.indent_style;
        }
        if child_formatter.indent_width.is_none() {
            formatter.indent_width = parent_formatter.indent_width;
        }
        if child_formatter.line_width.is_none() {
            formatter.line_width = parent_formatter.line_width;
        }
        if child_formatter.quote_style.is_none() && child_formatter.single_quote.is_none() {
            formatter.quote_style = parent_formatter.quote_style;
        }
        if child_formatter.trailing_comma.is_none() {
            formatter.trailing_comma = parent_formatter.trailing_comma;
        }
        if child_formatter.bracket_spacing.is_none() {
            formatter.bracket_spacing = parent_formatter.bracket_spacing;
        }
        if child_formatter.arrow_parens.is_none() {
            formatter.arrow_parentheses = parent_formatter.arrow_parentheses;
        }
        if child_formatter.quote_props.is_none() {
            formatter.quote_property = parent_formatter.quote_property;
        }
        if child_formatter.bracket_same_line.is_none() {
            formatter.bracket_same_line = parent_formatter.bracket_same_line;
        }
        if child_formatter.single_attribute_per_line.is_none() {
            formatter.single_attribute_per_line = parent_formatter.single_attribute_per_line;
        }
        if child_formatter.organize_imports.is_none() {
            formatter.organize_imports = parent_formatter.organize_imports;
        }
        if child_formatter.import_sort_order.is_none() {
            formatter.import_sort_order = parent_formatter.import_sort_order;
        }

        // linter
        let child_linter = &self.json.linter;
        let linter = &mut self.package_options.linter;
        let parent_linter = &parent_package.linter;

        if child_linter.enabled.is_none() {
            linter.enabled = parent_linter.enabled;
        }
        if child_linter.rules.preset.is_none()
            && child_linter.rules.recommended.is_none()
            && child_linter.rules.all.is_none()
        {
            linter.preset = parent_linter.preset;
        }
        for (category, severity) in &parent_linter.categories {
            if !linter.categories.contains_key(category) {
                linter.categories.insert(*category, *severity);
            }
        }
        for (rule, severity) in &parent_linter.overrides {
            if !linter.overrides.contains_key(rule) {
                linter.overrides.insert(rule.clone(), *severity);
            }
        }

        // cache
        if self.json.cache.mode.is_none() {
            self.workspace_options.cache.mode = parent_workspace.cache.mode;
        }

        // runtime
        self.package_options.runtime =
            runtime_options_with_base(&parent_package.runtime, Some(&self.json.runtime));

        // declaration maps
        let mut environments = environment_options_from_json(&self.json.environments);
        extend_environment_options(&mut environments, &parent_package.environments);
        self.package_options.environments = environments;

        // watch and daemon
        if self.json.watch.debounce_ms.is_none() {
            self.package_options.watch.debounce_ms = parent_package.watch.debounce_ms;
        }
        if self.json.watch.poll_interval_ms.is_none() {
            self.package_options.watch.poll_interval_ms = parent_package.watch.poll_interval_ms;
        }
        if self.json.daemon.idle_shutdown_ms.is_none() {
            self.package_options.daemon.idle_shutdown_ms = parent_package.daemon.idle_shutdown_ms;
        }

        // targets
        if let Some(targets) = &self.json.targets {
            for name in targets.keys() {
                let target_json = self.merged_target_json(parent, name).unwrap_or_else(|| {
                    panic!("failed to merge target declaration during inheritance: target={name}")
                });
                let options = TargetOptions::from_json_with_runtime(
                    &target_json,
                    &self.package_options.runtime,
                );
                self.package_options.targets.insert(name.clone(), options);
            }
        }
        for (name, target) in &parent_package.targets {
            if !self.package_options.targets.contains_key(name) {
                self.package_options
                    .targets
                    .insert(name.clone(), target.clone());
            }
        }

        // profiles
        if let Some(profiles) = &self.json.profiles {
            for name in profiles.keys() {
                let profile_json = self.merged_profile_json(parent, name).unwrap_or_else(|| {
                    panic!("failed to merge profile declaration during inheritance: profile={name}")
                });
                let profile = ProfileConfig::from_json(&profile_json);
                self.package_options.profiles.insert(name.clone(), profile);
            }
        }
        for (name, profile) in &parent_package.profiles {
            if !self.package_options.profiles.contains_key(name) {
                self.package_options
                    .profiles
                    .insert(name.clone(), profile.clone());
            }
        }
        for (name, mode) in &parent_package.modes {
            if !self.package_options.modes.contains_key(name) {
                self.package_options
                    .modes
                    .insert(name.clone(), mode.clone());
            }
        }
        if self.package_options.default_target.is_none() {
            self.package_options.default_target = parent_package.default_target.clone();
        }

        // keep the workspace package defaults aligned
        self.workspace_options.package = self.package_options.clone();
    }

    /// Derive effective package options from this declaration.
    pub fn package_options(&self) -> PackageOptions {
        self.package_options.clone()
    }

    /// Derive effective workspace options from this declaration.
    pub fn workspace_options(&self) -> WorkspaceOptions {
        self.workspace_options.clone()
    }

    /// Return one merged target declaration JSON for one inherited target name.
    fn merged_target_json(&self, parent: &Self, name: &str) -> Option<TargetJson> {
        let child_json = self.raw_named_json("targets", name)?;
        let merged_json = if let Some(parent_json) = parent.raw_named_json("targets", name) {
            Self::merge_json(parent_json, child_json)
        } else {
            child_json.clone()
        };

        serde_json::from_value(merged_json).ok()
    }

    /// Return one merged profile declaration JSON for one inherited profile name.
    fn merged_profile_json(&self, parent: &Self, name: &str) -> Option<ProfileConfigJson> {
        let child_json = self.raw_named_json("profiles", name)?;
        let merged_json = if let Some(parent_json) = parent.raw_named_json("profiles", name) {
            Self::merge_json(parent_json, child_json)
        } else {
            child_json.clone()
        };

        serde_json::from_value(merged_json).ok()
    }

    /// Return one named raw JSON entry from one declaration section.
    fn raw_named_json<'a>(&'a self, section: &str, name: &str) -> Option<&'a Value> {
        self.raw_json.get(section)?.get(name)
    }

    /// Merge one child JSON value over one parent JSON value.
    fn merge_json(parent: &Value, child: &Value) -> Value {
        match (parent, child) {
            (Value::Object(parent), Value::Object(child)) => {
                let mut merged = parent.clone();

                for (key, child_value) in child {
                    let merged_value = if let Some(parent_value) = merged.get(key) {
                        Self::merge_json(parent_value, child_value)
                    } else {
                        child_value.clone()
                    };

                    merged.insert(key.clone(), merged_value);
                }

                Value::Object(merged)
            }
            _ => child.clone(),
        }
    }
}
