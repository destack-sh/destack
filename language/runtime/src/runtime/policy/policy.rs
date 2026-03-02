use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::AgentId;
use crate::runtime::bindings::{
    BindingBlocking, BindingDescriptor, BindingEffect, BindingEffectClass, BindingEngine,
    BindingReplayPayload, BindingScope,
};
use crate::runtime::random::Random;
use destack_source::matches as glob_matches;
use destack_workspace as workspace;
use destack_workspace::{
    ExecutionMode, ReplayPayloadMode, RuntimeAccess, RuntimeLabelOperator, RuntimeSelector,
    RuntimeWorld,
};

use super::{
    Effect, Hook, HookEvent, Lifetime, ProbabilityPpm, Rule, RuleId, Trigger,
    validate_rule_fault_compatibility,
};

/// Runtime policy specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Policy {
    /// Ordered runtime rules.
    pub rules: Vec<Rule>,
}

/// Runtime policy command payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PolicyCommand {
    /// Install one runtime rule into the mutable rule set.
    Install {
        /// Rule payload to install.
        rule: Rule,
    },
    /// Remove one runtime rule from the mutable rule set.
    Remove {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Enable one runtime rule in the mutable rule set.
    Enable {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Disable one runtime rule in the mutable rule set.
    Disable {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Replace one runtime rule in the mutable rule set.
    Replace {
        /// Stable rule identifier.
        rule_id: RuleId,
        /// Replacement rule payload.
        rule: Rule,
    },
}

/// One policy decision accepted by trigger evaluation for one hook event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct PolicyDecision {
    /// Stable identifier of the rule that fired.
    pub rule_id: RuleId,
    /// Hook that produced this effect.
    pub hook: Hook,
    /// Agent identifier for this effect.
    pub agent_id: AgentId,
    /// World policy revision at fire time.
    pub policy_revision: u64,
    /// Monotonic policy event index within this revision.
    pub event_index: u64,
    /// World virtual timestamp at fire time.
    pub virtual_time_ns: u64,
    /// Number of accepted effect firings for this event.
    pub fire_count: u64,
    /// Effect payload to execute.
    pub effect: Effect,
}

impl Policy {
    /// Convert workspace static policy rules into one runtime policy specification.
    pub fn from_workspace_policy_rules(policy_rules: &[workspace::RuntimePolicyRule]) -> Self {
        let rules = policy_rules
            .iter()
            .enumerate()
            .map(|(index, rule)| Rule::from_workspace_policy_rule(index, rule))
            .collect();

        Self { rules }
    }

    /// Validate all policy invariants.
    pub fn validate(&self) -> RuntimeResult<()> {
        self.validate_unique_rule_ids()?;

        // validate each rule payload
        for rule in &self.rules {
            validate_rule_fault_compatibility(rule)?;
        }

        Ok(())
    }

    /// Return all enabled rules in declaration order.
    pub fn enabled_rules(&self) -> Vec<Rule> {
        // filter enabled rules in declaration order
        let mut enabled_rules = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            enabled_rules.push(rule.clone());
        }

        enabled_rules
    }

    /// Apply one runtime command to this policy.
    pub(crate) fn apply_command(&mut self, command: PolicyCommand) -> RuntimeResult<()> {
        // validate the command at the boundary first
        self.validate_command(&command)?;

        // apply the command under trusted invariants
        self.apply_command_unchecked(command);

        Ok(())
    }

    /// Validate one runtime command against the current policy.
    fn validate_command(&self, command: &PolicyCommand) -> RuntimeResult<()> {
        match command {
            PolicyCommand::Install { rule } => {
                if self.has_rule_id(&rule.id) {
                    return Err(Self::invalid_command_error(format!(
                        "runtime install requires unique rule ids: {}",
                        rule.id.0
                    )));
                }

                validate_rule_fault_compatibility(rule)?;
            }
            PolicyCommand::Remove { rule_id }
            | PolicyCommand::Enable { rule_id }
            | PolicyCommand::Disable { rule_id } => {
                if self.has_rule_id(rule_id) {
                    return Ok(());
                }

                return Err(Self::invalid_command_error(format!(
                    "runtime command requires one installed rule id: {}",
                    rule_id.0
                )));
            }
            PolicyCommand::Replace { rule_id, rule } => {
                if rule.id != *rule_id {
                    return Err(Self::invalid_command_error(format!(
                        "runtime replace requires replacement id to match: {}",
                        rule_id.0
                    )));
                }

                if !self.has_rule_id(rule_id) {
                    return Err(Self::invalid_command_error(format!(
                        "runtime replace requires one installed rule id: {}",
                        rule_id.0
                    )));
                }

                validate_rule_fault_compatibility(rule)?;
            }
        }

        Ok(())
    }

    /// Apply one already-validated command to this policy.
    fn apply_command_unchecked(&mut self, command: PolicyCommand) {
        match command {
            PolicyCommand::Install { mut rule } => {
                rule.enabled = true;
                self.rules.push(rule);
            }
            PolicyCommand::Remove { rule_id } => {
                let is_removed = self.remove_rule(&rule_id);
                debug_assert!(is_removed);
            }
            PolicyCommand::Enable { rule_id } => {
                let is_updated = self.set_rule_enabled(&rule_id, true);
                debug_assert!(is_updated);
            }
            PolicyCommand::Disable { rule_id } => {
                let is_updated = self.set_rule_enabled(&rule_id, false);
                debug_assert!(is_updated);
            }
            PolicyCommand::Replace { rule_id, mut rule } => {
                rule.enabled = true;
                let is_replaced = self.replace_rule(&rule_id, rule);
                debug_assert!(is_replaced);
            }
        }
    }

    /// Return true when one rule id exists in this policy.
    fn has_rule_id(&self, rule_id: &RuleId) -> bool {
        self.rules.iter().any(|rule| rule.id == *rule_id)
    }

    /// Return true when one matching rule id exists and has been replaced.
    fn replace_rule(&mut self, rule_id: &RuleId, replacement: Rule) -> bool {
        let mut is_replaced = false;

        for rule in &mut self.rules {
            if rule.id != *rule_id {
                continue;
            }

            *rule = replacement.clone();
            is_replaced = true;
        }

        is_replaced
    }

    /// Return true when one matching rule id exists and has been removed.
    fn remove_rule(&mut self, rule_id: &RuleId) -> bool {
        let before_len = self.rules.len();
        self.rules.retain(|rule| rule.id != *rule_id);
        self.rules.len() < before_len
    }

    /// Return true when one matching rule id exists and has been updated.
    fn set_rule_enabled(&mut self, rule_id: &RuleId, is_enabled: bool) -> bool {
        let mut is_updated = false;

        for rule in &mut self.rules {
            if rule.id != *rule_id {
                continue;
            }

            rule.enabled = is_enabled;
            is_updated = true;
        }

        is_updated
    }

    /// Return one invalid-policy error.
    fn invalid_policy_error(message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }

    /// Return one invalid-policy-command error.
    fn invalid_command_error(message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }

    /// Validate that this policy contains unique rule ids.
    fn validate_unique_rule_ids(&self) -> RuntimeResult<()> {
        let mut installed_rule_ids: HashSet<RuleId> = HashSet::new();
        for rule in &self.rules {
            if installed_rule_ids.insert(rule.id.clone()) {
                continue;
            }

            return Err(Self::invalid_policy_error(format!(
                "runtime rules require unique rule ids: {}",
                rule.id.0
            )));
        }

        Ok(())
    }
}

/// Runtime and agent identity data used for selector matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyIdentity {
    /// Runtime name for selector matching.
    pub runtime_name: String,
    /// Runtime labels for selector matching.
    pub runtime_labels: BTreeMap<String, String>,
    /// Agent name for selector matching.
    pub agent_name: String,
    /// Agent labels for selector matching.
    pub agent_labels: BTreeMap<String, String>,
}

/// Runtime counters and gate state for one rule in one scope.
#[derive(Debug, Clone, Default)]
pub(crate) struct RuleState {
    /// Total matched decisions seen in this state.
    pub matched_decisions_seen: u64,
    /// Total accepted trigger firings for this state.
    pub fires: u64,
    /// Cooldown-until timestamp in virtual nanoseconds.
    pub cooldown_until_virtual_ns: Option<u64>,
    /// Whether this rule has activated.
    pub is_activated: bool,
    /// Activation timestamp in virtual nanoseconds.
    pub activated_at_virtual_ns: Option<u64>,
    /// Matching hits observed after activation.
    pub active_hits_seen: u64,
    /// Whether this rule has expired.
    pub is_expired: bool,
}

/// Active policy and runtime trigger state.
#[derive(Debug, Clone)]
pub(crate) struct PolicyState {
    /// Active policy specification.
    pub spec: Policy,
    /// World-local monotonic policy revision.
    pub revision: u64,
    /// Enabled rules for this policy revision.
    enabled_rules: Vec<Rule>,
    /// Total matching call events seen.
    total_calls_seen: u64,
    /// Total policy events seen.
    total_events_seen: u64,
    /// Runtime rule state by rule id and agent id.
    rule_states: HashMap<RuleId, HashMap<AgentId, RuleState>>,
}

impl PolicyState {
    /// Create one active policy with empty runtime state.
    pub(crate) fn new(policy: Policy) -> Self {
        debug_assert!(policy.validate().is_ok());
        let enabled_rules = policy.enabled_rules();

        Self {
            spec: policy,
            revision: 1,
            enabled_rules,
            total_calls_seen: 0,
            total_events_seen: 0,
            rule_states: HashMap::new(),
        }
    }

    /// Replace active policy and reset runtime trigger state.
    pub(crate) fn set_policy(&mut self, policy: Policy) -> RuntimeResult<()> {
        policy.validate()?;

        self.spec = policy;
        self.revision = self.revision.saturating_add(1);
        self.refresh_enabled_rules();
        self.reset_runtime_state();

        Ok(())
    }

    /// Apply one policy command and reset runtime trigger state.
    pub(crate) fn apply_policy_command(&mut self, command: PolicyCommand) -> RuntimeResult<()> {
        self.spec.apply_command(command)?;
        self.revision = self.revision.saturating_add(1);
        self.refresh_enabled_rules();
        self.reset_runtime_state();

        Ok(())
    }

    /// Resolve one access decision for one binding call.
    pub(crate) fn resolve_binding_access(
        &self,
        identity: &PolicyIdentity,
        mode: ExecutionMode,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_access: RuntimeAccess,
    ) -> RuntimeAccess {
        for rule in &self.enabled_rules {
            if rule.trigger.is_some() {
                continue;
            }

            if !self.selector_matches(&rule.when, identity, Some(descriptor), mode, engine) {
                continue;
            }

            let Effect::SetAccess { access } = &rule.action else {
                continue;
            };

            return *access;
        }

        default_access
    }

    /// Resolve one world decision for one binding call.
    pub(crate) fn resolve_binding_world(
        &self,
        identity: &PolicyIdentity,
        mode: ExecutionMode,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_world: RuntimeWorld,
    ) -> RuntimeWorld {
        // runtime bindings are always host owned
        if descriptor.scope == BindingScope::Runtime {
            return RuntimeWorld::Host;
        }

        for rule in &self.enabled_rules {
            if rule.trigger.is_some() {
                continue;
            }

            if !self.selector_matches(&rule.when, identity, Some(descriptor), mode, engine) {
                continue;
            }

            let Effect::SetWorld { world } = &rule.action else {
                continue;
            };

            return *world;
        }

        default_world
    }

    /// Resolve one replay payload decision for one binding call.
    pub(crate) fn resolve_binding_replay_payload(
        &self,
        identity: &PolicyIdentity,
        mode: ExecutionMode,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_replay_payload: BindingReplayPayload,
    ) -> BindingReplayPayload {
        for rule in &self.enabled_rules {
            if rule.trigger.is_some() {
                continue;
            }

            if !self.selector_matches(&rule.when, identity, Some(descriptor), mode, engine) {
                continue;
            }

            let Effect::SetReplay { payload } = &rule.action else {
                continue;
            };

            return Self::replay_payload_from_mode(*payload);
        }

        default_replay_payload
    }

    /// Evaluate one policy event.
    pub(crate) fn on_event(
        &mut self,
        event: &HookEvent,
        identity: &PolicyIdentity,
        mode: ExecutionMode,
        random: &Random,
    ) -> Vec<PolicyDecision> {
        // update event counters for policy metadata and activation windows
        self.total_events_seen = self.total_events_seen.saturating_add(1);
        let event_index = self.total_events_seen;

        if event.is_call_event {
            self.total_calls_seen = self.total_calls_seen.saturating_add(1);
        }

        let total_calls_seen = self.total_calls_seen;
        let mut decisions = Vec::new();

        // evaluate each matched rule in declaration order
        for rule in &self.enabled_rules {
            if !self.selector_matches(
                &rule.when,
                identity,
                event.descriptor,
                mode,
                event.state.engine,
            ) {
                continue;
            }

            if !matches!(rule.action, Effect::Fault { .. }) {
                continue;
            }

            if !rule.matches_hook(event.hook) {
                continue;
            }

            let Some(trigger) = rule.trigger.as_ref() else {
                continue;
            };

            let rule_state = self
                .rule_states
                .entry(rule.id.clone())
                .or_default()
                .entry(event.agent_id)
                .or_default();

            if !rule_state.ensure_activated(
                trigger,
                total_calls_seen,
                event.virtual_time_ns,
                random,
            ) {
                continue;
            }

            rule_state.active_hits_seen = rule_state.active_hits_seen.saturating_add(1);

            if rule_state.update_expired(trigger, event.virtual_time_ns) {
                continue;
            }

            if !rule_state.matches_cadence(trigger) {
                continue;
            }

            if !rule_state.matches_cooldown(trigger, event.virtual_time_ns) {
                continue;
            }

            if !RuleState::matches_probability(trigger.probability_ppm, random) {
                continue;
            }

            let fire_count = rule_state.accepted_fire_count(trigger);
            if fire_count == 0 {
                continue;
            }

            rule_state.record_fire(trigger, event.virtual_time_ns, fire_count);

            decisions.push(PolicyDecision {
                rule_id: rule.id.clone(),
                hook: event.hook,
                agent_id: event.agent_id,
                policy_revision: self.revision,
                event_index,
                virtual_time_ns: event.virtual_time_ns,
                fire_count,
                effect: rule.action.clone(),
            });
        }

        decisions
    }

    /// Return matched decision totals across all scopes per enabled rule.
    #[cfg(test)]
    pub(crate) fn matched_decisions_seen_totals(&self) -> Vec<u64> {
        let mut totals = Vec::new();
        for rule in &self.enabled_rules {
            let total = self
                .rule_states
                .get(&rule.id)
                .map(|state_by_scope| {
                    state_by_scope.values().fold(0u64, |sum, rule_state| {
                        sum.saturating_add(rule_state.matched_decisions_seen)
                    })
                })
                .unwrap_or(0);
            totals.push(total);
        }

        totals
    }

    /// Refresh enabled rule cache from the policy specification.
    fn refresh_enabled_rules(&mut self) {
        self.enabled_rules = self.spec.enabled_rules();
    }

    /// Reset all runtime state for the active policy revision.
    fn reset_runtime_state(&mut self) {
        self.total_calls_seen = 0;
        self.total_events_seen = 0;
        self.rule_states = HashMap::new();
    }

    /// Return true when one selector matches one policy identity and call context.
    fn selector_matches(
        &self,
        selector: &RuntimeSelector,
        identity: &PolicyIdentity,
        descriptor: Option<BindingDescriptor>,
        mode: ExecutionMode,
        engine: Option<BindingEngine>,
    ) -> bool {
        // descriptor-aware selectors require one descriptor
        if descriptor.is_none() && Self::selector_has_binding_clauses(selector) {
            return false;
        }

        // match runtime identity selector
        if let Some(runtime_selector) = &selector.runtime
            && !Self::matches_identity_selector(
                runtime_selector,
                &identity.runtime_name,
                &identity.runtime_labels,
            )
        {
            return false;
        }

        // match agent identity selector
        if let Some(agent_selector) = &selector.agent
            && !Self::matches_identity_selector(
                agent_selector,
                &identity.agent_name,
                &identity.agent_labels,
            )
        {
            return false;
        }

        // match engine selector
        if let Some(rule_engine) = selector.engine {
            let Some(engine) = engine else {
                return false;
            };
            if !Self::matches_engine(rule_engine, engine) {
                return false;
            }
        }

        // match execution mode selector
        if let Some(modes) = &selector.execution_modes {
            let matches_mode = modes.iter().copied().any(|rule_mode| rule_mode == mode);
            if !matches_mode {
                return false;
            }
        }

        // match platform selector
        if let Some(platforms) = &selector.platforms {
            let current_platform = Self::runtime_platform_name();
            let matches_platform = platforms
                .iter()
                .any(|pattern| Self::glob_match(pattern, current_platform));
            if !matches_platform {
                return false;
            }
        }

        // match descriptor-aware selectors
        if let Some(descriptor) = descriptor {
            if let Some(pattern) = &selector.binding
                && !Self::glob_match(pattern, descriptor.name)
            {
                return false;
            }

            if let Some(pattern) = &selector.module {
                let module_name = descriptor
                    .name
                    .strip_prefix("destack.")
                    .unwrap_or(descriptor.name);
                if !Self::glob_match(pattern, module_name) {
                    return false;
                }
            }

            if let Some(pattern) = &selector.component {
                let component_name = Self::binding_component_name(descriptor.name);
                if !Self::glob_match(pattern, component_name) {
                    return false;
                }
            }

            if let Some(pattern) = &selector.capability {
                let has_match = descriptor
                    .requires()
                    .iter()
                    .any(|capability| Self::glob_match(pattern, capability));
                if !has_match {
                    return false;
                }
            }

            if let Some(scope) = selector.scope
                && !Self::matches_scope(scope, descriptor.scope())
            {
                return false;
            }

            if let Some(blocking) = selector.blocking
                && !Self::matches_blocking(blocking, descriptor.blocking())
            {
                return false;
            }

            if let Some(effect) = selector.effect
                && !Self::matches_effect(effect, descriptor.effect_class)
            {
                return false;
            }
        }

        true
    }

    /// Return true when one selector depends on descriptor fields.
    fn selector_has_binding_clauses(selector: &RuntimeSelector) -> bool {
        selector.binding.is_some()
            || selector.capability.is_some()
            || selector.component.is_some()
            || selector.module.is_some()
            || selector.scope.is_some()
            || selector.blocking.is_some()
            || selector.effect.is_some()
    }

    /// Return true when one identity selector matches name and labels.
    fn matches_identity_selector(
        selector: &workspace::RuntimeIdentitySelector,
        name: &str,
        labels: &BTreeMap<String, String>,
    ) -> bool {
        if let Some(name_pattern) = &selector.name
            && !Self::glob_match(name_pattern, name)
        {
            return false;
        }

        if let Some(label_selector) = &selector.labels
            && !Self::matches_label_selector(label_selector, labels)
        {
            return false;
        }

        true
    }

    /// Return true when one label selector matches labels.
    fn matches_label_selector(
        selector: &workspace::RuntimeLabelSelector,
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
            if !Self::matches_label_requirement(requirement, labels) {
                return false;
            }
        }

        true
    }

    /// Return true when one label requirement matches labels.
    fn matches_label_requirement(
        requirement: &workspace::RuntimeLabelRequirement,
        labels: &BTreeMap<String, String>,
    ) -> bool {
        match requirement.operator {
            RuntimeLabelOperator::In => {
                let Some(actual_value) = labels.get(&requirement.key) else {
                    return false;
                };
                requirement.values.iter().any(|value| value == actual_value)
            }
            RuntimeLabelOperator::NotIn => {
                let Some(actual_value) = labels.get(&requirement.key) else {
                    return false;
                };
                !requirement.values.iter().any(|value| value == actual_value)
            }
            RuntimeLabelOperator::Exists => labels.contains_key(&requirement.key),
            RuntimeLabelOperator::DoesNotExist => !labels.contains_key(&requirement.key),
        }
    }

    /// Convert workspace replay mode into runtime replay payload.
    const fn replay_payload_from_mode(mode: ReplayPayloadMode) -> BindingReplayPayload {
        match mode {
            ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
            ReplayPayloadMode::ArgumentsAndResults => BindingReplayPayload::ArgumentsAndResults,
        }
    }

    /// Return true when one engine selector matches the call engine.
    fn matches_engine(rule_engine: BindingEngine, engine: BindingEngine) -> bool {
        rule_engine == engine
    }

    /// Return true when one scope selector matches the binding scope.
    fn matches_scope(rule_scope: BindingScope, scope: BindingScope) -> bool {
        rule_scope == scope
    }

    /// Return true when one blocking selector matches the binding class.
    fn matches_blocking(rule_blocking: BindingBlocking, blocking: BindingBlocking) -> bool {
        rule_blocking == blocking
    }

    /// Return true when one effect selector matches the binding effect class.
    fn matches_effect(rule_effect: BindingEffect, effect_class: BindingEffectClass) -> bool {
        rule_effect == effect_class.binding_effect()
    }

    /// Match one text value against one glob pattern.
    fn glob_match(pattern: &str, text: &str) -> bool {
        glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
    }

    /// Return the component segment for one binding id.
    fn binding_component_name(binding_name: &str) -> &str {
        let stripped_name = binding_name
            .strip_prefix("destack.")
            .unwrap_or(binding_name);
        let mut component_segments = stripped_name.splitn(2, '.');
        component_segments.next().unwrap_or("unknown")
    }

    /// Return the current compile target platform name.
    fn runtime_platform_name() -> &'static str {
        #[cfg(target_os = "linux")]
        {
            return "linux";
        }

        #[cfg(target_os = "macos")]
        {
            "macos"
        }

        #[cfg(target_os = "ios")]
        {
            "ios"
        }

        #[cfg(target_os = "windows")]
        {
            "windows"
        }

        #[cfg(target_os = "wasi")]
        {
            return "wasi";
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "ios",
            target_os = "windows",
            target_os = "wasi"
        )))]
        {
            "unknown"
        }
    }
}

impl RuleState {
    /// Return true when this state can transition into the activated state.
    fn ensure_activated(
        &mut self,
        trigger: &Trigger,
        total_calls_seen: u64,
        now_virtual_ns: u64,
        random: &Random,
    ) -> bool {
        if self.is_expired {
            return false;
        }
        if self.is_activated {
            return true;
        }

        if let Some(activation_window) = trigger.activation
            && !activation_window.is_reached(total_calls_seen, now_virtual_ns)
        {
            return false;
        }

        if !Self::matches_probability(trigger.activation_ppm, random) {
            return false;
        }

        self.is_activated = true;
        self.activated_at_virtual_ns = Some(now_virtual_ns);
        self.active_hits_seen = 0;
        true
    }

    /// Update expiration state and return whether this state is expired.
    fn update_expired(&mut self, trigger: &Trigger, now_virtual_ns: u64) -> bool {
        if self.is_expired {
            return true;
        }

        let Some(lifetime) = trigger.lifetime else {
            return false;
        };

        let is_expired = match lifetime {
            Lifetime::UntilDisabled => false,
            Lifetime::ForDurationNs { duration_ns } => {
                let activated_at_virtual_ns =
                    self.activated_at_virtual_ns.unwrap_or(now_virtual_ns);
                let elapsed = now_virtual_ns.saturating_sub(activated_at_virtual_ns);
                elapsed >= duration_ns
            }
            Lifetime::ForCallCount { call_count } => self.active_hits_seen > call_count,
        };

        self.is_expired = is_expired;
        is_expired
    }

    /// Return true when this state passes cadence controls.
    fn matches_cadence(&self, trigger: &Trigger) -> bool {
        let skip_hits = trigger.skip_hits.unwrap_or(0);
        if self.active_hits_seen <= skip_hits {
            return false;
        }

        let Some(interval_hits) = trigger.interval_hits else {
            return true;
        };

        let interval_hits = interval_hits.max(1);
        let cadence_offset = self
            .active_hits_seen
            .saturating_sub(skip_hits.saturating_add(1));

        cadence_offset.is_multiple_of(interval_hits)
    }

    /// Return true when this state passes cooldown controls.
    fn matches_cooldown(&self, trigger: &Trigger, now_virtual_ns: u64) -> bool {
        let Some(cooldown_ns) = trigger.cooldown_ns else {
            return true;
        };
        if cooldown_ns == 0 {
            return true;
        }

        let Some(cooldown_until_virtual_ns) = self.cooldown_until_virtual_ns else {
            return true;
        };

        now_virtual_ns >= cooldown_until_virtual_ns
    }

    /// Return the accepted fire count for one event hit.
    fn accepted_fire_count(&self, trigger: &Trigger) -> u64 {
        let burst = u64::from(trigger.burst.unwrap_or(1).max(1));
        let Some(max_occurrences) = trigger.max_occurrences else {
            return burst;
        };
        if self.fires >= max_occurrences {
            return 0;
        }

        let remaining = max_occurrences.saturating_sub(self.fires);
        burst.min(remaining)
    }

    /// Record one accepted firing on this state.
    fn record_fire(&mut self, trigger: &Trigger, now_virtual_ns: u64, fire_count: u64) {
        self.matched_decisions_seen = self.matched_decisions_seen.saturating_add(fire_count);
        self.fires = self.fires.saturating_add(fire_count);

        if let Some(cooldown_ns) = trigger.cooldown_ns
            && cooldown_ns > 0
        {
            self.cooldown_until_virtual_ns = Some(now_virtual_ns.saturating_add(cooldown_ns));
        }
    }

    /// Return true when one probability gate accepts this event.
    fn matches_probability(probability_ppm: Option<ProbabilityPpm>, random: &Random) -> bool {
        let Some(probability_ppm) = probability_ppm else {
            return true;
        };
        let bounded_ppm = probability_ppm.0.min(1_000_000);
        if bounded_ppm == 0 {
            return false;
        }
        if bounded_ppm == 1_000_000 {
            return true;
        }

        let draw_ppm = random.next_u64() % 1_000_000;
        draw_ppm < u64::from(bounded_ppm)
    }
}
