use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{CheckEvent, CheckState, Dependency};
use crate::{CompilerError, CompilerResult};

/// One decided node meaning.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Decision {
    /// Resolved lexical name.
    Name(dir::NameResolution),
    /// Resolved explicit generic application.
    Instantiation(dir::InstantiationResolution),
    /// Resolved contextual receiver.
    Receiver(dir::ReceiverResolution),
    /// Resolved member access.
    Member(dir::MemberResolution),
    /// Resolved call, including builtin operator applications.
    Call(dir::CallResolution),
    /// Resolved writable place expression.
    Place(dir::PlaceResolution),
    /// Resolved runtime predicate expression.
    Guard(dir::GuardResolution),
    /// Resolved construct expression.
    Construct(dir::ConstructResolution),
    /// Resolved pattern meaning.
    Pattern(dir::PatternResolution),
    /// Resolved assignment pattern meaning.
    AssignPattern(dir::AssignPatternResolution),
    /// Rejected node with reported diagnostics.
    Rejected,
}

impl CheckState<'_> {
    /// Record one node decision and wake its waiters.
    pub(in crate::check) fn record_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
    ) -> CompilerResult<()> {
        self.decisions
            .decide(node, decision, self.node_message(node))?;
        self.record_event(CheckEvent::NodeDecided { node });

        // wake tasks parked on the decision
        for waiter in self.solver.wake(Dependency::Decision(node)) {
            self.queue_task(waiter);
        }

        Ok(())
    }

    /// Return one selected node decision.
    pub(in crate::check) fn decision(&self, node: dir::GlobalNodeIdAny) -> Option<&Decision> {
        self.decisions.get(node)
    }
}

/// Node decisions for one checked component.
#[derive(Debug)]
pub(in crate::check) struct DecisionTable {
    /// Decisions keyed by source node.
    decisions: IndexMap<dir::GlobalNodeIdAny, Decision>,
}

impl DecisionTable {
    /// Create an empty decision table.
    pub(in crate::check) fn new() -> Self {
        Self {
            decisions: IndexMap::new(),
        }
    }

    /// Return one node decision when decided.
    pub(in crate::check) fn get(&self, node: dir::GlobalNodeIdAny) -> Option<&Decision> {
        self.decisions.get(&node)
    }

    /// Record one node decision.
    /// Re-derived matching decisions collapse, conflicting decisions error.
    pub(in crate::check) fn decide(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
        message: String,
    ) -> CompilerResult<()> {
        match self.decisions.get(&node) {
            // collapse identical re-derivations without waking anyone
            Some(previous) if previous == &decision => Ok(()),
            // a node must decide exactly once
            Some(_) => Err(CompilerError::Internal {
                message: format!("check node {message} was decided twice"),
            }),
            // first decision without waiters
            None => {
                self.decisions.insert(node, decision);

                Ok(())
            }
        }
    }

    /// Return the number of decided nodes.
    pub(in crate::check) fn count(&self) -> usize {
        self.decisions.len()
    }

    /// Take decided nodes owned by one module.
    pub(in crate::check) fn take_module(
        &mut self,
        module: ModuleId,
    ) -> Vec<(dir::GlobalNodeIdAny, Decision)> {
        let mut decisions = Vec::new();
        let nodes = self
            .decisions
            .iter()
            .filter_map(|(node, _)| (node.module_id == module).then_some(*node))
            .collect::<Vec<_>>();

        for node in nodes {
            if let Some(decision) = self.decisions.swap_remove(&node) {
                decisions.push((node, decision));
            }
        }

        decisions
    }
}
