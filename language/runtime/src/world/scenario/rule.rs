use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::random::Random;
use crate::world::Topology;
use crate::world::policy::{ActionSelector, Subject, SubjectSelector, TargetSelector};

use super::{
    Fault, FaultTarget, Lifetime, ProbabilityPpm, RuntimeEvent, Trigger,
    validate_fault_rule_compatibility,
};

/// Stable identifier for one scenario fault rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FaultRuleId(pub String);

impl FaultRuleId {
    /// Create one fault rule identifier from one string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// One scenario rule that injects one fault when its trigger accepts an event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaultRule {
    /// Stable fault rule identifier.
    pub id: FaultRuleId,
    /// Whether this rule is enabled.
    pub enabled: bool,
    /// Subject selector for this rule.
    pub subject: SubjectSelector,
    /// Action selector for this rule.
    pub action: ActionSelector,
    /// Target selector for this rule.
    pub target: TargetSelector,
    /// Trigger controls for this fault rule.
    pub trigger: Trigger,
    /// Fault injected by this rule.
    pub fault: Fault,
}

impl FaultRule {
    /// Create one enabled fault rule.
    pub fn new(id: impl Into<String>, fault: Fault, trigger: Trigger) -> Self {
        Self {
            id: FaultRuleId::new(id),
            enabled: true,
            subject: SubjectSelector::default(),
            action: ActionSelector::default(),
            target: TargetSelector::default(),
            trigger,
            fault,
        }
    }

    /// Attach one subject selector to this rule.
    pub fn subject(mut self, selector: SubjectSelector) -> Self {
        self.subject = selector;
        self
    }

    /// Attach one action selector to this rule.
    pub fn action(mut self, selector: ActionSelector) -> Self {
        self.action = selector;
        self
    }

    /// Attach one target selector to this rule.
    pub fn target(mut self, selector: TargetSelector) -> Self {
        self.target = selector;
        self
    }

    /// Set rule enabled state.
    pub fn enabled(mut self, is_enabled: bool) -> Self {
        self.enabled = is_enabled;
        self
    }

    /// Validate this rule against shape and catalog constraints.
    pub(crate) fn validate_with_topology(&self, topology: &Topology) -> RuntimeResult<()> {
        if matches!(self.fault.target, FaultTarget::Call {}) && self.action.is_empty() {
            return Err(Self::invalid_rule_error(format!(
                "runtime call fault rule {} requires an action selector",
                self.id.0
            )));
        }

        validate_fault_rule_compatibility(self, topology)
    }

    /// Return whether this rule accepts one runtime event.
    pub(crate) fn matches_event(&self, event: &RuntimeEvent, subject: Subject<'_>) -> bool {
        if self.trigger.on != event.kind() {
            return false;
        }

        self.subject.matches(subject)
            && self.action.matches(event.attempt())
            && self.target.matches_binding_attempt()
    }

    /// Return one invalid-rule error.
    fn invalid_rule_error(message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }
}

/// Runtime gate state for one scenario rule in one worker scope.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct FaultRuleState {
    /// Total accepted trigger firings for this state.
    pub fires: u64,
    /// Cooldown-until timestamp in monotonic nanoseconds.
    pub cooldown_until_ns: Option<u64>,
    /// Whether this rule has activated.
    pub is_activated: bool,
    /// Activation timestamp in monotonic nanoseconds.
    pub activated_at_ns: Option<u64>,
    /// Matching hits observed after activation.
    pub active_hits_seen: u64,
    /// Whether this rule has expired.
    pub is_expired: bool,
}

impl FaultRuleState {
    /// Evaluate one trigger event and return accepted fire count.
    pub(crate) fn accept_event(
        &mut self,
        trigger: &Trigger,
        total_calls_seen: u64,
        now_ns: u64,
        random: &Random,
    ) -> RuntimeResult<u64> {
        if !self.activate_if_ready(trigger, total_calls_seen, now_ns, random) {
            return Ok(0);
        }

        self.active_hits_seen = self.active_hits_seen.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "scenario active hit counter space exhausted".to_string(),
            }
            .boxed()
        })?;
        if self.expire_if_elapsed(trigger, now_ns) {
            return Ok(0);
        }

        if !self.matches_cadence(trigger) {
            return Ok(0);
        }
        if !self.matches_cooldown(trigger, now_ns) {
            return Ok(0);
        }
        if !Self::matches_probability(trigger.probability_ppm, random) {
            return Ok(0);
        }

        let burst = trigger.burst.unwrap_or(1) as u64;
        let remaining = trigger
            .max_occurrences
            .map(|max| {
                if self.fires >= max {
                    0
                } else {
                    max - self.fires
                }
            })
            .unwrap_or(burst);
        let fires = burst.min(remaining);

        self.fires = self.fires.checked_add(fires).ok_or_else(|| {
            RuntimeError::Internal {
                message: "scenario fire counter space exhausted".to_string(),
            }
            .boxed()
        })?;
        if let Some(cooldown_ns) = trigger.cooldown_ns {
            let cooldown_until_ns = now_ns.checked_add(cooldown_ns).ok_or_else(|| {
                RuntimeError::Internal {
                    message: "scenario cooldown timestamp space exhausted".to_string(),
                }
                .boxed()
            })?;
            self.cooldown_until_ns = Some(cooldown_until_ns);
        }

        Ok(fires)
    }

    /// Activate this state when the trigger gate accepts this event.
    fn activate_if_ready(
        &mut self,
        trigger: &Trigger,
        total_calls_seen: u64,
        now_ns: u64,
        random: &Random,
    ) -> bool {
        if self.is_activated {
            return true;
        }

        let is_reached = trigger
            .activation
            .is_none_or(|activation| activation.is_reached(total_calls_seen, now_ns));
        if !is_reached || !Self::matches_probability(trigger.activation_ppm, random) {
            return false;
        }

        self.is_activated = true;
        self.activated_at_ns = Some(now_ns);

        true
    }

    /// Expire this state when the trigger lifetime has elapsed.
    fn expire_if_elapsed(&mut self, trigger: &Trigger, now_ns: u64) -> bool {
        if self.is_expired {
            return true;
        }

        let is_expired = match trigger.lifetime {
            Some(Lifetime::ForDurationNs { duration_ns }) => self
                .activated_at_ns
                .is_some_and(|start| now_ns >= start && now_ns - start >= duration_ns),
            Some(Lifetime::ForCallCount { call_count }) => self.active_hits_seen > call_count,
            Some(Lifetime::UntilDisabled) | None => false,
        };

        self.is_expired = is_expired;

        is_expired
    }

    /// Return whether trigger cadence accepts this hit.
    fn matches_cadence(&self, trigger: &Trigger) -> bool {
        let skip_hits = trigger.skip_hits.unwrap_or(0);
        if self.active_hits_seen <= skip_hits {
            return false;
        }

        let interval_hits = trigger.interval_hits.unwrap_or(1).max(1);

        (self.active_hits_seen - skip_hits - 1).is_multiple_of(interval_hits)
    }

    /// Return whether cooldown accepts this event.
    fn matches_cooldown(&self, trigger: &Trigger, now_ns: u64) -> bool {
        if trigger.cooldown_ns.is_none() {
            return true;
        }

        self.cooldown_until_ns
            .is_none_or(|cooldown_until| now_ns >= cooldown_until)
    }

    /// Return whether one optional probability accepts this event.
    fn matches_probability(probability: Option<ProbabilityPpm>, random: &Random) -> bool {
        let Some(probability) = probability else {
            return true;
        };

        if probability.0 >= 1_000_000 {
            return true;
        }

        random.next_u64() % 1_000_000 < probability.0 as u64
    }
}
