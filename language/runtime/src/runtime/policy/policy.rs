use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::WorkerId;
use crate::runtime::binding::{
    BindingDescriptor, BindingEngine, BindingProvider, BindingReplayPayload, RuntimeAccess,
    RuntimeWorld,
};
use crate::runtime::random::Random;
use destack_workspace::{ExecutionMode, ReplayPayloadMode};

use super::{
    CallSubject, FaultKindCatalog, FaultTarget, Hook, HookEvent, Lifetime, PolicyCallId,
    ProbabilityPpm, Rule, RuleAction, RuleId, Trigger, validate_rule_fault_compatibility,
};

/// Runtime policy specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Policy {
    /// Ordered runtime rules.
    pub rules: Vec<Rule>,
}

/// One policy decision accepted by trigger evaluation for one policy event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct PolicyDecision {
    /// Stable identifier of the rule that fired.
    pub rule_id: RuleId,
    /// Hook that produced this action.
    pub hook: Hook,
    /// Worker identifier for this action.
    pub worker_id: WorkerId,
    /// Binding call identifier when one call event fired this decision.
    pub call_id: Option<PolicyCallId>,
    /// Rule action payload to execute.
    pub action: RuleAction,
}

/// Binding decision payload used by hot binding call paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BindingDecision {
    /// Final access decision after rule evaluation.
    pub access: RuntimeAccess,
    /// Final world decision after rule evaluation.
    pub world: RuntimeWorld,
    /// Final replay payload decision after rule evaluation.
    pub replay_payload: BindingReplayPayload,
}

/// Rule matching subject for policy evaluation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuleSubject<'a> {
    /// Runtime name for selector matching.
    pub runtime_name: &'a str,
    /// Runtime labels for selector matching.
    pub runtime_labels: &'a BTreeMap<String, String>,
    /// Worker name for selector matching.
    pub worker_name: &'a str,
    /// Worker labels for selector matching.
    pub worker_labels: &'a BTreeMap<String, String>,
    /// Execution mode for selector matching.
    pub mode: ExecutionMode,
}

impl<'a> RuleSubject<'a> {
    /// Create one rule matching subject.
    pub(crate) fn new(
        runtime_name: &'a str,
        runtime_labels: &'a BTreeMap<String, String>,
        worker_name: &'a str,
        worker_labels: &'a BTreeMap<String, String>,
        mode: ExecutionMode,
    ) -> Self {
        Self {
            runtime_name,
            runtime_labels,
            worker_name,
            worker_labels,
            mode,
        }
    }
}

impl Policy {
    /// Validate all policy invariants against one topology kind catalog.
    pub(crate) fn validate_with_kind_catalog(
        &self,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        self.validate_unique_rule_ids()?;

        // validate each rule payload
        for rule in &self.rules {
            Self::validate_rule(rule, kind_catalog)?;
        }

        Ok(())
    }

    /// Install one rule into this policy with full validation.
    pub(crate) fn install_rule(
        &mut self,
        mut rule: Rule,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        if self.has_rule_id(&rule.id) {
            return Err(Self::invalid_policy_error(format!(
                "runtime install requires unique rule ids: {}",
                rule.id.0
            )));
        }

        Self::validate_rule(&rule, kind_catalog)?;
        rule.enabled = true;
        self.rules.push(rule);

        Ok(())
    }

    /// Remove one rule from this policy.
    pub(crate) fn remove_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        if self.remove_rule_unchecked(rule_id) {
            return Ok(());
        }

        Err(Self::invalid_policy_error(format!(
            "runtime command requires one installed rule id: {}",
            rule_id.0
        )))
    }

    /// Enable one rule in this policy.
    pub(crate) fn enable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        if self.set_rule_enabled_unchecked(rule_id, true) {
            return Ok(());
        }

        Err(Self::invalid_policy_error(format!(
            "runtime command requires one installed rule id: {}",
            rule_id.0
        )))
    }

    /// Disable one rule in this policy.
    pub(crate) fn disable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        if self.set_rule_enabled_unchecked(rule_id, false) {
            return Ok(());
        }

        Err(Self::invalid_policy_error(format!(
            "runtime command requires one installed rule id: {}",
            rule_id.0
        )))
    }

    /// Replace one rule in this policy with full validation.
    pub(crate) fn replace_rule(
        &mut self,
        rule_id: &RuleId,
        mut rule: Rule,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        if rule.id != *rule_id {
            return Err(Self::invalid_policy_error(format!(
                "runtime replace requires replacement id to match: {}",
                rule_id.0
            )));
        }

        if !self.has_rule_id(rule_id) {
            return Err(Self::invalid_policy_error(format!(
                "runtime replace requires one installed rule id: {}",
                rule_id.0
            )));
        }

        Self::validate_rule(&rule, kind_catalog)?;
        rule.enabled = true;
        let is_replaced = self.replace_rule_unchecked(rule_id, rule);
        debug_assert!(is_replaced);

        Ok(())
    }

    /// Return true when one rule id exists in this policy.
    fn has_rule_id(&self, rule_id: &RuleId) -> bool {
        self.rules.iter().any(|rule| rule.id == *rule_id)
    }

    /// Return true when one matching rule id exists and has been replaced.
    fn replace_rule_unchecked(&mut self, rule_id: &RuleId, replacement: Rule) -> bool {
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
    fn remove_rule_unchecked(&mut self, rule_id: &RuleId) -> bool {
        let before_len = self.rules.len();
        self.rules.retain(|rule| rule.id != *rule_id);
        self.rules.len() < before_len
    }

    /// Return true when one matching rule id exists and has been updated.
    fn set_rule_enabled_unchecked(&mut self, rule_id: &RuleId, is_enabled: bool) -> bool {
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

    /// Validate one rule against shape and catalog constraints.
    fn validate_rule(rule: &Rule, kind_catalog: &impl FaultKindCatalog) -> RuntimeResult<()> {
        Self::validate_rule_shape(rule)?;
        validate_rule_fault_compatibility(rule, kind_catalog)
    }

    /// Validate rule shape invariants.
    fn validate_rule_shape(rule: &Rule) -> RuntimeResult<()> {
        let has_trigger = rule.trigger.is_some();
        let has_call_selector = rule.call.is_some();

        // binding decisions require an explicit call selector
        if Self::is_binding_decision_action(&rule.action) && !has_call_selector {
            return Err(Self::invalid_policy_error(format!(
                "runtime binding decision rule {} requires a call selector",
                rule.id.0
            )));
        }

        // binding decisions are static selectors and cannot carry triggers
        if Self::is_binding_decision_action(&rule.action) && has_trigger {
            return Err(Self::invalid_policy_error(format!(
                "runtime binding decision rule {} must not define a trigger",
                rule.id.0
            )));
        }

        // fault and custom actions are dynamic and must carry triggers
        if matches!(
            rule.action,
            RuleAction::Fault { .. } | RuleAction::Custom { .. }
        ) && !has_trigger
        {
            return Err(Self::invalid_policy_error(format!(
                "runtime action rule {} requires a trigger",
                rule.id.0
            )));
        }

        // call-target faults require call selectors
        if let RuleAction::Fault { fault } = &rule.action
            && matches!(fault.target, FaultTarget::Call {})
            && !has_call_selector
        {
            return Err(Self::invalid_policy_error(format!(
                "runtime call fault rule {} requires a call selector",
                rule.id.0
            )));
        }

        Ok(())
    }

    /// Return true when one action is a static binding decision action.
    fn is_binding_decision_action(action: &RuleAction) -> bool {
        matches!(
            action,
            RuleAction::SetWorld { .. }
                | RuleAction::SetAccess { .. }
                | RuleAction::SetReplay { .. }
        )
    }

    /// Return one invalid-policy error.
    fn invalid_policy_error(message: impl Into<String>) -> Box<RuntimeError> {
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

/// Runtime counters and gate state for one rule in one scope.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct RuleState {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PolicyState {
    /// Active policy specification.
    pub spec: Policy,
    /// Enabled static binding decision rule indices for this policy revision.
    binding_rule_indices: Vec<usize>,
    /// Enabled triggered action rule indices for this policy revision.
    triggered_rule_indices: Vec<usize>,
    /// Total matching call events seen per worker.
    total_calls_seen_by_worker: HashMap<WorkerId, u64>,
    /// Runtime rule state by rule id and worker id.
    rule_states: HashMap<RuleId, HashMap<WorkerId, RuleState>>,
}

impl PolicyState {
    /// Create one active policy with empty runtime state.
    pub(crate) fn new(policy: Policy) -> Self {
        let (binding_rule_indices, triggered_rule_indices) =
            Self::split_enabled_rule_indices(&policy);

        Self {
            spec: policy,
            binding_rule_indices,
            triggered_rule_indices,
            total_calls_seen_by_worker: HashMap::new(),
            rule_states: HashMap::new(),
        }
    }

    /// Replace active policy and reset runtime trigger state.
    pub(crate) fn set_policy(
        &mut self,
        policy: Policy,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        policy.validate_with_kind_catalog(kind_catalog)?;

        self.replace_policy(policy);

        Ok(())
    }

    /// Install one rule and reset runtime trigger state.
    pub(crate) fn install_rule(
        &mut self,
        rule: Rule,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.install_rule(rule, kind_catalog)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Remove one rule and reset runtime trigger state.
    pub(crate) fn remove_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.remove_rule(rule_id)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Enable one rule and reset runtime trigger state.
    pub(crate) fn enable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.enable_rule(rule_id)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Disable one rule and reset runtime trigger state.
    pub(crate) fn disable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.disable_rule(rule_id)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Replace one rule and reset runtime trigger state.
    pub(crate) fn replace_rule(
        &mut self,
        rule_id: &RuleId,
        rule: Rule,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.replace_rule(rule_id, rule, kind_catalog)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Resolve binding decisions for one binding call in one selector scope.
    pub(crate) fn resolve_binding_for_subject(
        &self,
        subject: RuleSubject<'_>,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_access: RuntimeAccess,
        default_world: RuntimeWorld,
        default_replay_payload: BindingReplayPayload,
    ) -> BindingDecision {
        self.resolve_binding(
            subject.runtime_name,
            subject.runtime_labels,
            subject.worker_name,
            subject.worker_labels,
            subject.mode,
            descriptor,
            engine,
            default_access,
            default_world,
            default_replay_payload,
        )
    }

    /// Evaluate one policy event in one selector scope.
    pub(crate) fn on_event_for_subject(
        &mut self,
        event: &HookEvent,
        subject: RuleSubject<'_>,
        random: &Random,
    ) -> Vec<PolicyDecision> {
        self.on_event(
            event,
            subject.runtime_name,
            subject.runtime_labels,
            subject.worker_name,
            subject.worker_labels,
            subject.mode,
            random,
        )
    }

    /// Resolve binding decisions for one binding call without explanation metadata.
    pub(crate) fn resolve_binding(
        &self,
        runtime_name: &str,
        runtime_labels: &BTreeMap<String, String>,
        worker_name: &str,
        worker_labels: &BTreeMap<String, String>,
        mode: ExecutionMode,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        default_access: RuntimeAccess,
        default_world: RuntimeWorld,
        default_replay_payload: BindingReplayPayload,
    ) -> BindingDecision {
        let mut access = default_access;
        let mut world = default_world;
        let mut replay_payload = default_replay_payload;
        let mut has_access_decision = false;
        let mut has_world_decision = descriptor.provider == BindingProvider::Runtime;
        let mut has_replay_decision = false;

        // runtime bindings are always host owned
        if descriptor.provider == BindingProvider::Runtime {
            world = RuntimeWorld::Host;
        }

        for rule_index in &self.binding_rule_indices {
            let rule = &self.spec.rules[*rule_index];
            if !Self::matches_call_selector(
                rule,
                runtime_name,
                runtime_labels,
                worker_name,
                worker_labels,
                Some(descriptor),
                mode,
                engine,
            ) {
                continue;
            }

            if !has_access_decision
                && let RuleAction::SetAccess {
                    access: selected_access,
                } = &rule.action
            {
                access = *selected_access;
                has_access_decision = true;
            }

            if !has_world_decision
                && let RuleAction::SetWorld {
                    world: selected_world,
                } = &rule.action
            {
                world = *selected_world;
                has_world_decision = true;
            }

            if !has_replay_decision && let RuleAction::SetReplay { payload } = &rule.action {
                replay_payload = match *payload {
                    ReplayPayloadMode::ResultsOnly => BindingReplayPayload::Results,
                    ReplayPayloadMode::ArgumentsAndResults => {
                        BindingReplayPayload::ArgumentsAndResults
                    }
                };
                has_replay_decision = true;
            }

            if has_access_decision && has_world_decision && has_replay_decision {
                break;
            }
        }

        BindingDecision {
            access,
            world,
            replay_payload,
        }
    }

    /// Evaluate one policy event.
    pub(crate) fn on_event(
        &mut self,
        event: &HookEvent,
        runtime_name: &str,
        runtime_labels: &BTreeMap<String, String>,
        worker_name: &str,
        worker_labels: &BTreeMap<String, String>,
        mode: ExecutionMode,
        random: &Random,
    ) -> Vec<PolicyDecision> {
        let event_worker_id = event.worker_id();

        // update call counters for activation windows
        let total_calls_seen = {
            let calls_seen = self
                .total_calls_seen_by_worker
                .entry(event_worker_id)
                .or_insert(0);
            if event.counts_as_call_event() {
                *calls_seen = calls_seen.saturating_add(1);
            }

            *calls_seen
        };

        // capture the event-local evaluation state
        let event_hook = event.hook();
        let event_call_id = event.call_id();
        let event_virtual_time_ns = event.virtual_time_ns();
        let mut decisions = Vec::new();

        // evaluate each matched rule in declaration order
        for rule_index in &self.triggered_rule_indices {
            let rule = &self.spec.rules[*rule_index];

            // skip non-matching selectors early
            if !Self::matches_call_selector(
                rule,
                runtime_name,
                runtime_labels,
                worker_name,
                worker_labels,
                event.binding_descriptor(),
                mode,
                event.engine(),
            ) {
                continue;
            }

            // skip non-matching hooks early
            if !rule.matches_hook(event_hook) {
                continue;
            }

            // fault actions must always carry a trigger
            let Some(trigger) = rule.trigger.as_ref() else {
                continue;
            };

            // load mutable state for this rule and worker
            let rule_state = self
                .rule_states
                .entry(rule.id.clone())
                .or_default()
                .entry(event_worker_id)
                .or_default();

            // evaluate trigger gates and record accepted firings
            let fire_count =
                rule_state.accept_event(trigger, total_calls_seen, event_virtual_time_ns, random);
            if fire_count == 0 {
                continue;
            }

            // emit one decision for each accepted rule match
            decisions.push(PolicyDecision {
                rule_id: rule.id.clone(),
                hook: event_hook,
                worker_id: event_worker_id,
                call_id: event_call_id,
                action: rule.action.clone(),
            });
        }

        decisions
    }

    /// Rebuild enabled rule indices from the active policy.
    fn rebuild_rule_indices(&mut self) {
        let (binding_rule_indices, triggered_rule_indices) =
            Self::split_enabled_rule_indices(&self.spec);
        self.binding_rule_indices = binding_rule_indices;
        self.triggered_rule_indices = triggered_rule_indices;
    }

    /// Replace the active policy and reset runtime trigger state.
    fn replace_policy(&mut self, policy: Policy) {
        self.spec = policy;
        self.rebuild_rule_indices();
        self.reset_runtime_state();
    }

    /// Split enabled rule indices into binding and triggered-action rules.
    fn split_enabled_rule_indices(policy: &Policy) -> (Vec<usize>, Vec<usize>) {
        let mut binding_rule_indices = Vec::new();
        let mut triggered_rule_indices = Vec::new();

        for (index, rule) in policy.rules.iter().enumerate() {
            if !rule.enabled {
                continue;
            }

            if rule.trigger.is_some() {
                triggered_rule_indices.push(index);
            } else {
                binding_rule_indices.push(index);
            }
        }

        (binding_rule_indices, triggered_rule_indices)
    }

    /// Return true when one rule has no call selector or matches one call selector.
    fn matches_call_selector(
        rule: &Rule,
        runtime_name: &str,
        runtime_labels: &BTreeMap<String, String>,
        worker_name: &str,
        worker_labels: &BTreeMap<String, String>,
        descriptor: Option<BindingDescriptor>,
        mode: ExecutionMode,
        engine: Option<BindingEngine>,
    ) -> bool {
        let Some(selector) = &rule.call else {
            return true;
        };

        let subject = CallSubject {
            runtime_name,
            runtime_labels,
            worker_name,
            worker_labels,
            binding: descriptor,
            mode,
            engine,
        };

        selector.matches(subject)
    }

    /// Reset all runtime state for the active policy revision.
    fn reset_runtime_state(&mut self) {
        self.total_calls_seen_by_worker = HashMap::new();
        self.rule_states = HashMap::new();
    }
}

impl RuleState {
    /// Evaluate one trigger event and return accepted fire count.
    fn accept_event(
        &mut self,
        trigger: &Trigger,
        total_calls_seen: u64,
        now_virtual_ns: u64,
        random: &Random,
    ) -> u64 {
        if !self.ensure_activated(trigger, total_calls_seen, now_virtual_ns, random) {
            return 0;
        }

        self.active_hits_seen = self.active_hits_seen.saturating_add(1);
        if self.update_expired(trigger, now_virtual_ns) {
            return 0;
        }

        if !self.matches_cadence(trigger) {
            return 0;
        }
        if !self.matches_cooldown(trigger, now_virtual_ns) {
            return 0;
        }
        if !Self::matches_probability(trigger.probability_ppm, random) {
            return 0;
        }

        let fire_count = self.accepted_fire_count(trigger);
        if fire_count == 0 {
            return 0;
        }

        self.record_fire(trigger, now_virtual_ns, fire_count);
        fire_count
    }

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
