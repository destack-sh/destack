use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{PolicyOptions, PolicyOptionsJson};

use super::{AppOptions, AppOptionsJson};

/// Product assembled from one or more build targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ProductOptions {
    /// Target names keyed by product role.
    pub targets: BTreeMap<String, String>,
    /// App declaration used for host integration.
    pub app: AppOptions,
    /// Product policy declarations and rules.
    pub policy: PolicyOptions,
}

impl ProductOptions {
    /// Convert from JSON product options.
    pub fn from_json_with_policy(json: &ProductOptionsJson, base_policy: &PolicyOptions) -> Self {
        let mut policy = base_policy.clone();

        if let Some(policy_json) = &json.policy {
            policy_json.apply_to(&mut policy);
        }

        Self {
            targets: json.targets.clone().unwrap_or_default(),
            app: json.app.as_ref().map(AppOptions::from).unwrap_or_default(),
            policy,
        }
    }
}

/// Product options JSON.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ProductOptionsJson {
    /// Target names keyed by product role.
    pub targets: Option<BTreeMap<String, String>>,
    /// App declaration used for host integration.
    pub app: Option<AppOptionsJson>,
    /// Product policy declarations and rules.
    pub policy: Option<PolicyOptionsJson>,
}

impl ProductOptionsJson {
    /// Validate product options before they are normalized.
    pub fn validate(&self) -> Result<(), String> {
        let Some(targets) = &self.targets else {
            return Err("product must declare targets".to_string());
        };

        if targets.is_empty() {
            return Err("product targets cannot be empty".to_string());
        }

        for (role, target) in targets {
            if role.is_empty() {
                return Err("product target role cannot be empty".to_string());
            }

            if target.is_empty() {
                return Err(format!("product target for role '{role}' cannot be empty"));
            }
        }

        if let Some(policy) = &self.policy {
            policy.validate()?;
        }

        Ok(())
    }
}
