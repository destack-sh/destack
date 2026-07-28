use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckEvent, CheckState};
use crate::{CompilerError, CompilerResult};

/// One decided node meaning, committed into the module's resolution segment.
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
    /// Resolved operator application.
    Operator(dir::OperatorResolution),
    /// Resolved call.
    Call(dir::CallResolution),
    /// Resolved subscript access.
    Subscript(dir::SubscriptResolution),
    /// Resolved assignment target.
    Assignment(dir::AssignmentResolution),
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

impl Decision {
    /// Return this decision's kind.
    fn kind(&self) -> DecisionKind {
        match self {
            Self::Name(_) => DecisionKind::Name,
            Self::Instantiation(_) => DecisionKind::Instantiation,
            Self::Receiver(_) => DecisionKind::Receiver,
            Self::Member(_) => DecisionKind::Member,
            Self::Operator(_) => DecisionKind::Operator,
            Self::Call(_) => DecisionKind::Call,
            Self::Subscript(_) => DecisionKind::Subscript,
            Self::Assignment(_) => DecisionKind::Assignment,
            Self::Guard(_) => DecisionKind::Guard,
            Self::Construct(_) => DecisionKind::Construct,
            Self::Pattern(_) => DecisionKind::Pattern,
            Self::AssignPattern(_) => DecisionKind::AssignPattern,
            Self::Rejected => DecisionKind::Rejected,
        }
    }
}

/// The meaning kind one node decided to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum DecisionKind {
    /// Resolved lexical name.
    Name,
    /// Resolved explicit generic application.
    Instantiation,
    /// Resolved contextual receiver.
    Receiver,
    /// Resolved member access.
    Member,
    /// Resolved operator application.
    Operator,
    /// Resolved call.
    Call,
    /// Resolved subscript access.
    Subscript,
    /// Resolved assignment target.
    Assignment,
    /// Resolved runtime predicate expression.
    Guard,
    /// Resolved construct expression.
    Construct,
    /// Resolved pattern meaning.
    Pattern,
    /// Resolved assignment pattern meaning.
    AssignPattern,
    /// Rejected node with reported diagnostics.
    Rejected,
}

/// Decided node kinds for one checked component.
///
/// Resolution payloads live in each module's resolution segment; this table
/// only enforces the decide-once invariant and records rejections.
#[derive(Debug)]
pub(in crate::check) struct DecisionTable {
    /// Decided kinds keyed by source node, in decide order.
    kinds: FxIndexMap<dir::GlobalNodeIdAny, DecisionKind>,
}

impl DecisionTable {
    /// Create an empty decision table.
    pub(in crate::check) fn new() -> Self {
        Self {
            kinds: FxIndexMap::default(),
        }
    }

    /// Return one node's decided kind.
    pub(in crate::check) fn kind(&self, node: dir::GlobalNodeIdAny) -> Option<DecisionKind> {
        self.kinds.get(&node).copied()
    }

    /// Return the number of decided nodes.
    pub(in crate::check) fn count(&self) -> usize {
        self.kinds.len()
    }

    /// Truncate decided kinds back to one probe mark, newest first.
    pub(in crate::check) fn truncate_to(&mut self, count: usize) {
        while self.kinds.len() > count {
            self.kinds.pop();
        }
    }
}

impl CheckState<'_> {
    /// Commit one node decision into its module's resolution segment.
    pub(in crate::check) fn commit_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
    ) -> CompilerResult<()> {
        // collapse identical re-derivations, reject conflicting ones
        if let Some(kind) = self.decisions.kind(node) {
            if kind == decision.kind() && self.decision_matches(node, &decision) {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "check node {} was decided twice: previous = {kind:?}, new = {decision:?}",
                    self.node_label(node),
                ),
            });
        }
        self.decisions.kinds.insert(node, decision.kind());

        // file the payload into its final segment home
        let resolutions = &mut self.module_mut(node.module_id).resolutions;
        match decision {
            Decision::Name(resolution) => resolutions.set_name_resolution(node, resolution),
            Decision::Instantiation(resolution) => {
                resolutions.set_instantiation_resolution(node, resolution)
            }
            Decision::Receiver(resolution) => resolutions.set_receiver_resolution(node, resolution),
            Decision::Member(resolution) => resolutions.set_member_resolution(node, resolution),
            Decision::Operator(resolution) => resolutions.set_operator_resolution(node, resolution),
            Decision::Call(resolution) => resolutions.set_call_resolution(node, resolution),
            Decision::Subscript(resolution) => {
                resolutions.set_subscript_resolution(node, resolution)
            }
            Decision::Assignment(resolution) => {
                resolutions.set_assignment_resolution(node, resolution)
            }
            Decision::Guard(resolution) => resolutions.set_guard_resolution(node, resolution),
            Decision::Construct(resolution) => {
                resolutions.set_construct_resolution(node, resolution)
            }
            Decision::Pattern(resolution) => resolutions.set_pattern_resolution(node, resolution),
            Decision::AssignPattern(resolution) => {
                resolutions.set_assign_pattern_resolution(node, resolution)
            }
            Decision::Rejected => {}
        }

        self.record_event(CheckEvent::NodeDecided { node });

        Ok(())
    }

    /// Return whether one new decision matches the committed resolution.
    fn decision_matches(&self, node: dir::GlobalNodeIdAny, decision: &Decision) -> bool {
        let resolutions = &self.module(node.module_id).resolutions;

        match decision {
            Decision::Name(resolution) => resolutions.name_resolution(node) == Some(resolution),
            Decision::Instantiation(resolution) => {
                resolutions.instantiation_resolution(node) == Some(resolution)
            }
            Decision::Receiver(resolution) => {
                resolutions.receiver_resolution(node) == Some(resolution)
            }
            Decision::Member(resolution) => resolutions.member_resolution(node) == Some(resolution),
            Decision::Operator(resolution) => {
                resolutions.operator_resolution(node) == Some(resolution)
            }
            Decision::Call(resolution) => resolutions.call_resolution(node) == Some(resolution),
            Decision::Subscript(resolution) => {
                resolutions.subscript_resolution(node) == Some(resolution)
            }
            Decision::Assignment(resolution) => {
                resolutions.assignment_resolution(node) == Some(resolution)
            }
            Decision::Guard(resolution) => resolutions.guard_resolution(node) == Some(resolution),
            Decision::Construct(resolution) => {
                resolutions.construct_resolution(node) == Some(resolution)
            }
            Decision::Pattern(resolution) => {
                resolutions.pattern_resolution(node) == Some(resolution)
            }
            Decision::AssignPattern(resolution) => {
                resolutions.assign_pattern_resolution(node) == Some(resolution)
            }
            Decision::Rejected => true,
        }
    }

    /// Return one node's decided kind.
    pub(in crate::check) fn decision_kind(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<DecisionKind> {
        self.decisions.kind(node)
    }

    /// Return one node's committed resolutions.
    pub(in crate::check) fn resolutions(&self, module: ModuleId) -> &dir::ResolutionSegment {
        &self.module(module).resolutions
    }
}
