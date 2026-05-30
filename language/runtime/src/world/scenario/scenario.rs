use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::WorkerId;
use crate::runtime::random::Random;
use crate::world::Topology;
use crate::world::policy::Subject;

use super::{FaultRule, FaultRuleId, FaultRuleState, RuntimeEvent, TriggeredFault};

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
    /// Total matching call events seen per worker.
    total_calls_seen_by_worker: HashMap<WorkerId, u64>,
    /// Runtime fault rule state by rule id and worker id.
    rule_states: HashMap<FaultRuleId, HashMap<WorkerId, FaultRuleState>>,
}

impl Scenario {
    /// Create one active scenario with empty runtime state.
    pub fn new(id: ScenarioId, rules: Vec<FaultRule>) -> Self {
        Self {
            id,
            name: None,
            enabled: true,
            labels: BTreeMap::new(),
            rules,
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
    pub(crate) fn validate_with_topology(&self, topology: &Topology) -> RuntimeResult<()> {
        self.validate_unique_rule_ids()?;

        for rule in &self.rules {
            rule.validate_with_topology(topology)?;
        }

        Ok(())
    }

    /// Add one rule into this scenario.
    pub(crate) fn add_rule(&mut self, rule: FaultRule, topology: &Topology) -> RuntimeResult<()> {
        if self.has_rule_id(&rule.id) {
            return Err(Self::invalid_scenario_error(format!(
                "runtime scenario requires unique rule ids: {}",
                rule.id.0
            )));
        }

        rule.validate_with_topology(topology)?;
        self.rules.push(rule);

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
        topology: &Topology,
    ) -> RuntimeResult<()> {
        if rule.id != *rule_id {
            return Err(Self::invalid_scenario_error(format!(
                "runtime scenario replace requires replacement id to match: {}",
                rule_id.0
            )));
        }

        rule.validate_with_topology(topology)?;
        let Some(existing_rule) = self.rules.iter_mut().find(|rule| rule.id == *rule_id) else {
            return Err(Self::unknown_rule_error(rule_id));
        };

        *existing_rule = rule;
        self.rule_states.remove(rule_id);

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

        Ok(())
    }

    /// Decide faults for one event.
    pub(crate) fn decide_faults(
        &mut self,
        event: &RuntimeEvent,
        subject: Subject<'_>,
        random: &Random,
    ) -> RuntimeResult<Vec<TriggeredFault>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        let event_worker_id = event.worker_id();
        let total_calls_seen = self.record_call_event(event_worker_id, event)?;
        let mut faults = Vec::new();

        for rule_index in 0..self.rules.len() {
            let rule = &self.rules[rule_index];
            if !rule.enabled {
                continue;
            }
            if !rule.matches_event(event, subject) {
                continue;
            }

            let rule_id = rule.id.clone();
            let fault = rule.fault.clone();
            let fire_count = self.accept_rule_event(rule_index, event, total_calls_seen, random)?;
            if fire_count == 0 {
                continue;
            }

            faults.push(TriggeredFault {
                rule_id,
                event: event.kind(),
                worker_id: event_worker_id,
                call_id: event.call_id(),
                fault,
            });
        }

        Ok(faults)
    }

    /// Accept one matching event against one enabled rule state.
    fn accept_rule_event(
        &mut self,
        rule_index: usize,
        event: &RuntimeEvent,
        total_calls_seen: u64,
        random: &Random,
    ) -> RuntimeResult<u64> {
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
    fn record_call_event(
        &mut self,
        worker_id: WorkerId,
        event: &RuntimeEvent,
    ) -> RuntimeResult<u64> {
        let calls_seen = self
            .total_calls_seen_by_worker
            .entry(worker_id)
            .or_insert(0);

        if event.counts_as_call_event() {
            *calls_seen = calls_seen.checked_add(1).ok_or_else(|| {
                RuntimeError::Internal {
                    message: "scenario call counter space exhausted".to_string(),
                }
                .boxed()
            })?;
        }

        Ok(*calls_seen)
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self::new(ScenarioId::new(0), Vec::new())
    }
}
