use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Policy;

use super::App;

/// Product assembled from one or more build targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    /// Target names keyed by product role.
    pub targets: BTreeMap<String, String>,
    /// App declaration used for host integration.
    pub app: App,
    /// Product policy declarations and rules.
    pub policy: Policy,
}

impl Product {
    /// Validate product configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.targets.is_empty() {
            return Err("product must declare targets".to_string());
        }

        for (role, target) in &self.targets {
            if role.is_empty() {
                return Err("product target role cannot be empty".to_string());
            }

            if target.is_empty() {
                return Err(format!("product target for role '{role}' cannot be empty"));
            }
        }

        self.policy.validate()?;

        Ok(())
    }
}
