use serde::{Deserialize, Serialize};

use super::runtime::{RuntimeIdentitySelector, RuntimeIdentitySelectorJson};

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

/// Policy backend used for allowed runtime effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum PolicyBackend {
    /// Execute against the live host.
    #[default]
    Host,
    /// Execute against the configured simulation model.
    Simulation,
}

/// Package selector used by static policy rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimePackageSelector {
    /// Package name glob patterns.
    pub patterns: Vec<String>,
}

impl RuntimePackageSelector {
    /// Return whether this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

/// Policy subject matched by one rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PolicySubject {
    /// Package selector.
    pub package: Option<RuntimePackageSelector>,
    /// Runtime identity selector.
    pub runtime: Option<RuntimeIdentitySelector>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelector>,
}

impl PolicySubject {
    /// Return whether this selector matches no subject dimension.
    pub fn is_empty(&self) -> bool {
        package_selector_is_empty(&self.package) && self.runtime.is_none() && self.worker.is_none()
    }
}

/// Declared policy action required by one package.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyRequirement {
    /// Policy domain where the action is exercised.
    pub domain: PolicyDomain,
    /// Action name required by package code.
    pub action: String,
    /// Resource scope touched by the action.
    pub resource: String,
}

/// Policy rule for package, target, or runtime evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Policy domain where the action is exercised.
    pub domain: PolicyDomain,
    /// Subject matched by this rule.
    pub subject: PolicySubject,
    /// Action name matched by this rule.
    pub action: String,
    /// Resource scope matched by this rule.
    pub resource: String,
    /// Access outcome selected by this rule.
    pub access: PolicyAccess,
    /// Backend selected when access is allowed.
    pub backend: Option<PolicyBackend>,
}

/// Static policy configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PolicyOptions {
    /// Package-declared policy requirements.
    pub requires: Vec<PolicyRequirement>,
    /// Ordered policy rules.
    pub rules: Vec<PolicyRule>,
}

impl PolicyOptions {
    /// Append child policy declarations to one parent policy.
    pub fn from_json_with_parent(json: Option<&PolicyOptionsJson>, parent: &Self) -> Self {
        let mut options = parent.clone();

        if let Some(json) = json {
            json.apply_to(&mut options);
        }

        options
    }

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
            if rule.access == PolicyAccess::Deny && rule.backend.is_some() {
                return Err(format!(
                    "policy.rules[{index}].backend is only valid when access is allow"
                ));
            }
        }

        Ok(())
    }
}

/// Package selector accepted by static policy JSON.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuntimePackageSelectorJson {
    /// Package name shorthand.
    Pattern(String),
    /// Package name shorthand list.
    Patterns(Vec<String>),
    /// Structured package selector.
    Selector(RuntimePackageSelectorJsonObject),
}

/// Structured package selector accepted by static policy JSON.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimePackageSelectorJsonObject {
    /// Package name glob patterns.
    pub patterns: Vec<String>,
}

impl From<&RuntimePackageSelectorJson> for RuntimePackageSelector {
    fn from(value: &RuntimePackageSelectorJson) -> Self {
        match value {
            RuntimePackageSelectorJson::Pattern(pattern) => Self {
                patterns: vec![pattern.clone()],
            },
            RuntimePackageSelectorJson::Patterns(patterns) => Self {
                patterns: patterns.clone(),
            },
            RuntimePackageSelectorJson::Selector(selector) => Self {
                patterns: selector.patterns.clone(),
            },
        }
    }
}

/// Policy subject JSON.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PolicySubjectJson {
    /// Package selector.
    pub package: Option<RuntimePackageSelectorJson>,
    /// Runtime identity selector.
    pub runtime: Option<RuntimeIdentitySelectorJson>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelectorJson>,
}

impl PolicySubjectJson {
    /// Return whether this selector matches no subject dimension.
    pub fn is_empty(&self) -> bool {
        PolicySubject::from(self).is_empty()
    }
}

impl From<&PolicySubjectJson> for PolicySubject {
    fn from(value: &PolicySubjectJson) -> Self {
        Self {
            package: value.package.as_ref().map(RuntimePackageSelector::from),
            runtime: value.runtime.as_ref().map(RuntimeIdentitySelector::from),
            worker: value.worker.as_ref().map(RuntimeIdentitySelector::from),
        }
    }
}

/// Policy requirement JSON.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PolicyRequirementJson {
    /// Policy domain where the action is exercised.
    #[serde(default)]
    pub domain: PolicyDomain,
    /// Action name required by package code.
    pub action: String,
    /// Resource scope touched by the action.
    pub resource: String,
}

impl From<&PolicyRequirementJson> for PolicyRequirement {
    fn from(value: &PolicyRequirementJson) -> Self {
        Self {
            domain: value.domain,
            action: value.action.clone(),
            resource: value.resource.clone(),
        }
    }
}

/// Policy rule JSON.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PolicyRuleJson {
    /// Policy domain where the action is exercised.
    #[serde(default)]
    pub domain: PolicyDomain,
    /// Subject matched by this rule.
    pub subject: PolicySubjectJson,
    /// Action name matched by this rule.
    pub action: String,
    /// Resource scope matched by this rule.
    pub resource: String,
    /// Access outcome selected by this rule.
    pub access: PolicyAccess,
    /// Backend selected when access is allowed.
    pub backend: Option<PolicyBackend>,
}

impl From<&PolicyRuleJson> for PolicyRule {
    fn from(value: &PolicyRuleJson) -> Self {
        let backend = match value.access {
            PolicyAccess::Allow => Some(value.backend.unwrap_or_default()),
            PolicyAccess::Deny => None,
        };

        Self {
            domain: value.domain,
            subject: PolicySubject::from(&value.subject),
            action: value.action.clone(),
            resource: value.resource.clone(),
            access: value.access,
            backend,
        }
    }
}

/// Static policy configuration JSON.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PolicyOptionsJson {
    /// Package-declared policy requirements.
    pub requires: Option<Vec<PolicyRequirementJson>>,
    /// Ordered policy rules.
    pub rules: Option<Vec<PolicyRuleJson>>,
}

impl PolicyOptionsJson {
    /// Validate explicit policy declarations before they are normalized.
    pub fn validate(&self) -> Result<(), String> {
        // requirements
        if let Some(requires) = &self.requires {
            for (index, requirement) in requires.iter().enumerate() {
                validate_policy_text(&requirement.action, "requires", index, "action")?;
                validate_policy_text(&requirement.resource, "requires", index, "resource")?;
            }
        }

        // rules
        if let Some(rules) = &self.rules {
            for (index, rule) in rules.iter().enumerate() {
                validate_policy_text(&rule.action, "rules", index, "action")?;
                validate_policy_text(&rule.resource, "rules", index, "resource")?;

                // subject selector
                if rule.subject.is_empty() {
                    return Err(format!("policy.rules[{index}].subject must not be empty"));
                }

                // deny rules
                if rule.access == PolicyAccess::Deny && rule.backend.is_some() {
                    return Err(format!(
                        "policy.rules[{index}].backend is only valid when access is allow"
                    ));
                }
            }
        }

        Ok(())
    }

    /// Append explicit policy declarations to one base set of options.
    pub fn apply_to(&self, options: &mut PolicyOptions) {
        if let Some(requires) = &self.requires {
            options
                .requires
                .extend(requires.iter().map(PolicyRequirement::from));
        }

        if let Some(rules) = &self.rules {
            options.rules.extend(rules.iter().map(PolicyRule::from));
        }
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

fn package_selector_is_empty(selector: &Option<RuntimePackageSelector>) -> bool {
    match selector {
        Some(selector) => selector.is_empty(),
        None => true,
    }
}
