use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileId};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::{
    CompilerOptions, ConditionGate, ConditionOptions, ConditionOptionsJson, DependencyJsonMap,
    DependencyMap, DiagnosticPolicy, EnvironmentOptions, FeatureOptions, FormatterOptions,
    LinterOptions, ModeOptions, PolicyOptions, PolicyOptionsJson, ProductOptions,
    ProductOptionsJson, ProfileOptions, ProfileOptionsJson, RoleOptions, RuntimeOptions,
    TagOptions, TargetOptions, VendorOptions, VendorOptionsJson, builtin_modes, builtin_roles,
    dependency_options_from_json, environment_options_from_json, parse_jsonc_file,
    runtime_options_from_json, validate_dependency_json_map,
};

use super::compiler::CompilerOptionsJson;
use super::environment::EnvironmentJson;
use super::formatter::FormatterJson;
use super::linter::LinterJson;
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
    /// Package dependencies.
    pub dependencies: Option<DependencyJsonMap>,
    /// Vendored dependency resolution options.
    pub vendoring: Option<VendorOptionsJson>,
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
    /// Named source graph conditions.
    pub conditions: Option<ConditionOptionsJson>,
    /// Default target for the package.
    pub default_target: Option<String>,
    /// Default product for the package.
    pub default_product: Option<String>,
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

/// Parsed or effective `destack.json` declaration.
#[derive(Debug, Clone)]
pub struct DestackDeclaration {
    /// The id of the `destack.json` file.
    pub file_id: FileId,
    /// The declaration files used to build this effective declaration.
    pub declaration_file_ids: Vec<FileId>,
    /// Path to the `destack.json` file.
    pub path: PathBuf,
    /// The directory containing the `destack.json` file.
    pub directory: PathBuf,
    /// The effective options parsed from the declaration JSON.
    pub options: DestackOptions,
    /// The effective declaration JSON.
    declaration_json: Value,

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
    /// Package dependencies.
    pub dependencies: DependencyMap,
    /// Vendored dependency resolution options.
    pub vendoring: VendorOptions,
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
    /// Named source graph conditions.
    pub conditions: ConditionOptions,
    /// Default target for the package.
    pub default_target: Option<String>,
    /// Default product for the package.
    pub default_product: Option<String>,
    /// Workspace package root glob patterns when discovery is explicit.
    pub workspace_packages: Option<Vec<String>>,
    /// Workspace member groups.
    pub workspace_groups: IndexMap<String, Vec<String>>,
}

impl DestackDeclaration {
    /// Parse one `destack.json` declaration from one file.
    pub fn parse(file: &Arc<File>) -> Result<Self, serde_json::Error> {
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
        let declaration_json = parse_jsonc_file(file)?;

        Self::from_json(file.id, vec![file.id], path, declaration_json)
    }

    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        self.options.extends()
    }

    /// Validate resolved declaration invariants.
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

        validate_extends("mode", &self.conditions.modes, |mode| &mode.extends)
            .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
        validate_extends("role", &self.conditions.roles, |role| &role.extends)
            .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
        validate_extends("feature", &self.conditions.features, |feature| {
            &feature.extends
        })
        .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
        validate_extends("tag", &self.conditions.tags, |tag| &tag.extends)
            .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;

        Ok(())
    }

    /// Inherit settings from one parent declaration.
    pub fn extend_from(&mut self, parent: &Self) -> Result<(), serde_json::Error> {
        let mut declaration_json =
            Self::merge_declaration_json(&parent.declaration_json, &self.declaration_json);
        let options: DestackOptions = serde_json::from_value(declaration_json.clone())?;
        let compiler = CompilerOptions::from(&options.compiler);
        let declaration_file_ids =
            merge_declaration_file_ids(&parent.declaration_file_ids, &self.declaration_file_ids);

        // preserve monotonic compiler restrictions
        Self::apply_parent_restrictions(&mut declaration_json, &compiler, &parent.compiler)?;

        *self = Self::from_json(
            self.file_id,
            declaration_file_ids,
            self.path.clone(),
            declaration_json,
        )?;

        Ok(())
    }

    /// Build one declaration from effective declaration JSON.
    fn from_json(
        file_id: FileId,
        declaration_file_ids: Vec<FileId>,
        path: PathBuf,
        declaration_json: Value,
    ) -> Result<Self, serde_json::Error> {
        let options: DestackOptions = serde_json::from_value(declaration_json.clone())?;

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
        validate_dependency_json_map(options.dependencies.as_ref())
            .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
        let condition_json = options.conditions.as_ref();
        validate_named_json_map(
            "mode",
            condition_json.and_then(|conditions| conditions.modes.as_ref()),
            super::mode::ModeJson::validate,
        )?;
        validate_named_json_map(
            "role",
            condition_json.and_then(|conditions| conditions.roles.as_ref()),
            super::role::RoleJson::validate,
        )?;
        validate_named_json_map(
            "feature",
            condition_json.and_then(|conditions| conditions.features.as_ref()),
            super::feature::FeatureJson::validate,
        )?;
        validate_named_json_map(
            "tag",
            condition_json.and_then(|conditions| conditions.tags.as_ref()),
            super::tag::TagJson::validate,
        )?;

        let directory = path.parent().map(PathBuf::from).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                "destack.json must have a parent directory",
            ))
        })?;
        let compiler = CompilerOptions::from(&options.compiler);
        let dependencies = dependency_options_from_json(&options.dependencies);
        let vendoring = VendorOptions::from_json(options.vendoring.as_ref());
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

        // source graph conditions
        let conditions = condition_options_from_json(condition_json)?;
        Ok(Self {
            file_id,
            declaration_file_ids,
            path,
            directory,
            declaration_json,
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
            dependencies,
            vendoring,
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
            conditions,
            default_target: options.default_target.clone(),
            default_product: options.default_product.clone(),
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

    /// Merge one child declaration JSON value over one parent declaration JSON value.
    fn merge_declaration_json(parent: &Value, child: &Value) -> Value {
        let mut parent = parent.clone();

        // drop inheritance directives before merging
        if let Value::Object(parent) = &mut parent {
            parent.remove("extends");
        }

        Self::merge_json(&parent, child)
    }

    /// Merge one child JSON value over one parent JSON value.
    fn merge_json(parent: &Value, child: &Value) -> Value {
        match (parent, child) {
            (Value::Object(parent), Value::Object(child)) => {
                let mut merged = parent.clone();

                for (key, child_value) in child {
                    let merged_value = if let Some(parent_value) = merged.get(key) {
                        if key == "policy" {
                            Self::merge_policy_json(parent_value, child_value)
                        } else if key == "dependencies" {
                            Self::merge_dependency_json(parent_value, child_value)
                        } else {
                            Self::merge_json(parent_value, child_value)
                        }
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

    /// Merge dependency maps without merging individual dependency declarations.
    fn merge_dependency_json(parent: &Value, child: &Value) -> Value {
        match (parent, child) {
            (Value::Object(parent), Value::Object(child)) => {
                let mut merged = parent.clone();

                for (key, child_value) in child {
                    merged.insert(key.clone(), child_value.clone());
                }

                Value::Object(merged)
            }
            _ => child.clone(),
        }
    }

    /// Merge one child policy JSON value over one parent policy JSON value.
    fn merge_policy_json(parent: &Value, child: &Value) -> Value {
        let mut merged = Self::merge_json(parent, child);

        // append policy declarations instead of replacing them
        if let Value::Object(merged) = &mut merged {
            Self::merge_policy_array(merged, parent, child, "requires");
            Self::merge_policy_array(merged, parent, child, "rules");
        }

        merged
    }

    /// Merge one policy array when both declarations define it.
    fn merge_policy_array(
        merged: &mut serde_json::Map<String, Value>,
        parent: &Value,
        child: &Value,
        key: &str,
    ) {
        let Some(parent_items) = parent.get(key).and_then(Value::as_array) else {
            return;
        };
        let Some(child_items) = child.get(key).and_then(Value::as_array) else {
            return;
        };

        let mut items = Vec::with_capacity(parent_items.len() + child_items.len());
        items.extend(parent_items.iter().cloned());
        items.extend(child_items.iter().cloned());

        merged.insert(key.to_string(), Value::Array(items));
    }

    /// Apply parent restrictions that descendants cannot loosen.
    fn apply_parent_restrictions(
        declaration_json: &mut Value,
        compiler: &CompilerOptions,
        parent: &CompilerOptions,
    ) -> Result<(), serde_json::Error> {
        Self::apply_parent_restriction(
            declaration_json,
            "noManaged",
            compiler.no_managed,
            parent.no_managed,
        )?;
        Self::apply_parent_restriction(
            declaration_json,
            "noHeap",
            compiler.no_heap,
            parent.no_heap,
        )?;
        Self::apply_parent_restriction(
            declaration_json,
            "noRuntime",
            compiler.no_runtime,
            parent.no_runtime,
        )?;
        Self::apply_parent_restriction(
            declaration_json,
            "noInternalImport",
            compiler.no_internal_import,
            parent.no_internal_import,
        )?;
        Self::apply_parent_restriction(
            declaration_json,
            "noImplicitDynamicDispatch",
            compiler.no_implicit_dynamic_dispatch,
            parent.no_implicit_dynamic_dispatch,
        )?;

        Ok(())
    }

    /// Apply one parent restriction when it is stricter than the child.
    fn apply_parent_restriction(
        declaration_json: &mut Value,
        key: &str,
        compiler_policy: DiagnosticPolicy,
        parent_policy: DiagnosticPolicy,
    ) -> Result<(), serde_json::Error> {
        if !parent_policy.is_stricter_than(compiler_policy) {
            return Ok(());
        }

        let Some(declaration_json) = declaration_json.as_object_mut() else {
            return Err(serde_json::Error::io(invalid_config_error(
                "effective destack declaration must be an object",
            )));
        };

        let compiler_json = declaration_json
            .entry("compiler")
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        let Some(compiler_json) = compiler_json.as_object_mut() else {
            return Err(serde_json::Error::io(invalid_config_error(
                "effective compiler declaration must be an object",
            )));
        };

        compiler_json.insert(key.to_string(), policy_json_value(parent_policy));

        Ok(())
    }
}

/// Return one invalid config IO error.
fn invalid_config_error(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, message.into())
}

/// Convert condition declarations into normalized condition options.
fn condition_options_from_json(
    json: Option<&ConditionOptionsJson>,
) -> Result<ConditionOptions, serde_json::Error> {
    // merge built-in and declared modes
    let mut modes = builtin_modes();
    if let Some(json) = json.and_then(|conditions| conditions.modes.as_ref()) {
        modes.extend(json.iter().map(|(name, mode)| {
            let mode = ModeOptions::from_json(mode);

            (name.clone(), mode)
        }));
    }

    // merge built-in and declared roles
    let mut roles = builtin_roles();
    if let Some(json) = json.and_then(|conditions| conditions.roles.as_ref()) {
        roles.extend(json.iter().map(|(name, role)| {
            let role = RoleOptions::from_json(role);

            (name.clone(), role)
        }));
    }

    // convert declared feature and tag groups
    let features: IndexMap<String, FeatureOptions> = json
        .and_then(|conditions| conditions.features.as_ref())
        .map(|features| {
            features
                .iter()
                .map(|(name, feature)| {
                    let feature = FeatureOptions::from_json(feature);

                    (name.clone(), feature)
                })
                .collect()
        })
        .unwrap_or_default();
    let tags: IndexMap<String, TagOptions> = json
        .and_then(|conditions| conditions.tags.as_ref())
        .map(|tags| {
            tags.iter()
                .map(|(name, tag)| {
                    let tag = TagOptions::from_json(tag);

                    (name.clone(), tag)
                })
                .collect()
        })
        .unwrap_or_default();

    // create aliases for every declared source graph condition
    let mut aliases = IndexMap::new();
    for name in modes.keys() {
        insert_condition_alias(&mut aliases, name, ConditionGate::mode(name.clone()))?;
    }
    for name in roles.keys() {
        insert_condition_alias(&mut aliases, name, ConditionGate::role(name.clone()))?;
    }
    for name in features.keys() {
        insert_condition_alias(&mut aliases, name, ConditionGate::feature(name.clone()))?;
    }
    for name in tags.keys() {
        insert_condition_alias(&mut aliases, name, ConditionGate::tag(name.clone()))?;
    }

    // merge explicit aliases after checking for automatic alias collisions
    if let Some(json) = json.and_then(|conditions| conditions.aliases.as_ref()) {
        for (name, alias) in json {
            if aliases.contains_key(name) {
                let error = format!("condition alias '{name}' conflicts with a condition name");
                return Err(serde_json::Error::io(invalid_config_error(error)));
            }

            let alias = ConditionGate::from_json(alias);
            if alias.is_empty() {
                let error = format!("condition alias '{name}' must define at least one selector");
                return Err(serde_json::Error::io(invalid_config_error(error)));
            }

            aliases.insert(name.clone(), alias);
        }
    }

    Ok(ConditionOptions {
        modes,
        roles,
        features,
        tags,
        aliases,
    })
}

/// Insert one automatic condition alias.
fn insert_condition_alias(
    aliases: &mut IndexMap<String, ConditionGate>,
    name: &str,
    gate: ConditionGate,
) -> Result<(), serde_json::Error> {
    if aliases.contains_key(name) {
        let error = format!("condition alias '{name}' is declared by multiple condition groups");
        return Err(serde_json::Error::io(invalid_config_error(error)));
    }

    aliases.insert(name.to_string(), gate);

    Ok(())
}

/// Merge declaration file ids in inherited order.
fn merge_declaration_file_ids(parent: &[FileId], child: &[FileId]) -> Vec<FileId> {
    let mut file_ids = Vec::with_capacity(parent.len() + child.len());

    for file_id in parent.iter().chain(child) {
        if !file_ids.contains(file_id) {
            file_ids.push(*file_id);
        }
    }

    file_ids
}

/// Validate named JSON declarations.
fn validate_named_json_map<T>(
    kind: &str,
    items: Option<&IndexMap<String, T>>,
    validate: impl Fn(&T) -> Result<(), String>,
) -> Result<(), serde_json::Error> {
    if let Some(items) = items {
        for (name, item) in items {
            validate(item).map_err(|error| {
                serde_json::Error::io(invalid_config_error(format!("{kind} '{name}': {error}")))
            })?;
        }
    }

    Ok(())
}

/// Return one diagnostic policy JSON value.
fn policy_json_value(policy: DiagnosticPolicy) -> Value {
    let value = match policy {
        DiagnosticPolicy::Allow => "allow",
        DiagnosticPolicy::Warn => "warn",
        DiagnosticPolicy::Deny => "deny",
    };

    Value::String(value.to_string())
}

/// Validate named declaration inheritance edges.
fn validate_extends<T>(
    kind: &str,
    items: &IndexMap<String, T>,
    extends: impl Fn(&T) -> &[String],
) -> Result<(), String> {
    for (name, item) in items {
        for parent in extends(item) {
            if parent == name {
                return Err(format!("{kind} '{name}' extends itself"));
            }

            if !items.contains_key(parent) {
                return Err(format!("{kind} '{name}' extends unknown {kind} '{parent}'"));
            }
        }
    }

    Ok(())
}
