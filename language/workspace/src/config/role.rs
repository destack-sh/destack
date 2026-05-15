use indexmap::IndexMap;
use serde::Deserialize;

use super::{
    DependencyJsonMap, DependencyMap, dependency_options_from_json, validate_dependency_json_map,
};

/// Built-in source graph role declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Role {
    /// Stable role name.
    pub name: &'static str,
    /// Human-readable role description.
    pub description: &'static str,
}

impl Role {
    /// Client source graph role.
    pub const CLIENT: Self = Self {
        name: "client",
        description: "Client role.",
    };

    /// Server source graph role.
    pub const SERVER: Self = Self {
        name: "server",
        description: "Server role.",
    };

    /// Built-in source graph roles.
    pub const BUILTINS: &'static [Self] = &[Self::CLIENT, Self::SERVER];

    /// Return normalized options for this built-in role.
    pub fn options(self) -> RoleOptions {
        RoleOptions {
            description: Some(self.description.to_string()),
            ..RoleOptions::default()
        }
    }
}

/// Named source graph role options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoleOptions {
    /// Human-readable role description.
    pub description: Option<String>,
    /// Role labels.
    pub labels: IndexMap<String, String>,
    /// Role names included before this role.
    pub extends: Vec<String>,
    /// Dependencies enabled by this role.
    pub dependencies: DependencyMap,
}

impl RoleOptions {
    /// Convert from one JSON role.
    pub fn from_json(json: &RoleJson) -> Self {
        Self {
            description: json.description.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            extends: json
                .extends
                .as_ref()
                .map(RoleExtends::names)
                .unwrap_or_default(),
            dependencies: dependency_options_from_json(&json.dependencies),
        }
    }
}

/// Return the built-in source graph roles.
pub fn builtin_roles() -> IndexMap<String, RoleOptions> {
    Role::BUILTINS
        .iter()
        .map(|role| (role.name.to_string(), role.options()))
        .collect()
}

/// Source graph role JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RoleJson {
    /// Human-readable role description.
    pub description: Option<String>,
    /// Role labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Role names included before this role.
    pub extends: Option<RoleExtends>,
    /// Dependencies enabled by this role.
    pub dependencies: Option<DependencyJsonMap>,
}

impl RoleJson {
    /// Validate one source graph role declaration.
    pub fn validate(&self) -> Result<(), String> {
        validate_dependency_json_map(self.dependencies.as_ref())
    }
}

/// Role extends field from `destack.json`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RoleExtends {
    /// Extend one role.
    One(String),
    /// Extend many roles in order.
    Many(Vec<String>),
}

impl RoleExtends {
    /// Return the referenced role names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(role) => vec![role.clone()],
            Self::Many(roles) => roles.clone(),
        }
    }
}
