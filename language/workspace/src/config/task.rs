use indexmap::IndexMap;
use serde::Deserialize;

/// Task options.
#[derive(Debug, Clone, Default)]
pub struct TaskOptions {
    /// Command to execute.
    pub command: Option<String>,
    /// Task dependencies to run first.
    pub depends_on: Vec<String>,
    /// Extra environment variables.
    pub env: IndexMap<String, String>,
    /// Working directory override.
    pub cwd: Option<String>,
}

impl TaskOptions {
    /// Inherit unset task settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.command.is_none() {
            self.command = parent.command.clone();
        }
        if self.depends_on.is_empty() {
            self.depends_on = parent.depends_on.clone();
        }
        if self.cwd.is_none() {
            self.cwd = parent.cwd.clone();
        }
        if self.env.is_empty() {
            self.env = parent.env.clone();
        }
    }
}

impl From<&TaskJson> for TaskOptions {
    fn from(json: &TaskJson) -> Self {
        match json {
            TaskJson::Command(command) => Self {
                command: Some(command.clone()),
                ..Default::default()
            },
            TaskJson::Object(object) => Self {
                command: object.command.clone(),
                depends_on: object.depends_on.clone().unwrap_or_default(),
                env: object.env.clone().unwrap_or_default(),
                cwd: object.cwd.clone(),
            },
        }
    }
}

/// Task JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TaskJson {
    /// Shorthand command string.
    Command(String),
    /// Structured task object.
    Object(TaskObjectJson),
}

/// Structured task JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskObjectJson {
    /// Command to execute.
    pub command: Option<String>,
    /// Task dependencies to run first.
    pub depends_on: Option<Vec<String>>,
    /// Extra environment variables.
    pub env: Option<IndexMap<String, String>>,
    /// Working directory override.
    pub cwd: Option<String>,
}
