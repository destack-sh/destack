use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileId};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::{
    CompilerOptions, EnvironmentOptions, FormatterOptions, LinterOptions, ModeOptions,
    PolicyOptions, PolicyOptionsJson, ProductOptions, ProductOptionsJson, ProfileOptions,
    ProfileOptionsJson, RuntimeOptions, TargetOptions, builtin_modes,
    environment_options_from_json, extend_environment_options, parse_jsonc_file,
    runtime_options_from_json, runtime_options_with_base,
};

use super::compiler::CompilerOptionsJson;
use super::environment::EnvironmentJson;
use super::formatter::FormatterJson;
use super::linter::LinterJson;
use super::mode::ModeJson;
use super::runtime::RuntimeConfigJson;
use super::target::TargetJson;

/// Top-level options parsed from `destack.json`.
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DestackOptions {
    /// Package name.
    pub name: Option<String>,
    /// Package version.
    pub version: Option<String>,
    /// Whether the package is private.
    #[serde(rename = "private")]
    pub r#private: Option<bool>,
    /// Package description.
    pub description: Option<String>,
    /// Package license identifier.
    pub license: Option<String>,
    /// Package repository metadata.
    pub repository: Option<Value>,
    /// Package homepage.
    pub homepage: Option<String>,
    /// Package keywords.
    pub keywords: Option<Vec<String>>,
    /// Repository wide workspace package and group configuration.
    workspace: Option<WorkspaceLayout>,
    /// Config path inherited before this config.
    pub extends: Option<String>,
    /// Specific files to include in the project.
    pub files: Option<Vec<String>>,
    /// Glob patterns for files to include.
    pub include: Option<Vec<String>>,
    /// Glob patterns for files to exclude.
    pub exclude: Option<Vec<String>>,
    /// Compiler options.
    pub compiler: CompilerOptionsJson,
    /// Package policy declarations and rules.
    pub policy: PolicyOptionsJson,
    /// Runtime options.
    pub runtime: RuntimeConfigJson,
    /// Formatter options.
    pub formatter: FormatterJson,
    /// Linter options.
    pub linter: LinterJson,
    /// Build targets.
    pub targets: Option<IndexMap<String, TargetJson>>,
    /// Deliverable products.
    pub products: Option<IndexMap<String, ProductOptionsJson>>,
    /// Named reusable toolchain and runtime environments.
    pub environments: Option<IndexMap<String, EnvironmentJson>>,
    /// Named profiles for semantic configuration.
    pub profiles: Option<IndexMap<String, ProfileOptionsJson>>,
    /// Named source graph modes.
    pub modes: Option<IndexMap<String, ModeJson>>,
    /// Default target for the package.
    pub default_target: Option<String>,
}

impl DestackOptions {
    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        self.extends.iter().map(String::as_str)
    }
}

/// Workspace package layout.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct WorkspaceLayout {
    /// Package root glob patterns.
    packages: Option<Vec<String>>,
    /// Named groups of package paths.
    groups: Option<IndexMap<String, Vec<String>>>,
}

/// Parsed `destack.json` config.
#[derive(Debug, Clone)]
pub struct DestackConfig {
    /// The id of the `destack.json` file.
    pub file_id: FileId,
    /// Path to the `destack.json` file.
    pub path: PathBuf,
    /// The directory containing the `destack.json` file.
    pub directory: PathBuf,
    /// The raw options parsed from the `destack.json` file.
    pub options: DestackOptions,
    /// The raw config JSON for exact child over parent merging.
    raw_options: Value,

    /// Package name.
    pub name: Option<String>,
    /// Package version.
    pub version: Option<String>,
    /// Whether the package is private.
    pub is_private: Option<bool>,
    /// Package description.
    pub description: Option<String>,
    /// Package license identifier.
    pub license: Option<String>,
    /// Package repository metadata.
    pub repository: Option<Value>,
    /// Package homepage.
    pub homepage: Option<String>,
    /// Package keywords.
    pub keywords: Vec<String>,
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Compiler options.
    pub compiler: CompilerOptions,
    /// Package policy declarations and rules.
    pub policy: PolicyOptions,
    /// Runtime options.
    pub runtime: RuntimeOptions,
    /// Formatter options.
    pub formatter: FormatterOptions,
    /// Linter options.
    pub linter: LinterOptions,
    /// Build targets.
    pub targets: IndexMap<String, TargetOptions>,
    /// Deliverable products.
    pub products: IndexMap<String, ProductOptions>,
    /// Named reusable toolchain and runtime environments.
    pub environments: IndexMap<String, EnvironmentOptions>,
    /// Named profiles for semantic configuration.
    pub profiles: IndexMap<String, ProfileOptions>,
    /// Named source graph modes.
    pub modes: IndexMap<String, ModeOptions>,
    /// Default target for the package.
    pub default_target: Option<String>,
    /// Workspace package root glob patterns when discovery is explicit.
    pub workspace_packages: Option<Vec<String>>,
    /// Workspace member groups.
    pub workspace_groups: IndexMap<String, Vec<String>>,
}

impl DestackConfig {
    /// Parse one `destack.json` config from one file.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
        let raw_options = parse_jsonc_file(file)?;
        let options: DestackOptions = serde_json::from_value(raw_options.clone())?;

        options
            .linter
            .validate()
            .map_err(|error| serde_json::Error::io(Error::new(ErrorKind::InvalidData, error)))?;
        options
            .policy
            .validate()
            .map_err(|error| serde_json::Error::io(Error::new(ErrorKind::InvalidData, error)))?;
        if let Some(targets) = &options.targets {
            for target in targets.values() {
                target
                    .validate()
                    .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
            }
        }
        if let Some(products) = &options.products {
            for product in products.values() {
                product
                    .validate()
                    .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
            }
        }

        let path = file
            .path
            .clone()
            .or_else(|| file.uri.to_path_buf())
            .ok_or_else(|| {
                serde_json::Error::io(Error::new(
                    ErrorKind::InvalidData,
                    "destack.json must have a valid path",
                ))
            })?;
        let directory = path.parent().map(PathBuf::from).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                "destack.json must have a parent directory",
            ))
        })?;
        let compiler = CompilerOptions::from(&options.compiler);
        let mut policy = PolicyOptions::default();
        options.policy.apply_to(&mut policy);
        let runtime = runtime_options_from_json(Some(&options.runtime));
        let targets = options
            .targets
            .as_ref()
            .map(|target_map| {
                target_map
                    .iter()
                    .map(|(name, target_json)| {
                        let target = TargetOptions::from_json_with_runtime_and_policy(
                            target_json,
                            &runtime,
                            &policy,
                        )
                        .map_err(|error| {
                            serde_json::Error::io(invalid_config_error(format!(
                                "target '{name}': {error}"
                            )))
                        })?;

                        Ok((name.clone(), target))
                    })
                    .collect::<Result<IndexMap<_, _>, serde_json::Error>>()
            })
            .transpose()?
            .unwrap_or_default();
        let products = options
            .products
            .as_ref()
            .map(|product_map| {
                product_map
                    .iter()
                    .map(|(name, product_json)| {
                        let product = ProductOptions::from_json_with_policy(product_json, &policy);

                        (name.clone(), product)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut formatter = FormatterOptions::default();
        options.formatter.apply(&mut formatter);
        let mut linter = LinterOptions::default();
        options.linter.apply(&mut linter);

        // source graph modes
        let mut modes = builtin_modes();
        if let Some(mode_map) = &options.modes {
            modes.extend(
                mode_map
                    .iter()
                    .map(|(name, mode_json)| (name.clone(), ModeOptions::from_json(mode_json))),
            );
        }
        Ok(Self {
            file_id: file.id,
            path,
            directory,
            raw_options,
            name: options.name.clone(),
            version: options.version.clone(),
            is_private: options.r#private,
            description: options.description.clone(),
            license: options.license.clone(),
            repository: options.repository.clone(),
            homepage: options.homepage.clone(),
            keywords: options.keywords.clone().unwrap_or_default(),
            files: options.files.clone().unwrap_or_default(),
            include: options.include.clone().unwrap_or_default(),
            exclude: options.exclude.clone().unwrap_or_default(),
            compiler,
            policy,
            runtime,
            formatter,
            linter,
            targets,
            products,
            environments: environment_options_from_json(&options.environments),
            profiles: options
                .profiles
                .as_ref()
                .map(|profile_map| {
                    profile_map
                        .iter()
                        .map(|(name, profile_json)| {
                            let profile =
                                ProfileOptions::from_json(profile_json).map_err(|error| {
                                    serde_json::Error::io(invalid_config_error(format!(
                                        "profile '{name}': {error}"
                                    )))
                                })?;

                            Ok((name.clone(), profile))
                        })
                        .collect::<Result<IndexMap<_, _>, serde_json::Error>>()
                })
                .transpose()?
                .unwrap_or_default(),
            modes,
            default_target: options.default_target.clone(),
            workspace_packages: options
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.packages.clone()),
            workspace_groups: options
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.groups.clone())
                .unwrap_or_default(),
            options,
        })
    }

    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        self.options.extends()
    }

    /// Validate resolved config invariants.
    pub fn validate(&self) -> Result<(), serde_json::Error> {
        self.policy
            .validate()
            .map_err(|error| serde_json::Error::io(Error::new(ErrorKind::InvalidData, error)))?;

        for target in self.targets.values() {
            target.policy.validate().map_err(|error| {
                serde_json::Error::io(Error::new(ErrorKind::InvalidData, error))
            })?;
        }
        for product in self.products.values() {
            product.policy.validate().map_err(|error| {
                serde_json::Error::io(Error::new(ErrorKind::InvalidData, error))
            })?;
        }
        for (product_name, product) in &self.products {
            for (role, target_name) in &product.targets {
                if !self.targets.contains_key(target_name) {
                    let error = Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "product '{product_name}' role '{role}' references unknown target '{target_name}'"
                        ),
                    );
                    return Err(serde_json::Error::io(error));
                }
            }
        }

        for (mode, options) in &self.modes {
            for parent in &options.extends {
                if parent == mode {
                    let error = Error::new(
                        ErrorKind::InvalidData,
                        format!("mode '{mode}' extends itself"),
                    );
                    return Err(serde_json::Error::io(error));
                }
                if !self.modes.contains_key(parent) {
                    let error = Error::new(
                        ErrorKind::InvalidData,
                        format!("mode '{mode}' extends unknown mode '{parent}'"),
                    );
                    return Err(serde_json::Error::io(error));
                }
            }
        }

        Ok(())
    }

    /// Inherit settings from one parent config.
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub fn extend_from(&mut self, parent: &Self) -> Result<(), serde_json::Error> {
        // package metadata
        if self.options.name.is_none() {
            self.name = parent.name.clone();
        }
        if self.options.version.is_none() {
            self.version = parent.version.clone();
        }
        if self.options.r#private.is_none() {
            self.is_private = parent.is_private;
        }
        if self.options.description.is_none() {
            self.description = parent.description.clone();
        }
        if self.options.license.is_none() {
            self.license = parent.license.clone();
        }
        if self.options.repository.is_none() {
            self.repository = parent.repository.clone();
        }
        if self.options.homepage.is_none() {
            self.homepage = parent.homepage.clone();
        }
        if self.options.keywords.is_none() {
            self.keywords = parent.keywords.clone();
        }

        // workspace
        if let Some(workspace) = self.options.workspace.as_ref() {
            if workspace.packages.is_none() {
                self.workspace_packages = parent.workspace_packages.clone();
            }
            if workspace.groups.is_none() {
                self.workspace_groups = parent.workspace_groups.clone();
            } else {
                for (name, members) in &parent.workspace_groups {
                    if !self.workspace_groups.contains_key(name) {
                        self.workspace_groups.insert(name.clone(), members.clone());
                    }
                }
            }
        } else {
            self.workspace_packages = parent.workspace_packages.clone();
            self.workspace_groups = parent.workspace_groups.clone();
        }

        // source selection
        if self.files.is_empty() {
            self.files = parent.files.clone();
        }
        if self.include.is_empty() {
            self.include = parent.include.clone();
        }
        if self.exclude.is_empty() {
            self.exclude = parent.exclude.clone();
        }

        // compiler
        let parent_compiler = &parent.compiler;
        let compiler = &mut self.compiler;

        if compiler.environment.is_none() {
            compiler.environment = parent_compiler.environment.clone();
        }
        if compiler.profile.is_none() {
            compiler.profile = parent_compiler.profile.clone();
        }
        if compiler.modes.is_empty() {
            compiler.modes = parent_compiler.modes.clone();
        }
        if compiler.comptime_env.is_none() {
            compiler.comptime_env = parent_compiler.comptime_env.clone();
        }
        if compiler.tree.is_none() {
            compiler.tree = parent_compiler.tree.clone();
        }
        if self.options.compiler.globals.is_none() {
            compiler.globals = parent_compiler.globals.clone();
        }
        if self.options.compiler.derive.is_none() {
            compiler.derive = parent_compiler.derive.clone();
        }
        if parent_compiler
            .no_managed
            .is_stricter_than(compiler.no_managed)
        {
            compiler.no_managed = parent_compiler.no_managed;
        }
        if parent_compiler.no_heap.is_stricter_than(compiler.no_heap) {
            compiler.no_heap = parent_compiler.no_heap;
        }
        if parent_compiler
            .no_runtime
            .is_stricter_than(compiler.no_runtime)
        {
            compiler.no_runtime = parent_compiler.no_runtime;
        }
        if parent_compiler
            .no_internal_import
            .is_stricter_than(compiler.no_internal_import)
        {
            compiler.no_internal_import = parent_compiler.no_internal_import;
        }
        if parent_compiler
            .no_implicit_dynamic_dispatch
            .is_stricter_than(compiler.no_implicit_dynamic_dispatch)
        {
            compiler.no_implicit_dynamic_dispatch = parent_compiler.no_implicit_dynamic_dispatch;
        }
        if parent_compiler.no_throw.is_stricter_than(compiler.no_throw) {
            compiler.no_throw = parent_compiler.no_throw;
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
        if self.options.compiler.declaration_map.is_none() {
            compiler.declaration_map = parent_compiler.declaration_map;
        }
        if self.options.compiler.no_emit.is_none() {
            compiler.no_emit = parent_compiler.no_emit;
        }

        // policy
        self.policy =
            PolicyOptions::from_json_with_parent(Some(&self.options.policy), &parent.policy);

        // formatter
        let child_formatter = &self.options.formatter;
        let formatter = &mut self.formatter;
        let parent_formatter = &parent.formatter;

        if child_formatter.line_ending.is_none() {
            formatter.line_ending = parent_formatter.line_ending;
        }
        if child_formatter.indent_style.is_none() {
            formatter.indent_style = parent_formatter.indent_style;
        }
        if child_formatter.indent_width.is_none() {
            formatter.indent_width = parent_formatter.indent_width;
        }
        if child_formatter.line_width.is_none() {
            formatter.line_width = parent_formatter.line_width;
        }

        // linter
        let child_linter = &self.options.linter;
        let linter = &mut self.linter;
        let parent_linter = &parent.linter;

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

        // runtime
        self.runtime = runtime_options_with_base(&parent.runtime, Some(&self.options.runtime));

        // declaration maps
        let mut environments = environment_options_from_json(&self.options.environments);
        extend_environment_options(&mut environments, &parent.environments);
        self.environments = environments;

        // targets
        if let Some(targets) = &self.options.targets {
            for name in targets.keys() {
                let target_json = self.merged_target_json(parent, name)?;
                target_json
                    .validate()
                    .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
                let options = TargetOptions::from_json_with_runtime_and_policy(
                    &target_json,
                    &self.runtime,
                    &self.policy,
                )
                .map_err(|error| {
                    serde_json::Error::io(invalid_config_error(format!("target '{name}': {error}")))
                })?;
                self.targets.insert(name.clone(), options);
            }
        }
        for (name, target) in &parent.targets {
            if !self.targets.contains_key(name) {
                self.targets.insert(name.clone(), target.clone());
            }
        }

        // products
        if let Some(products) = &self.options.products {
            for name in products.keys() {
                let product_json = self.merged_product_json(parent, name)?;
                product_json
                    .validate()
                    .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
                let product = ProductOptions::from_json_with_policy(&product_json, &self.policy);
                self.products.insert(name.clone(), product);
            }
        }
        for (name, product) in &parent.products {
            if !self.products.contains_key(name) {
                self.products.insert(name.clone(), product.clone());
            }
        }

        // profiles
        if let Some(profiles) = &self.options.profiles {
            for name in profiles.keys() {
                let profile_json = self.merged_profile_json(parent, name)?;
                let profile = ProfileOptions::from_json(&profile_json).map_err(|error| {
                    serde_json::Error::io(invalid_config_error(format!(
                        "profile '{name}': {error}"
                    )))
                })?;
                self.profiles.insert(name.clone(), profile);
            }
        }
        for (name, profile) in &parent.profiles {
            if !self.profiles.contains_key(name) {
                self.profiles.insert(name.clone(), profile.clone());
            }
        }
        for (name, mode) in &parent.modes {
            if !self.modes.contains_key(name) {
                self.modes.insert(name.clone(), mode.clone());
            }
        }
        if self.default_target.is_none() {
            self.default_target = parent.default_target.clone();
        }

        Ok(())
    }

    /// Return one merged target JSON object for one inherited target name.
    fn merged_target_json(
        &self,
        parent: &Self,
        name: &str,
    ) -> Result<TargetJson, serde_json::Error> {
        let child_json = self.raw_named_json("targets", name).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                format!("failed to find target config during inheritance: target={name}"),
            ))
        })?;
        let merged_json = if let Some(parent_json) = parent.raw_named_json("targets", name) {
            Self::merge_json(parent_json, child_json)
        } else {
            child_json.clone()
        };

        serde_json::from_value(merged_json)
    }

    /// Return one merged product JSON object for one inherited product name.
    fn merged_product_json(
        &self,
        parent: &Self,
        name: &str,
    ) -> Result<ProductOptionsJson, serde_json::Error> {
        let child_json = self.raw_named_json("products", name).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                format!("failed to find product config during inheritance: product={name}"),
            ))
        })?;
        let merged_json = if let Some(parent_json) = parent.raw_named_json("products", name) {
            Self::merge_json(parent_json, child_json)
        } else {
            child_json.clone()
        };

        serde_json::from_value(merged_json)
    }

    /// Return one merged profile JSON object for one inherited profile name.
    fn merged_profile_json(
        &self,
        parent: &Self,
        name: &str,
    ) -> Result<ProfileOptionsJson, serde_json::Error> {
        let child_json = self.raw_named_json("profiles", name).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                format!("failed to find profile config during inheritance: profile={name}"),
            ))
        })?;
        let merged_json = if let Some(parent_json) = parent.raw_named_json("profiles", name) {
            Self::merge_json(parent_json, child_json)
        } else {
            child_json.clone()
        };

        serde_json::from_value(merged_json)
    }

    /// Return one named raw JSON entry from one config section.
    fn raw_named_json<'a>(&'a self, section: &str, name: &str) -> Option<&'a Value> {
        self.raw_options.get(section)?.get(name)
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

/// Return one invalid config IO error.
fn invalid_config_error(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, message.into())
}
