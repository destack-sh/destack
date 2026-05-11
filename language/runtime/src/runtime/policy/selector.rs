use std::collections::BTreeMap;

use crate::runtime::binding::{
    BindingAffinity, BindingDescriptor, BindingEffect, BindingEngine, BindingProvider,
    current_platform_name,
};
use destack_source::matches as glob_matches;
use destack_workspace::{
    ExecutionMode, RuntimeIdentitySelector, RuntimeLabelOperator, RuntimeLabelRequirement,
    RuntimeLabelSelector,
};

/// Selector clauses for runtime binding policies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct RuntimeSelector {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for full userland function names.
    pub function: Option<String>,
    /// Glob selector for action names.
    pub action: Option<String>,
    /// Glob selector for component names.
    pub component: Option<String>,
    /// Glob selector for module names.
    pub module: Option<String>,
    /// Engine selector.
    pub engine: Option<BindingEngine>,
    /// Execution selector.
    pub execution: Option<Vec<ExecutionMode>>,
    /// Target-platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding provider selector.
    pub provider: Option<BindingProvider>,
    /// Binding affinity selector.
    pub affinity: Option<BindingAffinity>,
    /// Binding effect selector.
    pub effect: Option<BindingEffect>,
    /// Runtime identity selector.
    pub runtime: Option<RuntimeIdentitySelector>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelector>,
}

/// Selector matching subject for one runtime policy check.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeSubject<'a> {
    /// Runtime name for selector matching.
    pub(crate) runtime_name: &'a str,
    /// Runtime labels for selector matching.
    pub(crate) runtime_labels: &'a BTreeMap<String, String>,
    /// Worker name for selector matching.
    pub(crate) worker_name: &'a str,
    /// Worker labels for selector matching.
    pub(crate) worker_labels: &'a BTreeMap<String, String>,
    /// Binding metadata when the event is a binding call.
    pub(crate) binding: Option<BindingDescriptor>,
    /// Execution mode for selector matching.
    pub(crate) mode: ExecutionMode,
    /// Engine for binding-call selector matching.
    pub(crate) engine: Option<BindingEngine>,
}

impl RuntimeSelector {
    /// Create one selector for one binding name glob.
    pub fn binding(pattern: impl Into<String>) -> Self {
        Self {
            binding: Some(pattern.into()),
            ..Self::default()
        }
    }

    /// Create one selector for one function name glob.
    pub fn function(pattern: impl Into<String>) -> Self {
        Self {
            function: Some(pattern.into()),
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

    /// Set the execution-engine selector.
    pub fn engine(mut self, engine: BindingEngine) -> Self {
        self.engine = Some(engine);
        self
    }

    /// Add one execution selector.
    pub fn execution(mut self, mode: ExecutionMode) -> Self {
        let mut modes = self.execution.take().unwrap_or_default();
        modes.push(mode);
        self.execution = Some(modes);
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

    /// Set the binding-effect selector.
    pub fn effect(mut self, effect: BindingEffect) -> Self {
        self.effect = Some(effect);
        self
    }

    /// Set runtime name selector.
    pub fn runtime_name(mut self, name: impl Into<String>) -> Self {
        let mut selector = self.runtime.take().unwrap_or_default();
        selector.name = Some(name.into());
        self.runtime = Some(selector);
        self
    }

    /// Set worker name selector.
    pub fn worker_name(mut self, name: impl Into<String>) -> Self {
        let mut selector = self.worker.take().unwrap_or_default();
        selector.name = Some(name.into());
        self.worker = Some(selector);
        self
    }

    /// Return true when this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.binding.is_none()
            && self.function.is_none()
            && self.action.is_none()
            && self.component.is_none()
            && self.module.is_none()
            && self.engine.is_none()
            && self.execution.is_none()
            && self.platforms.is_none()
            && self.provider.is_none()
            && self.affinity.is_none()
            && self.effect.is_none()
            && self.runtime.is_none()
            && self.worker.is_none()
    }

    /// Return true when this selector requires binding metadata.
    pub fn requires_binding(&self) -> bool {
        self.binding.is_some()
            || self.function.is_some()
            || self.action.is_some()
            || self.component.is_some()
            || self.module.is_some()
            || self.provider.is_some()
            || self.affinity.is_some()
            || self.effect.is_some()
    }

    /// Return true when this selector matches one runtime policy subject.
    pub(crate) fn matches(&self, subject: RuntimeSubject<'_>) -> bool {
        // binding metadata
        if self.requires_binding() && subject.binding.is_none() {
            return false;
        }

        // identity
        if !self.matches_identity(subject) {
            return false;
        }

        // execution
        if !self.matches_execution(subject) {
            return false;
        }

        // target
        if !self.matches_target() {
            return false;
        }

        // binding
        if let Some(binding) = subject.binding {
            return self.matches_binding(binding);
        }

        true
    }

    /// Return true when runtime and worker identity clauses match.
    fn matches_identity(&self, subject: RuntimeSubject<'_>) -> bool {
        if let Some(runtime) = &self.runtime
            && !matches_identity_selector(runtime, subject.runtime_name, subject.runtime_labels)
        {
            return false;
        }

        if let Some(worker) = &self.worker
            && !matches_identity_selector(worker, subject.worker_name, subject.worker_labels)
        {
            return false;
        }

        true
    }

    /// Return true when execution mode and engine clauses match.
    fn matches_execution(&self, subject: RuntimeSubject<'_>) -> bool {
        if let Some(engine) = self.engine
            && subject.engine != Some(engine)
        {
            return false;
        }

        if let Some(modes) = &self.execution
            && !modes.contains(&subject.mode)
        {
            return false;
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

        if self.function.is_some() {
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

        if let Some(effect) = self.effect
            && effect != binding.effect
        {
            return false;
        }

        true
    }
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
