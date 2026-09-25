use std::collections::BTreeMap;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host;
use crate::world::topology::LabelSet;
use crate::world::{EdgeId, EntityId};
use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_repository::{
    ConditionGate, ConditionSelector, ConditionSet, ExecutionMode, PackageSelector,
    RuntimeIdentitySelector, RuntimeLabelOperator, RuntimeLabelRequirement, RuntimeLabelSelector,
};
use tspp_serde::Reflect;
use tspp_source::matches;

/// Selector clauses for policy subjects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct SubjectSelector {
    /// Package selector.
    pub package: Option<PackageSelector>,
    /// Runtime identity selector.
    pub runtime_identity: Option<RuntimeIdentitySelector>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelector>,
    /// Execution-mode selector.
    pub execution: Option<Vec<ExecutionMode>>,
    /// Active source graph mode selector.
    pub mode: Option<ConditionSelector>,
    /// Active source graph role selector.
    pub role: Option<ConditionSelector>,
    /// Active optional feature selector.
    pub feature: Option<ConditionSelector>,
    /// Active source graph tag selector.
    pub tag: Option<ConditionSelector>,
    /// Active build target selector.
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

/// Subject for one policy evaluation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Subject<'a> {
    /// Package name for selector matching.
    pub(crate) package_name: Option<&'a str>,
    /// Runtime name for selector matching.
    pub(crate) runtime_name: &'a str,
    /// Runtime labels for selector matching.
    pub(crate) runtime_labels: &'a LabelSet,
    /// Worker name for selector matching.
    pub(crate) worker_name: &'a str,
    /// Worker labels for selector matching.
    pub(crate) worker_labels: &'a LabelSet,
    /// Execution mode for selector matching.
    pub(crate) mode: ExecutionMode,
    /// Source graph conditions for selector matching.
    pub(crate) conditions: &'a ConditionSet,
}

/// Selector clauses for attempted actions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
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
    pub provider: Option<program::BindingProvider>,
    /// Binding affinity selector.
    pub affinity: Option<program::BindingAffinity>,
    /// Binding effect selector.
    pub effect: Option<program::BindingEffect>,
}

/// Selector for one policy target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
            && self.runtime_identity.is_none()
            && self.worker.is_none()
            && self.execution.is_none()
            && self.mode.as_ref().is_none_or(ConditionSelector::is_empty)
            && self.role.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .feature
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self.tag.as_ref().is_none_or(ConditionSelector::is_empty)
            && self.target.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .product
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self
                .platform
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self.host.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .runtime
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
    }

    /// Return true when this selector matches one policy subject.
    pub(crate) fn matches(&self, subject: Subject<'_>) -> RuntimeResult<bool> {
        // package
        if let Some(package) = &self.package
            && !matches_package_selector(package, subject.package_name)?
        {
            return Ok(false);
        }

        // runtime
        if let Some(runtime) = &self.runtime_identity
            && !matches_identity_selector(runtime, subject.runtime_name, subject.runtime_labels)
        {
            return Ok(false);
        }

        // worker
        if let Some(worker) = &self.worker
            && !matches_identity_selector(worker, subject.worker_name, subject.worker_labels)
        {
            return Ok(false);
        }

        // execution mode
        if let Some(modes) = &self.execution
            && !modes.contains(&subject.mode)
        {
            return Ok(false);
        }

        // source graph conditions
        if !matches_conditions(self, subject.conditions) {
            return Ok(false);
        }

        Ok(true)
    }
}

impl<'a> Subject<'a> {
    /// Create one subject from runtime and worker state.
    pub(crate) fn new(
        runtime_name: &'a str,
        runtime_labels: &'a LabelSet,
        worker_name: &'a str,
        worker_labels: &'a LabelSet,
        mode: ExecutionMode,
        conditions: &'a ConditionSet,
    ) -> Self {
        Self {
            package_name: None,
            runtime_name,
            runtime_labels,
            worker_name,
            worker_labels,
            mode,
            conditions,
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
        self.platforms
            .get_or_insert_with(Vec::new)
            .push(platform.into());

        self
    }

    /// Set the binding-provider selector.
    pub fn provider(mut self, provider: program::BindingProvider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set the binding-affinity selector.
    pub fn affinity(mut self, affinity: program::BindingAffinity) -> Self {
        self.affinity = Some(affinity);
        self
    }

    /// Set the binding effect selector.
    pub fn effect(mut self, effect: program::BindingEffect) -> Self {
        self.effect = Some(effect);
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
            && self.effect.is_none()
    }

    /// Return true when this selector matches one binding call.
    pub(crate) fn matches(
        &self,
        program: &program::Program,
        binding: &program::Binding,
    ) -> RuntimeResult<bool> {
        // target
        if !self.matches_target() {
            return Ok(false);
        }

        // binding
        self.matches_binding(program, binding)
    }

    /// Return true when target-platform clauses match this runtime target.
    fn matches_target(&self) -> bool {
        let Some(platforms) = &self.platforms else {
            return true;
        };

        let platform = host::platform_name();

        platforms
            .iter()
            .any(|pattern| glob_match(pattern, platform))
    }

    /// Return true when binding clauses match one Program declaration.
    fn matches_binding(
        &self,
        program: &program::Program,
        binding: &program::Binding,
    ) -> RuntimeResult<bool> {
        let name = binding_name(program, binding)?;

        if let Some(pattern) = &self.binding
            && !glob_match(pattern, name)
        {
            return Ok(false);
        }

        if let Some(pattern) = &self.module
            && !glob_match(pattern, binding_module_name(name))
        {
            return Ok(false);
        }

        if let Some(pattern) = &self.component
            && !glob_match(pattern, binding_component_name(name))
        {
            return Ok(false);
        }

        if let Some(pattern) = &self.action
            && !matches_strings(program, program.binding_requires(binding), pattern)?
        {
            return Ok(false);
        }

        if let Some(provider) = self.provider
            && binding.provider != provider
        {
            return Ok(false);
        }

        if let Some(affinity) = self.affinity
            && binding.affinity != affinity
        {
            return Ok(false);
        }

        if let Some(effect) = self.effect
            && effect != binding.effect
        {
            return Ok(false);
        }

        Ok(true)
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

    /// Return true when this selector matches one binding attempt target.
    pub(crate) fn matches_binding(&self) -> RuntimeResult<bool> {
        match self {
            Self::Any => Ok(true),
            Self::Entity { .. } | Self::Edge { .. } => Err(RuntimeError::Internal {
                message: "binding policy targets require one resolved topology target".to_string(),
            }
            .boxed()),
        }
    }
}

/// Resolve one binding name from Program metadata.
fn binding_name<'a>(
    program: &'a program::Program,
    binding: &program::Binding,
) -> RuntimeResult<&'a str> {
    program.string(binding.name).ok_or_else(|| {
        RuntimeError::Internal {
            message: format!("binding {:?} references a missing name", binding.id),
        }
        .boxed()
    })
}

/// Return true when one Program string list matches one glob.
fn matches_strings(
    program: &program::Program,
    strings: &[tspp_core::StringId],
    pattern: &str,
) -> RuntimeResult<bool> {
    for id in strings {
        let value = program.string(*id).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("binding metadata references missing string {id:?}"),
            }
            .boxed()
        })?;

        if glob_match(pattern, value) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Return true when one identity selector matches name and labels.
fn matches_identity_selector(
    selector: &RuntimeIdentitySelector,
    name: &str,
    labels: &LabelSet,
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
fn matches_package_selector(
    selector: &PackageSelector,
    package_name: Option<&str>,
) -> RuntimeResult<bool> {
    let Some(package_name) = package_name else {
        return Err(RuntimeError::Internal {
            message: "package policy selectors require Program package identity".to_string(),
        }
        .boxed());
    };

    if selector.patterns.is_empty() {
        return Ok(true);
    }

    let is_match = selector
        .patterns
        .iter()
        .any(|pattern| glob_match(pattern, package_name));

    Ok(is_match)
}

/// Return true when all source graph selectors match.
fn matches_conditions(selector: &SubjectSelector, conditions: &ConditionSet) -> bool {
    let gate = ConditionGate {
        mode: selector.mode.clone(),
        role: selector.role.clone(),
        feature: selector.feature.clone(),
        tag: selector.tag.clone(),
        target: selector.target.clone(),
        product: selector.product.clone(),
        platform: selector.platform.clone(),
        host: selector.host.clone(),
        runtime: selector.runtime.clone(),
        ..ConditionGate::default()
    };

    gate.matches(conditions)
}

/// Return true when one label selector matches labels.
fn matches_label_selector(selector: &RuntimeLabelSelector, labels: &LabelSet) -> bool {
    for (key, expected_value) in &selector.match_labels {
        let Some(actual_value) = labels.get(key) else {
            return false;
        };

        if actual_value != expected_value.as_str() {
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
fn matches_label_requirement(requirement: &RuntimeLabelRequirement, labels: &LabelSet) -> bool {
    match requirement.operator {
        RuntimeLabelOperator::In => labels.get(&requirement.key).is_some_and(|actual_value| {
            requirement.values.iter().any(|value| value == actual_value)
        }),
        RuntimeLabelOperator::NotIn => labels.get(&requirement.key).is_some_and(|actual_value| {
            requirement.values.iter().all(|value| value != actual_value)
        }),
        RuntimeLabelOperator::Exists => labels.contains_key(&requirement.key),
        RuntimeLabelOperator::DoesNotExist => !labels.contains_key(&requirement.key),
    }
}

/// Match one text value against one glob pattern.
fn glob_match(pattern: &str, text: &str) -> bool {
    matches(pattern.as_bytes(), text.as_bytes())
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
