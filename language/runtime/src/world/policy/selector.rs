use std::collections::BTreeMap;

use crate::host::binding::{
    BindingAffinity, BindingDescriptor, BindingDeterminism, BindingProvider, current_platform_name,
};
use crate::world::{EdgeId, EntityId};
use destack_source::matches as glob_matches;
use destack_workspace::{
    ExecutionMode, PackageSelector, RuntimeIdentitySelector, RuntimeLabelOperator,
    RuntimeLabelRequirement, RuntimeLabelSelector,
};
use serde::{Deserialize, Serialize};

/// Selector clauses for policy subjects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SubjectSelector {
    /// Package selector.
    pub package: Option<PackageSelector>,
    /// Runtime identity selector.
    pub runtime: Option<RuntimeIdentitySelector>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelector>,
    /// Execution-mode selector.
    pub execution: Option<Vec<ExecutionMode>>,
}

/// Subject facts for one policy evaluation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Subject<'a> {
    /// Package name for selector matching.
    pub(crate) package_name: Option<&'a str>,
    /// Runtime name for selector matching.
    pub(crate) runtime_name: &'a str,
    /// Runtime labels for selector matching.
    pub(crate) runtime_labels: &'a BTreeMap<String, String>,
    /// Worker name for selector matching.
    pub(crate) worker_name: &'a str,
    /// Worker labels for selector matching.
    pub(crate) worker_labels: &'a BTreeMap<String, String>,
    /// Execution mode for selector matching.
    pub(crate) mode: ExecutionMode,
}

/// Selector clauses for attempted actions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ActionSelector {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for required action names.
    pub action: Option<String>,
    /// Glob selector for binding component names.
    pub component: Option<String>,
    /// Glob selector for binding module names.
    pub module: Option<String>,
    /// Target-platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding provider selector.
    pub provider: Option<BindingProvider>,
    /// Binding affinity selector.
    pub affinity: Option<BindingAffinity>,
    /// Binding determinism selector.
    pub determinism: Option<BindingDeterminism>,
}

/// Attempt facts for one policy action.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Attempt {
    /// Binding metadata when the event is a binding call.
    pub(crate) binding: Option<BindingDescriptor>,
}

/// Selector for one policy target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetSelector {
    /// Match any target.
    #[default]
    Any,
    /// Match topology entities.
    Entity {
        /// Entity selector expression.
        selector: EntitySelector,
    },
    /// Match topology edges.
    Edge {
        /// Edge selector expression.
        selector: EdgeSelector,
    },
}

/// Selector for one topology entity target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntitySelector {
    /// Match all entities of this class.
    Any,
    /// Match one entity by stable id.
    Id {
        /// Stable topology entity identifier.
        entity_id: EntityId,
    },
    /// Match a fixed set of entities by stable id.
    Ids {
        /// Stable topology entity identifiers.
        entity_ids: Vec<EntityId>,
    },
    /// Match entities by label constraints.
    Labels {
        /// Label selector expression.
        labels: RuntimeLabelSelector,
    },
    /// Match one deterministically chosen entity from one selector result set.
    ChooseOne {
        /// Source selector for deterministic sampling.
        selector: Box<EntitySelector>,
    },
}

impl EntitySelector {
    /// Match all entities.
    pub fn any() -> Self {
        Self::Any
    }

    /// Match one entity id.
    pub fn id(entity_id: impl Into<EntityId>) -> Self {
        Self::Id {
            entity_id: entity_id.into(),
        }
    }

    /// Match many entity ids.
    pub fn ids(entity_ids: Vec<EntityId>) -> Self {
        Self::Ids { entity_ids }
    }

    /// Match entities by one explicit label selector.
    pub fn labels(labels: RuntimeLabelSelector) -> Self {
        Self::Labels { labels }
    }

    /// Match entities by exact label key-value pairs.
    pub fn labels_exact(match_labels: BTreeMap<String, String>) -> Self {
        Self::Labels {
            labels: RuntimeLabelSelector {
                match_labels,
                match_expressions: Vec::new(),
            },
        }
    }

    /// Match entities by one exact label key-value pair.
    pub fn label(key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut match_labels = BTreeMap::new();
        match_labels.insert(key.into(), value.into());

        Self::labels_exact(match_labels)
    }

    /// Select one deterministic entity from one selector result set.
    pub fn choose_one(selector: EntitySelector) -> Self {
        Self::ChooseOne {
            selector: Box::new(selector),
        }
    }
}

/// Selector for one topology edge target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeSelector {
    /// Match all edges of this class.
    Any,
    /// Match one edge by stable id.
    Id {
        /// Stable topology edge identifier.
        edge_id: EdgeId,
    },
    /// Match a fixed set of edges by stable id.
    Ids {
        /// Stable topology edge identifiers.
        edge_ids: Vec<EdgeId>,
    },
    /// Match edges incident to one entity selector.
    Incident {
        /// Incident entity selector expression.
        entity: EntitySelector,
    },
    /// Match edges by source and destination entity selectors.
    Between {
        /// Source entity selector expression.
        from: EntitySelector,
        /// Destination entity selector expression.
        to: EntitySelector,
    },
    /// Match edges by label constraints.
    Labels {
        /// Label selector expression.
        labels: RuntimeLabelSelector,
    },
}

impl EdgeSelector {
    /// Match all edges.
    pub fn any() -> Self {
        Self::Any
    }

    /// Match one edge id.
    pub fn id(edge_id: impl Into<EdgeId>) -> Self {
        Self::Id {
            edge_id: edge_id.into(),
        }
    }

    /// Match many edge ids.
    pub fn ids(edge_ids: Vec<EdgeId>) -> Self {
        Self::Ids { edge_ids }
    }

    /// Match edges incident to one entity selector.
    pub fn incident(entity: EntitySelector) -> Self {
        Self::Incident { entity }
    }

    /// Match edges between one source and destination entity selector.
    pub fn between(from: EntitySelector, to: EntitySelector) -> Self {
        Self::Between { from, to }
    }

    /// Match edges by one explicit label selector.
    pub fn labels(labels: RuntimeLabelSelector) -> Self {
        Self::Labels { labels }
    }

    /// Match edges by exact label key-value pairs.
    pub fn labels_exact(match_labels: BTreeMap<String, String>) -> Self {
        Self::Labels {
            labels: RuntimeLabelSelector {
                match_labels,
                match_expressions: Vec::new(),
            },
        }
    }
}

impl SubjectSelector {
    /// Return true when this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.package.as_ref().is_none_or(PackageSelector::is_empty)
            && self.runtime.is_none()
            && self.worker.is_none()
            && self.execution.is_none()
    }

    /// Return true when this selector matches one policy subject.
    pub(crate) fn matches(&self, subject: Subject<'_>) -> bool {
        // package
        if let Some(package) = &self.package
            && !matches_package_selector(package, subject.package_name)
        {
            return false;
        }

        // runtime
        if let Some(runtime) = &self.runtime
            && !matches_identity_selector(runtime, subject.runtime_name, subject.runtime_labels)
        {
            return false;
        }

        // worker
        if let Some(worker) = &self.worker
            && !matches_identity_selector(worker, subject.worker_name, subject.worker_labels)
        {
            return false;
        }

        // execution mode
        if let Some(modes) = &self.execution
            && !modes.contains(&subject.mode)
        {
            return false;
        }

        true
    }
}

impl<'a> Subject<'a> {
    /// Create one subject from runtime and worker facts.
    pub(crate) fn new(
        runtime_name: &'a str,
        runtime_labels: &'a BTreeMap<String, String>,
        worker_name: &'a str,
        worker_labels: &'a BTreeMap<String, String>,
        mode: ExecutionMode,
    ) -> Self {
        Self {
            package_name: None,
            runtime_name,
            runtime_labels,
            worker_name,
            worker_labels,
            mode,
        }
    }
}

impl ActionSelector {
    /// Create one selector for one binding name glob.
    pub fn binding(pattern: impl Into<String>) -> Self {
        Self {
            binding: Some(pattern.into()),
            ..Self::default()
        }
    }

    /// Set the action glob selector.
    pub fn action(mut self, pattern: impl Into<String>) -> Self {
        self.action = Some(pattern.into());
        self
    }

    /// Set the component glob selector.
    pub fn component(mut self, pattern: impl Into<String>) -> Self {
        self.component = Some(pattern.into());
        self
    }

    /// Set the module glob selector.
    pub fn module(mut self, pattern: impl Into<String>) -> Self {
        self.module = Some(pattern.into());
        self
    }

    /// Add one target-platform selector.
    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        let mut platforms = self.platforms.take().unwrap_or_default();
        platforms.push(platform.into());
        self.platforms = Some(platforms);
        self
    }

    /// Set the binding-provider selector.
    pub fn provider(mut self, provider: BindingProvider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set the binding-affinity selector.
    pub fn affinity(mut self, affinity: BindingAffinity) -> Self {
        self.affinity = Some(affinity);
        self
    }

    /// Set the binding-determinism selector.
    pub fn determinism(mut self, determinism: BindingDeterminism) -> Self {
        self.determinism = Some(determinism);
        self
    }

    /// Return true when this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.binding.is_none()
            && self.action.is_none()
            && self.component.is_none()
            && self.module.is_none()
            && self.platforms.is_none()
            && self.provider.is_none()
            && self.affinity.is_none()
            && self.determinism.is_none()
    }

    /// Return true when this selector requires binding metadata.
    pub fn requires_binding(&self) -> bool {
        self.binding.is_some()
            || self.action.is_some()
            || self.component.is_some()
            || self.module.is_some()
            || self.provider.is_some()
            || self.affinity.is_some()
            || self.determinism.is_some()
    }

    /// Return true when this selector matches one binding call.
    pub(crate) fn matches(&self, attempt: Attempt) -> bool {
        // binding metadata
        if self.requires_binding() && attempt.binding.is_none() {
            return false;
        }

        // target
        if !self.matches_target() {
            return false;
        }

        // binding
        if let Some(binding) = attempt.binding {
            return self.matches_binding(binding);
        }

        true
    }

    /// Return true when target-platform clauses match this runtime target.
    fn matches_target(&self) -> bool {
        let Some(platforms) = &self.platforms else {
            return true;
        };

        let platform = current_platform_name();

        platforms
            .iter()
            .any(|pattern| glob_match(pattern, platform))
    }

    /// Return true when binding clauses match one binding descriptor.
    fn matches_binding(&self, binding: BindingDescriptor) -> bool {
        if let Some(pattern) = &self.binding
            && !glob_match(pattern, binding.name)
        {
            return false;
        }

        if let Some(pattern) = &self.module
            && !glob_match(pattern, binding_module_name(binding.name))
        {
            return false;
        }

        if let Some(pattern) = &self.component
            && !glob_match(pattern, binding_component_name(binding.name))
        {
            return false;
        }

        if let Some(pattern) = &self.action
            && !binding
                .requires()
                .iter()
                .any(|action| glob_match(pattern, action))
        {
            return false;
        }

        if let Some(provider) = self.provider
            && binding.provider() != provider
        {
            return false;
        }

        if let Some(affinity) = self.affinity
            && binding.affinity() != affinity
        {
            return false;
        }

        if let Some(determinism) = self.determinism
            && determinism != binding.determinism
        {
            return false;
        }

        true
    }
}

impl TargetSelector {
    /// Match all targets.
    pub fn any() -> Self {
        Self::Any
    }

    /// Match one entity selector.
    pub fn entity(selector: EntitySelector) -> Self {
        Self::Entity { selector }
    }

    /// Match one edge selector.
    pub fn edge(selector: EdgeSelector) -> Self {
        Self::Edge { selector }
    }
}

/// Return true when one rule matches one subject and attempt.
pub(crate) fn matches_rule_selectors(
    rule: &super::Rule,
    subject: Subject<'_>,
    attempt: Attempt,
) -> bool {
    if !rule.subject.matches(subject) {
        return false;
    }

    rule.action.matches(attempt)
}

/// Return true when one identity selector matches name and labels.
fn matches_identity_selector(
    selector: &RuntimeIdentitySelector,
    name: &str,
    labels: &BTreeMap<String, String>,
) -> bool {
    if let Some(pattern) = &selector.name
        && !glob_match(pattern, name)
    {
        return false;
    }

    if let Some(label_selector) = &selector.labels
        && !matches_label_selector(label_selector, labels)
    {
        return false;
    }

    true
}

/// Return true when one package selector matches one package name.
fn matches_package_selector(selector: &PackageSelector, package_name: Option<&str>) -> bool {
    let Some(package_name) = package_name else {
        return false;
    };

    if selector.patterns.is_empty() {
        return true;
    }

    selector
        .patterns
        .iter()
        .any(|pattern| glob_match(pattern, package_name))
}

/// Return true when one label selector matches labels.
fn matches_label_selector(
    selector: &RuntimeLabelSelector,
    labels: &BTreeMap<String, String>,
) -> bool {
    for (key, expected_value) in &selector.match_labels {
        let Some(actual_value) = labels.get(key) else {
            return false;
        };

        if actual_value != expected_value {
            return false;
        }
    }

    for requirement in &selector.match_expressions {
        if !matches_label_requirement(requirement, labels) {
            return false;
        }
    }

    true
}

/// Return true when one label requirement matches labels.
fn matches_label_requirement(
    requirement: &RuntimeLabelRequirement,
    labels: &BTreeMap<String, String>,
) -> bool {
    match requirement.operator {
        RuntimeLabelOperator::In => labels
            .get(&requirement.key)
            .is_some_and(|value| requirement.values.contains(value)),
        RuntimeLabelOperator::NotIn => labels
            .get(&requirement.key)
            .is_some_and(|value| !requirement.values.contains(value)),
        RuntimeLabelOperator::Exists => labels.contains_key(&requirement.key),
        RuntimeLabelOperator::DoesNotExist => !labels.contains_key(&requirement.key),
    }
}

/// Match one text value against one glob pattern.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}

/// Return the module segment for one binding id.
fn binding_module_name(binding_name: &str) -> &str {
    binding_name
        .strip_prefix("destack.")
        .unwrap_or(binding_name)
}

/// Return the component segment for one binding id.
fn binding_component_name(binding_name: &str) -> &str {
    let module_name = binding_module_name(binding_name);

    module_name
        .split_once('.')
        .map(|(component, _)| component)
        .unwrap_or(module_name)
}
