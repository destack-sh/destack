use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::stacks::merge_metadata;

/// Dynamic runtime feature options.
#[derive(Debug, Clone, Default)]
pub struct FeatureOptions {
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Default feature value.
    pub default: Option<Value>,
    /// Named feature variants.
    pub variants: IndexMap<String, Value>,
    /// Ordered targeting rules.
    pub rules: Vec<FeatureRuleOptions>,
    /// Optional external feature provider key.
    pub provider: Option<String>,
    /// Extra feature arguments.
    pub with: Option<Value>,
}

impl FeatureOptions {
    /// Inherit unset feature settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);

        if self.default.is_none() {
            self.default = parent.default.clone();
        }

        for (name, value) in &parent.variants {
            self.variants
                .entry(name.clone())
                .or_insert_with(|| value.clone());
        }

        if self.rules.is_empty() {
            self.rules = parent.rules.clone();
        }

        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&FeatureJson> for FeatureOptions {
    fn from(json: &FeatureJson) -> Self {
        Self {
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            default: json.default.clone(),
            variants: json.variants.clone().unwrap_or_default(),
            rules: json
                .rules
                .as_ref()
                .map(|rules| rules.iter().map(FeatureRuleOptions::from).collect())
                .unwrap_or_default(),
            provider: json.provider.clone(),
            with: json.with.clone(),
        }
    }
}

/// Feature targeting rule options.
#[derive(Debug, Clone, Default)]
pub struct FeatureRuleOptions {
    /// Environment names targeted by this rule.
    pub environments: Vec<String>,
    /// Target names targeted by this rule.
    pub targets: Vec<String>,
    /// Required label matches.
    pub labels: IndexMap<String, String>,
    /// Required runtime context matches.
    pub context: IndexMap<String, Value>,
    /// Optional rollout percentage.
    pub percentage: Option<u8>,
    /// Named variant selected by this rule.
    pub variant: Option<String>,
    /// Direct value selected by this rule.
    pub value: Option<Value>,
    /// Extra rule arguments.
    pub with: Option<Value>,
}

impl FeatureRuleOptions {
    /// Inherit unset rule settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.environments.is_empty() {
            self.environments = parent.environments.clone();
        }

        if self.targets.is_empty() {
            self.targets = parent.targets.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        for (name, value) in &parent.context {
            self.context
                .entry(name.clone())
                .or_insert_with(|| value.clone());
        }

        if self.percentage.is_none() {
            self.percentage = parent.percentage;
        }

        if self.variant.is_none() {
            self.variant = parent.variant.clone();
        }

        if self.value.is_none() {
            self.value = parent.value.clone();
        }

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&FeatureRuleJson> for FeatureRuleOptions {
    fn from(json: &FeatureRuleJson) -> Self {
        Self {
            environments: json.environments.clone().unwrap_or_default(),
            targets: json.targets.clone().unwrap_or_default(),
            labels: json.labels.clone().unwrap_or_default(),
            context: json.context.clone().unwrap_or_default(),
            percentage: json.percentage,
            variant: json.variant.clone(),
            value: json.value.clone(),
            with: json.with.clone(),
        }
    }
}

/// Feature reference JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum FeatureRefsJson {
    /// One named feature definition reference.
    One(String),
    /// Many named feature definition references.
    Many(Vec<String>),
}

impl FeatureRefsJson {
    /// Return the referenced feature names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(name) => vec![name.clone()],
            Self::Many(names) => names.clone(),
        }
    }
}

/// Convert feature declarations into normalized options.
pub fn feature_options_from_json(
    json: &Option<IndexMap<String, FeatureJson>>,
) -> IndexMap<String, FeatureOptions> {
    json.as_ref()
        .map(|features| {
            features
                .iter()
                .map(|(name, feature)| (name.clone(), FeatureOptions::from(feature)))
                .collect()
        })
        .unwrap_or_default()
}

/// Inherit one feature map from a parent config.
pub fn extend_feature_options(
    current: &mut IndexMap<String, FeatureOptions>,
    parent: &IndexMap<String, FeatureOptions>,
) {
    for (name, feature) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(feature);
        } else {
            current.insert(name.clone(), feature.clone());
        }
    }
}

/// Dynamic runtime feature JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FeatureJson {
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Default feature value.
    pub default: Option<Value>,
    /// Named feature variants.
    pub variants: Option<IndexMap<String, Value>>,
    /// Ordered targeting rules.
    pub rules: Option<Vec<FeatureRuleJson>>,
    /// Optional external feature provider key.
    pub provider: Option<String>,
    /// Extra feature arguments.
    pub with: Option<Value>,
}

/// Feature targeting rule JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FeatureRuleJson {
    /// Environment names targeted by this rule.
    pub environments: Option<Vec<String>>,
    /// Target names targeted by this rule.
    pub targets: Option<Vec<String>>,
    /// Required label matches.
    pub labels: Option<IndexMap<String, String>>,
    /// Required runtime context matches.
    pub context: Option<IndexMap<String, Value>>,
    /// Optional rollout percentage.
    #[cfg_attr(feature = "schema", schemars(range(min = 0, max = 100)))]
    pub percentage: Option<u8>,
    /// Named variant selected by this rule.
    pub variant: Option<String>,
    /// Direct value selected by this rule.
    pub value: Option<Value>,
    /// Extra rule arguments.
    pub with: Option<Value>,
}
