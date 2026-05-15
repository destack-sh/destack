use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::WorkerId;
use crate::runtime::random::Random;
use crate::world::policy::Subject;

use super::{FaultKindCatalog, FaultRule, FaultRuleId, FaultRuleState, HookEvent, TriggeredFault};

/// Stable identifier for one scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ScenarioId(pub u64);

impl ScenarioId {
    /// Create one scenario identifier from one raw value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Active scenario with its runtime trigger state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario {
    /// Stable scenario identifier.
    pub id: ScenarioId,
    /// Optional human-readable scenario name.
    pub name: Option<String>,
    /// Whether this scenario is enabled.
    pub enabled: bool,
    /// Scenario labels used by control and observation surfaces.
    pub labels: BTreeMap<String, String>,
    /// Ordered fault rules.
    pub rules: Vec<FaultRule>,
    /// Enabled fault rule indices for this scenario revision.
    enabled_rule_indices: Vec<usize>,
    /// Total matching call events seen per worker.
    total_calls_seen_by_worker: HashMap<WorkerId, u64>,
    /// Runtime fault rule state by rule id and worker id.
    rule_states: HashMap<FaultRuleId, HashMap<WorkerId, FaultRuleState>>,
}

impl Scenario {
    /// Create one active scenario with empty runtime state.
    pub fn new(id: ScenarioId, rules: Vec<FaultRule>) -> Self {
        let enabled_rule_indices = Self::enabled_rule_indices(&rules);

        Self {
            id,
            name: None,
            enabled: true,
            labels: BTreeMap::new(),
            rules,
            enabled_rule_indices,
            total_calls_seen_by_worker: HashMap::new(),
            rule_states: HashMap::new(),
        }
    }

    /// Attach one optional name.
    pub fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Set scenario enabled state.
    pub fn enabled(mut self, is_enabled: bool) -> Self {
        self.enabled = is_enabled;
        self
    }

    /// Attach scenario labels.
    pub fn labels(mut self, labels: BTreeMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    /// Validate all scenario invariants against one topology kind catalog.
    pub(crate) fn validate_with_kind_catalog(
        &self,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        self.validate_unique_rule_ids()?;

        for rule in &self.rules {
            rule.validate_with_kind_catalog(kind_catalog)?;
        }

        Ok(())
    }

    /// Add one rule into this scenario.
    pub(crate) fn add_rule(
        &mut self,
        rule: FaultRule,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        if self.has_rule_id(&rule.id) {
            return Err(Self::invalid_scenario_error(format!(
                "runtime scenario requires unique rule ids: {}",
                rule.id.0
            )));
        }

        rule.validate_with_kind_catalog(kind_catalog)?;
        self.rules.push(rule);
        self.rebuild_rule_cache();

        Ok(())
    }

    /// Remove one rule from this scenario.
    pub(crate) fn remove_rule(&mut self, rule_id: &FaultRuleId) -> RuntimeResult<()> {
        let before_len = self.rules.len();
        self.rules.retain(|rule| rule.id != *rule_id);
        if self.rules.len() == before_len {
            return Err(Self::unknown_rule_error(rule_id));
        }

        self.rule_states.remove(rule_id);
        self.rebuild_rule_cache();

        Ok(())
    }

    /// Enable one rule in this scenario.
    pub(crate) fn enable_rule(&mut self, rule_id: &FaultRuleId) -> RuntimeResult<()> {
        self.set_rule_enabled(rule_id, true)
    }

    /// Disable one rule in this scenario.
    pub(crate) fn disable_rule(&mut self, rule_id: &FaultRuleId) -> RuntimeResult<()> {
        self.set_rule_enabled(rule_id, false)
    }

    /// Replace one rule in this scenario.
    pub(crate) fn replace_rule(
        &mut self,
        rule_id: &FaultRuleId,
        rule: FaultRule,
        kind_catalog: &impl FaultKindCatalog,
    ) -> RuntimeResult<()> {
        if rule.id != *rule_id {
            return Err(Self::invalid_scenario_error(format!(
                "runtime scenario replace requires replacement id to match: {}",
                rule_id.0
            )));
        }

        rule.validate_with_kind_catalog(kind_catalog)?;
        let Some(existing_rule) = self.rules.iter_mut().find(|rule| rule.id == *rule_id) else {
            return Err(Self::unknown_rule_error(rule_id));
        };

        *existing_rule = rule;
        self.rule_states.remove(rule_id);
        self.rebuild_rule_cache();

        Ok(())
    }

    /// Return one invalid-scenario error.
    fn invalid_scenario_error(message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }

    /// Return one unknown-rule error.
    fn unknown_rule_error(rule_id: &FaultRuleId) -> Box<RuntimeError> {
        Self::invalid_scenario_error(format!(
            "runtime scenario mutation requires one known rule id: {}",
            rule_id.0
        ))
    }

    /// Validate that this scenario contains unique rule ids.
    fn validate_unique_rule_ids(&self) -> RuntimeResult<()> {
        let mut rule_ids = HashSet::new();
        for rule in &self.rules {
            if rule_ids.insert(rule.id.clone()) {
                continue;
            }

            return Err(Self::invalid_scenario_error(format!(
                "runtime scenario rules require unique fault rule ids: {}",
                rule.id.0
            )));
        }

        Ok(())
    }

    /// Return true when one rule id exists in this scenario.
    fn has_rule_id(&self, rule_id: &FaultRuleId) -> bool {
        self.rules.iter().any(|rule| rule.id == *rule_id)
    }

    /// Set one rule enabled state.
    fn set_rule_enabled(&mut self, rule_id: &FaultRuleId, is_enabled: bool) -> RuntimeResult<()> {
        let Some(rule) = self.rules.iter_mut().find(|rule| rule.id == *rule_id) else {
            return Err(Self::unknown_rule_error(rule_id));
        };

        rule.enabled = is_enabled;
        self.rebuild_rule_cache();

        Ok(())
    }

    /// Rebuild cached rule state after one scenario spec mutation.
    fn rebuild_rule_cache(&mut self) {
        self.enabled_rule_indices = Self::enabled_rule_indices(&self.rules);
    }

    /// Decide faults for one hook event.
    pub(crate) fn decide_faults(
        &mut self,
        event: &HookEvent,
        subject: Subject<'_>,
        random: &Random,
    ) -> Vec<TriggeredFault> {
        if !self.enabled {
            return Vec::new();
        }

        let event_worker_id = event.worker_id();
        let total_calls_seen = self.record_call_event(event_worker_id, event);
        let mut faults = Vec::new();

        for position in 0..self.enabled_rule_indices.len() {
            let rule_index = self.enabled_rule_indices[position];
            let rule = &self.rules[rule_index];
            if !rule.matches_event(event, subject) {
                continue;
            }

            let rule_id = rule.id.clone();
            let fault = rule.fault.clone();
            let fire_count = self.accept_rule_event(rule_index, event, total_calls_seen, random);
            if fire_count == 0 {
                continue;
            }

            faults.push(TriggeredFault {
                rule_id,
                hook: event.hook(),
                worker_id: event_worker_id,
                call_id: event.call_id(),
                fault,
            });
        }

        faults
    }

    /// Accept one matching event against one enabled rule state.
    fn accept_rule_event(
        &mut self,
        rule_index: usize,
        event: &HookEvent,
        total_calls_seen: u64,
        random: &Random,
    ) -> u64 {
        let rule = &self.rules[rule_index];
        let rule_state = self
            .rule_states
            .entry(rule.id.clone())
            .or_default()
            .entry(event.worker_id())
            .or_default();

        rule_state.accept_event(&rule.trigger, total_calls_seen, event.time_ns(), random)
    }

    /// Record one call-counting event and return the total call count.
    fn record_call_event(&mut self, worker_id: WorkerId, event: &HookEvent) -> u64 {
        let calls_seen = self
            .total_calls_seen_by_worker
            .entry(worker_id)
            .or_insert(0);

        if event.counts_as_call_event() {
            *calls_seen = calls_seen.saturating_add(1);
        }

        *calls_seen
    }

    /// Return enabled fault rule indices.
    fn enabled_rule_indices(rules: &[FaultRule]) -> Vec<usize> {
        rules
            .iter()
            .enumerate()
            .filter_map(|(index, rule)| rule.enabled.then_some(index))
            .collect()
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self::new(ScenarioId::new(0), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use destack_workspace::ExecutionMode;

    use crate::host::binding::{
        BindingAffinity, BindingDescriptor, BindingDeterminism, BindingProvider, BindingReplayKind,
        BindingReplayPayload,
    };
    use crate::runtime::WorkerId;
    use crate::runtime::random::Random;
    use crate::world::policy::Subject;
    use crate::world::scenario::{
        ActivationWindow, Fault, FaultRule, FaultTarget, FaultType, Hook, HookEvent, Lifetime,
        Scenario, ScenarioCallId, ScenarioId, Trigger,
    };

    const TEST_RUNTIME_NAME: &str = "test-runtime";
    const TEST_WORKER_NAME: &str = "test-worker";

    /// Ensures call-count activation gates initial fault injection.
    #[test]
    fn test_decide_faults_respects_call_count_activation() {
        let trigger = Trigger {
            on: Hook::BindingBefore,
            activation: Some(ActivationWindow::AfterCallCount { call_count: 2 }),
            lifetime: None,
            activation_ppm: None,
            probability_ppm: None,
            max_occurrences: Some(1),
            cooldown_ns: None,
            burst: None,
            interval_hits: None,
            skip_hits: None,
        };
        let mut scenario =
            Scenario::new(ScenarioId::new(1), vec![fault_rule("activation", trigger)]);
        let random = Random::new(7);
        let runtime_labels = BTreeMap::new();
        let worker_labels = BTreeMap::new();

        let first_faults = scenario.decide_faults(
            &binding_before_event(10, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let second_faults = scenario.decide_faults(
            &binding_before_event(10, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );

        assert!(first_faults.is_empty());
        assert_eq!(second_faults.len(), 1);
    }

    /// Ensures call-count activation is scoped per worker.
    #[test]
    fn test_decide_faults_scopes_call_count_activation_by_worker() {
        let trigger = Trigger {
            on: Hook::BindingBefore,
            activation: Some(ActivationWindow::AfterCallCount { call_count: 2 }),
            lifetime: None,
            activation_ppm: None,
            probability_ppm: None,
            max_occurrences: Some(1),
            cooldown_ns: None,
            burst: None,
            interval_hits: None,
            skip_hits: None,
        };
        let mut scenario = Scenario::new(
            ScenarioId::new(1),
            vec![fault_rule("worker-activation", trigger)],
        );
        let random = Random::new(11);
        let runtime_labels = BTreeMap::new();
        let worker_labels = BTreeMap::new();

        let worker_one_first = scenario.decide_faults(
            &binding_before_event(1, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let worker_two_first = scenario.decide_faults(
            &binding_before_event(2, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let worker_two_second = scenario.decide_faults(
            &binding_before_event(2, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );

        assert!(worker_one_first.is_empty());
        assert!(worker_two_first.is_empty());
        assert_eq!(worker_two_second.len(), 1);
    }

    /// Ensures cadence and cooldown gate accepted fault injection.
    #[test]
    fn test_decide_faults_respects_cadence_and_cooldown() {
        let trigger = Trigger {
            on: Hook::SchedulerDequeue,
            activation: None,
            lifetime: None,
            activation_ppm: None,
            probability_ppm: None,
            max_occurrences: None,
            cooldown_ns: Some(10),
            burst: None,
            interval_hits: Some(2),
            skip_hits: Some(1),
        };
        let mut scenario = Scenario::new(ScenarioId::new(1), vec![fault_rule("cadence", trigger)]);
        let random = Random::new(17);
        let runtime_labels = BTreeMap::new();
        let worker_labels = BTreeMap::new();

        let first = scenario.decide_faults(
            &scheduler_dequeue_event(2, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let second = scenario.decide_faults(
            &scheduler_dequeue_event(2, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let third = scenario.decide_faults(
            &scheduler_dequeue_event(2, 5),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let fourth = scenario.decide_faults(
            &scheduler_dequeue_event(2, 10),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let total_faults = first.len() + second.len() + third.len() + fourth.len();

        assert_eq!(total_faults, 2);
    }

    /// Ensures call-count lifetime expires fault injection.
    #[test]
    fn test_decide_faults_respects_call_count_lifetime() {
        let trigger = Trigger {
            on: Hook::SchedulerDequeue,
            activation: None,
            lifetime: Some(Lifetime::ForCallCount { call_count: 2 }),
            activation_ppm: None,
            probability_ppm: None,
            max_occurrences: None,
            cooldown_ns: None,
            burst: None,
            interval_hits: None,
            skip_hits: None,
        };
        let mut scenario = Scenario::new(ScenarioId::new(1), vec![fault_rule("lifetime", trigger)]);
        let random = Random::new(23);
        let runtime_labels = BTreeMap::new();
        let worker_labels = BTreeMap::new();

        let first = scenario.decide_faults(
            &scheduler_dequeue_event(3, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let second = scenario.decide_faults(
            &scheduler_dequeue_event(3, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let third = scenario.decide_faults(
            &scheduler_dequeue_event(3, 0),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let total_faults = first.len() + second.len() + third.len();

        assert_eq!(total_faults, 2);
    }

    /// Ensures injected faults carry execution context.
    #[test]
    fn test_decide_faults_emits_fault_context() {
        let trigger = Trigger {
            on: Hook::BindingBefore,
            activation: None,
            lifetime: None,
            activation_ppm: None,
            probability_ppm: None,
            max_occurrences: Some(1),
            cooldown_ns: None,
            burst: None,
            interval_hits: None,
            skip_hits: None,
        };
        let mut scenario = Scenario::new(ScenarioId::new(1), vec![fault_rule("context", trigger)]);
        let random = Random::new(41);
        let runtime_labels = BTreeMap::new();
        let worker_labels = BTreeMap::new();

        let faults = scenario.decide_faults(
            &binding_before_event(99, 1234),
            test_subject(&runtime_labels, &worker_labels),
            &random,
        );
        let fault = &faults[0];

        assert_eq!(faults.len(), 1);
        assert_eq!(fault.rule_id.0, "test.context");
        assert_eq!(fault.hook, Hook::BindingBefore);
        assert_eq!(fault.worker_id, WorkerId(99));
        assert!(fault.call_id.is_some());
        assert!(matches!(fault.fault.target, FaultTarget::Call {}));
    }

    fn test_subject<'a>(
        runtime_labels: &'a BTreeMap<String, String>,
        worker_labels: &'a BTreeMap<String, String>,
    ) -> Subject<'a> {
        Subject::new(
            TEST_RUNTIME_NAME,
            runtime_labels,
            TEST_WORKER_NAME,
            worker_labels,
            ExecutionMode::Fast,
        )
    }

    fn fault_rule(id_suffix: &str, trigger: Trigger) -> FaultRule {
        FaultRule::new(
            format!("test.{id_suffix}"),
            Fault {
                target: FaultTarget::Call {},
                fault_type: FaultType::Error {
                    code: "EFAULT".to_string(),
                },
            },
            trigger,
        )
    }

    fn binding_before_event(worker_id: u64, time_ns: u64) -> HookEvent {
        HookEvent::BindingBefore {
            worker_id: WorkerId(worker_id),
            call_id: ScenarioCallId(1),
            descriptor: binding_descriptor(),
            time_ns,
        }
    }

    fn scheduler_dequeue_event(worker_id: u64, time_ns: u64) -> HookEvent {
        HookEvent::SchedulerDequeue {
            worker_id: WorkerId(worker_id),
            time_ns,
        }
    }

    fn binding_descriptor() -> BindingDescriptor {
        BindingDescriptor::new(
            "destack.test",
            "()",
            BindingDeterminism::Pure,
            BindingReplayKind::BindingCall,
            BindingReplayPayload::Results,
            &[],
            BindingProvider::Runtime,
            BindingAffinity::None,
        )
    }
}
