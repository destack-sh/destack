use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{File, FileId};
use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::{
    CompilerOptions, ConditionCatalog, ConditionGate, DependencyMap, DiagnosticPolicy,
    FormatterOptions, LinterOptions, Policy, Product, ProfileOptions, RuntimeOptions, Target,
    Vendor, builtin_modes, builtin_roles, parse_jsonc_file, validate_dependency_map,
};

/// Destack configuration document.
#[derive(Debug, Deserialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(title = "Destack"))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Destack {
    /// Package name.
    pub name: Option<String>,
    /// Package version.
    pub version: Option<String>,
    /// Whether the package is private.
    #[serde(rename = "private")]
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
    /// Repository wide workspace package and group configuration.
    workspace: Option<WorkspaceLayout>,
    /// Config path inherited before this config.
    pub extends: Option<String>,
    /// Specific files to include in the project.
    pub files: Vec<String>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files to exclude.
    pub exclude: Vec<String>,
    /// Package dependencies.
    pub dependencies: DependencyMap,
    /// Vendored dependency resolution declaration.
    pub vendoring: Vendor,
    /// Compiler configuration.
    pub compiler: CompilerOptions,
    /// Package policy declarations and rules.
    pub policy: Policy,
    /// Runtime configuration.
    pub runtime: RuntimeOptions,
    /// Formatter configuration.
    pub formatter: FormatterOptions,
    /// Linter configuration.
    pub linter: LinterOptions,
    /// Build targets.
    pub targets: IndexMap<String, Target>,
    /// Deliverable products.
    pub products: IndexMap<String, Product>,
    /// Named profiles for semantic configuration.
    pub profiles: IndexMap<String, ProfileOptions>,
    /// Named source graph conditions.
    pub conditions: ConditionCatalog,
    /// Default target for the package.
    pub default_target: Option<String>,
    /// Default product for the package.
    pub default_product: Option<String>,
}

impl Destack {
    /// Return the declared `extends` specifiers in order.
    pub(crate) fn extends(&self) -> impl Iterator<Item = &str> {
        self.extends.iter().map(String::as_str)
    }

    /// Complete derived config fields after deserialization.
    pub(crate) fn finish(&mut self) -> Result<(), serde_json::Error> {
        let mut modes = builtin_modes();
        modes.extend(std::mem::take(&mut self.conditions.modes));
        self.conditions.modes = modes;

        let mut roles = builtin_roles();
        roles.extend(std::mem::take(&mut self.conditions.roles));
        self.conditions.roles = roles;

        let mut aliases = IndexMap::new();
        for name in self.conditions.modes.keys() {
            insert_condition_alias(&mut aliases, name, ConditionGate::mode(name.clone()))?;
        }
        for name in self.conditions.roles.keys() {
            insert_condition_alias(&mut aliases, name, ConditionGate::role(name.clone()))?;
        }
        for name in self.conditions.features.keys() {
            insert_condition_alias(&mut aliases, name, ConditionGate::feature(name.clone()))?;
        }
        for name in self.conditions.tags.keys() {
            insert_condition_alias(&mut aliases, name, ConditionGate::tag(name.clone()))?;
        }
        for (name, alias) in std::mem::take(&mut self.conditions.aliases) {
            if aliases.contains_key(&name) {
                let error = format!("condition alias '{name}' conflicts with a condition name");
                return Err(serde_json::Error::io(invalid_config_error(error)));
            }
            if alias.is_empty() {
                let error = format!("condition alias '{name}' must define at least one selector");
                return Err(serde_json::Error::io(invalid_config_error(error)));
            }

            aliases.insert(name, alias);
        }
        self.conditions.aliases = aliases;

        Ok(())
    }

    /// Validate resolved configuration invariants.
    pub fn validate(&self) -> Result<(), String> {
        self.policy.validate()?;

        for target in self.targets.values() {
            target.policy.validate()?;
        }
        for product in self.products.values() {
            product.policy.validate()?;
            product.validate()?;
        }
        for (product_name, product) in &self.products {
            for (role, target_name) in &product.targets {
                if !self.targets.contains_key(target_name) {
                    let error = format!(
                        "product '{product_name}' role '{role}' references unknown target '{target_name}'"
                    );
                    return Err(error);
                }
            }
        }

        validate_extends("mode", &self.conditions.modes, |mode| &mode.extends)?;
        validate_extends("role", &self.conditions.roles, |role| &role.extends)?;
        validate_extends("feature", &self.conditions.features, |feature| {
            &feature.extends
        })?;
        validate_extends("tag", &self.conditions.tags, |tag| &tag.extends)?;

        Ok(())
    }
}

/// Return the JSON schema for `destack.json`.
#[cfg(feature = "schema")]
pub fn destack_schema() -> schemars::Schema {
    schemars::schema_for!(Destack)
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

/// Loaded `destack.json` file.
#[derive(Debug, Clone)]
pub struct DestackFile {
    /// The id of the `destack.json` file.
    pub file_id: FileId,
    /// The declaration files used to build this effective declaration.
    pub file_ids: Vec<FileId>,
    /// Path to the `destack.json` file.
    pub path: PathBuf,
    /// The directory containing the `destack.json` file.
    pub directory: PathBuf,
    /// The effective parsed declaration.
    pub destack: Destack,
    /// The effective declaration source.
    source: Value,
}

impl std::ops::Deref for DestackFile {
    type Target = Destack;

    fn deref(&self) -> &Self::Target {
        &self.destack
    }
}

impl DestackFile {
    /// Parse one `destack.json` configuration from one file.
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
        let source = parse_jsonc_file(file)?;

        Self::from_file(file.id, vec![file.id], path, source)
    }

    /// Return the declared `extends` specifiers in order.
    pub fn extends(&self) -> impl Iterator<Item = &str> {
        self.destack.extends()
    }

    /// Validate resolved configuration invariants.
    pub fn validate(&self) -> Result<(), serde_json::Error> {
        self.destack
            .validate()
            .map_err(|error| serde_json::Error::io(Error::new(ErrorKind::InvalidData, error)))?;

        Ok(())
    }

    /// Return explicit workspace package root patterns.
    pub fn workspace_packages(&self) -> Option<&[String]> {
        self.destack
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.packages.as_deref())
    }

    /// Return workspace member groups.
    pub fn workspace_groups(&self) -> Option<&IndexMap<String, Vec<String>>> {
        self.destack
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.groups.as_ref())
    }

    /// Inherit settings from one parent configuration.
    pub fn extend_from(&mut self, parent: &Self) -> Result<(), serde_json::Error> {
        let mut source = Self::merge_source(&parent.source, &self.source);
        let destack: Destack = serde_json::from_value(source.clone())?;
        let compiler = destack.compiler.clone();
        let file_ids = merge_file_ids(&parent.file_ids, &self.file_ids);

        // preserve monotonic compiler restrictions
        Self::apply_parent_restrictions(&mut source, &compiler, &parent.compiler)?;

        *self = Self::from_file(self.file_id, file_ids, self.path.clone(), source)?;

        Ok(())
    }

    /// Build one configuration from an effective source value.
    fn from_file(
        file_id: FileId,
        file_ids: Vec<FileId>,
        path: PathBuf,
        source: Value,
    ) -> Result<Self, serde_json::Error> {
        let mut file: Destack = serde_json::from_value(source.clone())?;

        file.finish()?;
        file.validate()
            .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;
        validate_dependency_map(Some(&file.dependencies))
            .map_err(|error| serde_json::Error::io(invalid_config_error(error)))?;

        let directory = path.parent().map(PathBuf::from).ok_or_else(|| {
            serde_json::Error::io(Error::new(
                ErrorKind::InvalidData,
                "destack.json must have a parent directory",
            ))
        })?;

        Ok(Self {
            file_id,
            file_ids,
            path,
            directory,
            source,
            destack: file,
        })
    }

    /// Merge one child source value over one parent source value.
    fn merge_source(parent: &Value, child: &Value) -> Value {
        let mut parent = parent.clone();

        // drop inheritance directives before merging
        if let Value::Object(parent) = &mut parent {
            parent.remove("extends");
        }

        Self::merge_json(&parent, child)
    }

    /// Merge one child file value over one parent file value.
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

    /// Merge one child policy file value over one parent policy file value.
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
        source: &mut Value,
        compiler: &CompilerOptions,
        parent: &CompilerOptions,
    ) -> Result<(), serde_json::Error> {
        Self::apply_parent_restriction(
            source,
            "noManaged",
            compiler.no_managed,
            parent.no_managed,
        )?;
        Self::apply_parent_restriction(source, "noHeap", compiler.no_heap, parent.no_heap)?;
        Self::apply_parent_restriction(
            source,
            "noRuntime",
            compiler.no_runtime,
            parent.no_runtime,
        )?;
        Self::apply_parent_restriction(
            source,
            "noInternalImport",
            compiler.no_internal_import,
            parent.no_internal_import,
        )?;
        Self::apply_parent_restriction(
            source,
            "noImplicitDynamicDispatch",
            compiler.no_implicit_dynamic_dispatch,
            parent.no_implicit_dynamic_dispatch,
        )?;

        Ok(())
    }

    /// Apply one parent restriction when it is stricter than the child.
    fn apply_parent_restriction(
        source: &mut Value,
        key: &str,
        compiler_policy: DiagnosticPolicy,
        parent_policy: DiagnosticPolicy,
    ) -> Result<(), serde_json::Error> {
        if !parent_policy.is_stricter_than(compiler_policy) {
            return Ok(());
        }

        let Some(source) = source.as_object_mut() else {
            return Err(serde_json::Error::io(invalid_config_error(
                "effective destack declaration must be an object",
            )));
        };

        let compiler_json = source
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
fn merge_file_ids(parent: &[FileId], child: &[FileId]) -> Vec<FileId> {
    let mut file_ids = Vec::with_capacity(parent.len() + child.len());

    for file_id in parent.iter().chain(child) {
        if !file_ids.contains(file_id) {
            file_ids.push(*file_id);
        }
    }

    file_ids
}

/// Return one diagnostic policy file value.
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
