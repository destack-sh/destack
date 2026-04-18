use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::super::policy::{ExecutionMode, ExecutionModeJson};

/// Engine selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingEngine {
    /// Match VM engine execution.
    Vm,
    /// Match native engine execution.
    Native,
}

/// Binding scope selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingScope {
    /// Match host scope bindings.
    Host,
    /// Match runtime scope bindings.
    Runtime,
}

/// Blocking behavior selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingBlocking {
    /// Match always-blocking bindings.
    Always,
    /// Match never-blocking bindings.
    Never,
    /// Match conditionally blocking bindings.
    Sometimes,
}

/// Affinity selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingAffinity {
    /// Match bindings callable from any execution context.
    Any,
    /// Match bindings that require the worker event-loop context.
    EventLoop,
    /// Match bindings that require the creating execution context.
    Owner,
    /// Match bindings that require the process main context.
    ProcessMain,
}

/// Binding effect class selector for runtime rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BindingEffect {
    /// Match pure bindings.
    Pure,
    /// Match deterministic bindings.
    Deterministic,
    /// Match recordable external bindings.
    ExternalRecordable,
    /// Match non-recordable external bindings.
    ExternalNonRecordable,
}

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

/// Selector clauses for runtime binding policies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RuntimeSelector {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for full userland function names.
    pub function: Option<String>,
    /// Glob selector for capability names.
    pub capability: Option<String>,
    /// Glob selector for component names.
    pub component: Option<String>,
    /// Glob selector for module names.
    pub module: Option<String>,
    /// Engine selector.
    pub engine: Option<BindingEngine>,
    /// Execution modes selector.
    pub execution_modes: Option<Vec<ExecutionMode>>,
    /// Platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding scope selector.
    pub scope: Option<BindingScope>,
    /// Binding blocking selector.
    pub blocking: Option<BindingBlocking>,
    /// Binding affinity selector.
    pub affinity: Option<BindingAffinity>,
    /// Binding effect selector.
    pub effect: Option<BindingEffect>,
    /// Runtime identity selector.
    pub runtime: Option<RuntimeIdentitySelector>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelector>,
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

    /// Set the capability glob selector.
    pub fn capability(mut self, pattern: impl Into<String>) -> Self {
        self.capability = Some(pattern.into());
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

    /// Set execution-mode selectors.
    pub fn execution_modes(mut self, modes: Vec<ExecutionMode>) -> Self {
        self.execution_modes = Some(modes);
        self
    }

    /// Add one execution-mode selector.
    pub fn execution_mode(mut self, mode: ExecutionMode) -> Self {
        let mut modes = self.execution_modes.take().unwrap_or_default();
        modes.push(mode);
        self.execution_modes = Some(modes);
        self
    }

    /// Set the platform selectors.
    pub fn platforms(mut self, platforms: Vec<String>) -> Self {
        self.platforms = Some(platforms);
        self
    }

    /// Add one platform selector.
    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        let mut platforms = self.platforms.take().unwrap_or_default();
        platforms.push(platform.into());
        self.platforms = Some(platforms);
        self
    }

    /// Set the binding-scope selector.
    pub fn scope(mut self, scope: BindingScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Set the binding-blocking selector.
    pub fn blocking(mut self, blocking: BindingBlocking) -> Self {
        self.blocking = Some(blocking);
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

    /// Set the runtime identity selector.
    pub fn runtime(mut self, runtime: RuntimeIdentitySelector) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Set the worker identity selector.
    pub fn worker(mut self, worker: RuntimeIdentitySelector) -> Self {
        self.worker = Some(worker);
        self
    }

    /// Set runtime name selector.
    pub fn runtime_name(mut self, name: impl Into<String>) -> Self {
        let mut selector = self.runtime.take().unwrap_or_default();
        selector.name = Some(name.into());
        self.runtime = Some(selector);
        self
    }

    /// Set runtime label selector.
    pub fn runtime_labels(mut self, labels: RuntimeLabelSelector) -> Self {
        let selector = self.runtime.take().unwrap_or_default().labels(labels);
        self.runtime = Some(selector);
        self
    }

    /// Add one runtime label requirement.
    pub fn runtime_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let selector = self.runtime.take().unwrap_or_default().label(key, value);
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

    /// Set worker label selector.
    pub fn worker_labels(mut self, labels: RuntimeLabelSelector) -> Self {
        let selector = self.worker.take().unwrap_or_default().labels(labels);
        self.worker = Some(selector);
        self
    }

    /// Add one worker label requirement.
    pub fn worker_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let selector = self.worker.take().unwrap_or_default().label(key, value);
        self.worker = Some(selector);
        self
    }

    /// Return true when this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.binding.is_none()
            && self.function.is_none()
            && self.capability.is_none()
            && self.component.is_none()
            && self.module.is_none()
            && self.engine.is_none()
            && self.execution_modes.is_none()
            && self.platforms.is_none()
            && self.scope.is_none()
            && self.blocking.is_none()
            && self.affinity.is_none()
            && self.effect.is_none()
            && self.runtime.is_none()
            && self.worker.is_none()
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
        Self {
            match_labels: value.match_labels.clone().unwrap_or_default(),
            match_expressions: value
                .match_expressions
                .as_ref()
                .map(|expressions| {
                    expressions
                        .iter()
                        .map(RuntimeLabelRequirement::from)
                        .collect()
                })
                .unwrap_or_default(),
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

/// Runtime rule selector clause for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSelectorJson {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for full userland function names.
    pub function: Option<String>,
    /// Glob selector for capability names.
    pub capability: Option<String>,
    /// Glob selector for component names.
    pub component: Option<String>,
    /// Glob selector for module names.
    pub module: Option<String>,
    /// Engine selector.
    pub engine: Option<BindingEngineJson>,
    /// Execution mode selector.
    pub execution: Option<RuntimeExecutionSelectorJson>,
    /// Platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding scope selector.
    pub scope: Option<BindingScopeJson>,
    /// Binding blocking selector.
    pub blocking: Option<BindingBlockingJson>,
    /// Binding affinity selector.
    pub affinity: Option<BindingAffinityJson>,
    /// Binding effect selector.
    pub effect: Option<BindingEffectJson>,
    /// Runtime identity selector.
    pub runtime: Option<RuntimeIdentitySelectorJson>,
    /// Worker identity selector.
    pub worker: Option<RuntimeIdentitySelectorJson>,
}

impl From<&RuntimeSelectorJson> for RuntimeSelector {
    fn from(value: &RuntimeSelectorJson) -> Self {
        Self {
            binding: value.binding.clone(),
            function: value.function.clone(),
            capability: value.capability.clone(),
            component: value.component.clone(),
            module: value.module.clone(),
            engine: value.engine.map(BindingEngine::from),
            execution_modes: value
                .execution
                .as_ref()
                .map(RuntimeExecutionSelectorJson::to_execution_modes),
            platforms: value.platforms.clone(),
            scope: value.scope.map(BindingScope::from),
            blocking: value.blocking.map(BindingBlocking::from),
            affinity: value.affinity.map(BindingAffinity::from),
            effect: value.effect.map(BindingEffect::from),
            runtime: value.runtime.as_ref().map(RuntimeIdentitySelector::from),
            worker: value.worker.as_ref().map(RuntimeIdentitySelector::from),
        }
    }
}

/// Runtime execution-mode selector for JSON deserialization.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuntimeExecutionSelectorJson {
    /// Match one execution mode.
    One(ExecutionModeJson),
    /// Match any execution mode in this list.
    Many(Vec<ExecutionModeJson>),
}

impl RuntimeExecutionSelectorJson {
    /// Convert one JSON selector to execution modes.
    pub fn to_execution_modes(&self) -> Vec<ExecutionMode> {
        match self {
            Self::One(mode) => vec![ExecutionMode::from(*mode)],
            Self::Many(modes) => modes.iter().copied().map(ExecutionMode::from).collect(),
        }
    }
}

/// Runtime rule engine selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingEngineJson {
    /// Match VM engine execution.
    Vm,
    /// Match native engine execution.
    Native,
}

impl From<BindingEngineJson> for BindingEngine {
    fn from(value: BindingEngineJson) -> Self {
        match value {
            BindingEngineJson::Vm => BindingEngine::Vm,
            BindingEngineJson::Native => BindingEngine::Native,
        }
    }
}

/// Runtime rule scope selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingScopeJson {
    /// Match host scope bindings.
    Host,
    /// Match runtime scope bindings.
    Runtime,
}

impl From<BindingScopeJson> for BindingScope {
    fn from(value: BindingScopeJson) -> Self {
        match value {
            BindingScopeJson::Host => BindingScope::Host,
            BindingScopeJson::Runtime => BindingScope::Runtime,
        }
    }
}

/// Runtime rule blocking selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingBlockingJson {
    /// Match always-blocking bindings.
    Always,
    /// Match never-blocking bindings.
    Never,
    /// Match conditionally blocking bindings.
    Sometimes,
}

impl From<BindingBlockingJson> for BindingBlocking {
    fn from(value: BindingBlockingJson) -> Self {
        match value {
            BindingBlockingJson::Always => BindingBlocking::Always,
            BindingBlockingJson::Never => BindingBlocking::Never,
            BindingBlockingJson::Sometimes => BindingBlocking::Sometimes,
        }
    }
}

/// Runtime rule affinity selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BindingAffinityJson {
    /// Match bindings callable from any execution context.
    Any,
    /// Match bindings that require the worker event-loop context.
    EventLoop,
    /// Match bindings that require the creating execution context.
    Owner,
    /// Match bindings that require the process main context.
    ProcessMain,
}

impl From<BindingAffinityJson> for BindingAffinity {
    fn from(value: BindingAffinityJson) -> Self {
        match value {
            BindingAffinityJson::Any => BindingAffinity::Any,
            BindingAffinityJson::EventLoop => BindingAffinity::EventLoop,
            BindingAffinityJson::Owner => BindingAffinity::Owner,
            BindingAffinityJson::ProcessMain => BindingAffinity::ProcessMain,
        }
    }
}

/// Runtime rule effect selector for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BindingEffectJson {
    /// Match pure bindings.
    Pure,
    /// Match deterministic bindings.
    Deterministic,
    /// Match recordable external bindings.
    ExternalRecordable,
    /// Match non-recordable external bindings.
    ExternalNonRecordable,
}

impl From<BindingEffectJson> for BindingEffect {
    fn from(value: BindingEffectJson) -> Self {
        match value {
            BindingEffectJson::Pure => BindingEffect::Pure,
            BindingEffectJson::Deterministic => BindingEffect::Deterministic,
            BindingEffectJson::ExternalRecordable => BindingEffect::ExternalRecordable,
            BindingEffectJson::ExternalNonRecordable => BindingEffect::ExternalNonRecordable,
        }
    }
}
