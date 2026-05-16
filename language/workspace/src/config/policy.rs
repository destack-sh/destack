use serde::{Deserialize, Serialize};

use super::ConditionSelector;
use super::runtime::RuntimeIdentitySelector;

/// Policy domain where one action is exercised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PolicyDomain {
    /// Compile-time program execution.
    Comptime,
    /// Ordinary program execution.
    #[default]
    Runtime,
}

/// Policy access result selected by one rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PolicyAccess {
    /// Allow the matching action.
    Allow,
    /// Deny the matching action.
    Deny,
}

/// Binding route used for allowed actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PolicyRoute {
    /// Execute against the live host.
    #[default]
    Host,
    /// Execute against the configured simulation model.
    Simulation,
}

/// Package selector used by static policy rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PackageSelector {
    /// Package name glob patterns.
    pub patterns: Vec<String>,
}

impl PackageSelector {
    /// Return whether this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

/// Policy subject matched by one rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PolicySubject {
    /// Package selector.
    pub package: Option<PackageSelector>,
    /// Runtime identity selector.
    pub runtime_identity: Option<RuntimeIdentitySelector>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelector>,
    /// Active mode selector.
    pub mode: Option<ConditionSelector>,
    /// Active role selector.
    pub role: Option<ConditionSelector>,
    /// Active feature selector.
    pub feature: Option<ConditionSelector>,
    /// Active tag selector.
    pub tag: Option<ConditionSelector>,
    /// Active target selector.
    pub target: Option<ConditionSelector>,
    /// Active product selector.
    pub product: Option<ConditionSelector>,
    /// Active target platform selector.
    pub platform: Option<ConditionSelector>,
    /// Active host environment selector.
    pub host: Option<ConditionSelector>,
    /// Active runtime selector.
    pub runtime: Option<ConditionSelector>,
}

impl PolicySubject {
    /// Return whether this selector matches no subject dimension.
    pub fn is_empty(&self) -> bool {
        package_selector_is_empty(&self.package)
            && self.runtime_identity.is_none()
            && self.worker.is_none()
            && condition_selector_is_empty(&self.mode)
            && condition_selector_is_empty(&self.role)
            && condition_selector_is_empty(&self.feature)
            && condition_selector_is_empty(&self.tag)
            && condition_selector_is_empty(&self.target)
            && condition_selector_is_empty(&self.product)
            && condition_selector_is_empty(&self.platform)
            && condition_selector_is_empty(&self.host)
            && condition_selector_is_empty(&self.runtime)
    }
}

/// Declared policy action required by one package.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PolicyRequirement {
    /// Policy domain where the action is exercised.
    #[serde(default)]
    pub domain: PolicyDomain,
    /// Action name required by package code.
    pub action: String,
    /// Resource scope touched by the action.
    pub resource: String,
}

/// Policy rule for package, target, or runtime evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PolicyRule {
    /// Policy domain where the action is exercised.
    #[serde(default)]
    pub domain: PolicyDomain,
    /// Subject matched by this rule.
    pub subject: PolicySubject,
    /// Action name matched by this rule.
    pub action: String,
    /// Resource scope matched by this rule.
    pub resource: String,
    /// Access outcome selected by this rule.
    pub access: PolicyAccess,
    /// Route selected when access is allowed.
    pub route: Option<PolicyRoute>,
}

/// Static policy configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    /// Package-declared policy requirements.
    pub requires: Vec<PolicyRequirement>,
    /// Ordered authorization rules.
    pub rules: Vec<PolicyRule>,
}

impl Policy {
    /// Validate resolved policy declarations.
    pub fn validate(&self) -> Result<(), String> {
        // requirements
        for (index, requirement) in self.requires.iter().enumerate() {
            validate_policy_text(&requirement.action, "requires", index, "action")?;
            validate_policy_text(&requirement.resource, "requires", index, "resource")?;
        }

        // rules
        for (index, rule) in self.rules.iter().enumerate() {
            validate_policy_text(&rule.action, "rules", index, "action")?;
            validate_policy_text(&rule.resource, "rules", index, "resource")?;

            // subject selector
            if rule.subject.is_empty() {
                return Err(format!("policy.rules[{index}].subject must not be empty"));
            }

            // deny rules
            if rule.access == PolicyAccess::Deny && rule.route.is_some() {
                return Err(format!(
                    "policy.rules[{index}].route is only valid when access is allow"
                ));
            }
        }

        Ok(())
    }
}

/// Validate that one policy text field is present after trimming.
fn validate_policy_text(
    text: &str,
    collection: &str,
    index: usize,
    field: &str,
) -> Result<(), String> {
    if text.trim().is_empty() {
        Err(format!(
            "policy.{collection}[{index}].{field} must not be empty"
        ))
    } else {
        Ok(())
    }
}

fn package_selector_is_empty(selector: &Option<PackageSelector>) -> bool {
    match selector {
        Some(selector) => selector.is_empty(),
        None => true,
    }
}

fn condition_selector_is_empty(selector: &Option<ConditionSelector>) -> bool {
    match selector {
        Some(selector) => selector.is_empty(),
        None => true,
    }
}
