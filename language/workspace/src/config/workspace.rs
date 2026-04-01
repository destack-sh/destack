use indexmap::IndexMap;
use serde::Deserialize;

/// Workspace membership declarations.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceMembershipDeclaration {
    /// Member root patterns within the repository.
    pub members: Vec<String>,
    /// Named member groups.
    pub groups: IndexMap<String, Vec<String>>,
}

impl WorkspaceMembershipDeclaration {
    /// Inherit unset workspace settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.members.is_empty() {
            self.members = parent.members.clone();
        }

        for (name, members) in &parent.groups {
            if !self.groups.contains_key(name) {
                self.groups.insert(name.clone(), members.clone());
            }
        }
    }
}

impl From<&WorkspaceJson> for WorkspaceMembershipDeclaration {
    fn from(json: &WorkspaceJson) -> Self {
        Self {
            members: json.members.clone().unwrap_or_default(),
            groups: json.groups.clone().unwrap_or_default(),
        }
    }
}

/// Workspace membership declarations.
///
/// Inputs: member root patterns and optional named groups.
/// Outputs: a repository wide project membership graph used for discovery.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceJson {
    /// Member root patterns within the repository.
    pub members: Option<Vec<String>>,
    /// Named groups of member paths.
    pub groups: Option<IndexMap<String, Vec<String>>>,
}
