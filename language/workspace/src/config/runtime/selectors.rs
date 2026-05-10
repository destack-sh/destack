use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Label selection operator for runtime identity selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeLabelOperator {
    /// Match labels whose value is in the supplied value set.
    In,
    /// Match labels whose value is not in the supplied value set.
    NotIn,
    /// Match labels where the key exists regardless of value.
    Exists,
    /// Match labels where the key does not exist.
    DoesNotExist,
}

/// One label requirement clause for runtime identity selectors.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeLabelRequirement {
    /// Label key to evaluate.
    pub key: String,
    /// Label requirement operator.
    pub operator: RuntimeLabelOperator,
    /// Label values for set-based operators.
    pub values: Vec<String>,
}

impl RuntimeLabelRequirement {
    /// Create one in-operator requirement.
    pub fn in_values(key: impl Into<String>, values: Vec<String>) -> Self {
        Self {
            key: key.into(),
            operator: RuntimeLabelOperator::In,
            values,
        }
    }

    /// Create one not-in-operator requirement.
    pub fn not_in_values(key: impl Into<String>, values: Vec<String>) -> Self {
        Self {
            key: key.into(),
            operator: RuntimeLabelOperator::NotIn,
            values,
        }
    }

    /// Create one exists-operator requirement.
    pub fn exists(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            operator: RuntimeLabelOperator::Exists,
            values: Vec::new(),
        }
    }

    /// Create one does-not-exist-operator requirement.
    pub fn does_not_exist(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            operator: RuntimeLabelOperator::DoesNotExist,
            values: Vec::new(),
        }
    }
}

/// Kubernetes-style label selector for runtime identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeLabelSelector {
    /// Exact-match labels that must all be present.
    pub match_labels: BTreeMap<String, String>,
    /// Additional set-based label requirements.
    pub match_expressions: Vec<RuntimeLabelRequirement>,
}

impl RuntimeLabelSelector {
    /// Create one empty label selector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one exact-match label requirement.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.match_labels.insert(key.into(), value.into());
        self
    }

    /// Add one expression requirement.
    pub fn expression(mut self, requirement: RuntimeLabelRequirement) -> Self {
        self.match_expressions.push(requirement);
        self
    }

    /// Add one in-operator expression requirement.
    pub fn in_values(mut self, key: impl Into<String>, values: Vec<String>) -> Self {
        self.match_expressions
            .push(RuntimeLabelRequirement::in_values(key, values));
        self
    }

    /// Add one not-in-operator expression requirement.
    pub fn not_in_values(mut self, key: impl Into<String>, values: Vec<String>) -> Self {
        self.match_expressions
            .push(RuntimeLabelRequirement::not_in_values(key, values));
        self
    }

    /// Add one exists-operator expression requirement.
    pub fn exists(mut self, key: impl Into<String>) -> Self {
        self.match_expressions
            .push(RuntimeLabelRequirement::exists(key));
        self
    }

    /// Add one does-not-exist-operator expression requirement.
    pub fn does_not_exist(mut self, key: impl Into<String>) -> Self {
        self.match_expressions
            .push(RuntimeLabelRequirement::does_not_exist(key));
        self
    }
}

/// Runtime identity selector for worker and runtime scopes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeIdentitySelector {
    /// Name selector for one runtime or one worker.
    pub name: Option<String>,
    /// Label selector for one runtime or one worker.
    pub labels: Option<RuntimeLabelSelector>,
}

impl RuntimeIdentitySelector {
    /// Create one identity selector that matches one name glob.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            labels: None,
        }
    }

    /// Attach one label selector.
    pub fn labels(mut self, labels: RuntimeLabelSelector) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Attach one exact-match label requirement.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let labels = self.labels.take().unwrap_or_default().label(key, value);
        self.labels = Some(labels);
        self
    }
}

/// Runtime label operator for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum RuntimeLabelOperatorJson {
    /// Match labels whose value is in the supplied value set.
    In,
    /// Match labels whose value is not in the supplied value set.
    NotIn,
    /// Match labels where the key exists regardless of value.
    Exists,
    /// Match labels where the key does not exist.
    DoesNotExist,
}

impl From<RuntimeLabelOperatorJson> for RuntimeLabelOperator {
    fn from(value: RuntimeLabelOperatorJson) -> Self {
        match value {
            RuntimeLabelOperatorJson::In => RuntimeLabelOperator::In,
            RuntimeLabelOperatorJson::NotIn => RuntimeLabelOperator::NotIn,
            RuntimeLabelOperatorJson::Exists => RuntimeLabelOperator::Exists,
            RuntimeLabelOperatorJson::DoesNotExist => RuntimeLabelOperator::DoesNotExist,
        }
    }
}

/// Runtime label requirement for JSON deserialization.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLabelRequirementJson {
    /// Label key to evaluate.
    pub key: String,
    /// Label requirement operator.
    pub operator: RuntimeLabelOperatorJson,
    /// Label values for set-based operators.
    pub values: Vec<String>,
}

impl From<&RuntimeLabelRequirementJson> for RuntimeLabelRequirement {
    fn from(value: &RuntimeLabelRequirementJson) -> Self {
        Self {
            key: value.key.clone(),
            operator: RuntimeLabelOperator::from(value.operator),
            values: value.values.clone(),
        }
    }
}

/// Kubernetes-style runtime label selector for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLabelSelectorJson {
    /// Exact-match labels that must all be present.
    pub match_labels: Option<BTreeMap<String, String>>,
    /// Additional set-based label requirements.
    pub match_expressions: Option<Vec<RuntimeLabelRequirementJson>>,
}

impl From<&RuntimeLabelSelectorJson> for RuntimeLabelSelector {
    fn from(value: &RuntimeLabelSelectorJson) -> Self {
        let match_labels = value.match_labels.clone().unwrap_or_default();
        let match_expressions = value
            .match_expressions
            .as_ref()
            .map(|expressions| {
                expressions
                    .iter()
                    .map(RuntimeLabelRequirement::from)
                    .collect()
            })
            .unwrap_or_default();

        Self {
            match_labels,
            match_expressions,
        }
    }
}

/// Runtime identity selector for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIdentitySelectorJson {
    /// Name selector for one runtime or one worker.
    pub name: Option<String>,
    /// Label selector for one runtime or one worker.
    pub labels: Option<RuntimeLabelSelectorJson>,
}

impl From<&RuntimeIdentitySelectorJson> for RuntimeIdentitySelector {
    fn from(value: &RuntimeIdentitySelectorJson) -> Self {
        Self {
            name: value.name.clone(),
            labels: value.labels.as_ref().map(RuntimeLabelSelector::from),
        }
    }
}
