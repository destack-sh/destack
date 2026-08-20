use destack_serde::Reflect;
use std::hash::Hash;
use std::slice;
use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    AccessResolution, ArgumentBinding, AssignPatternDecision, AssignmentDecision, BindingUse, Call,
    CallDecision, ConstructDecision, Expression, FunctionDecision, GlobalNodeId, GlobalNodeIdAny,
    GlobalSymbolId, GlobalTypeId, GuardDecision, MemberAccess, MemberDecision, OperationResolution,
    OperatorDecision, Pattern, PatternDecision, PlaceResolution, ReceiverDecision, SegmentView,
    Selection, SubscriptDecision, SubscriptTarget, TreeDecision, TypeFold, WalkSelections,
};

/// The one decision inference made for a DIR node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold, WalkSelections)]
pub enum Decision {
    /// Resolved contextual receiver.
    Receiver(ReceiverDecision),
    /// Resolved member access.
    Member(MemberDecision),
    /// Resolved function value.
    Function(FunctionDecision),
    /// Resolved control transfer target.
    Transfer(GlobalNodeId<Expression>),
    /// Resolved try residual transfer.
    Residual(ResidualDecision),
    /// Resolved pattern coverage proof.
    Coverage(CoverageDecision),
    /// Resolved operator application.
    Operator(OperatorDecision),
    /// Resolved call.
    Call(CallDecision),
    /// Resolved subscript access.
    Subscript(SubscriptDecision),
    /// Resolved assignment target.
    Assignment(Box<AssignmentDecision>),
    /// Resolved runtime predicate expression.
    Guard(GuardDecision),
    /// Resolved construct expression.
    Construct(ConstructDecision),
    /// Resolved tree literal.
    Tree(TreeDecision),
    /// Resolved pattern meaning.
    Pattern(PatternDecision),
    /// Resolved assignment pattern meaning.
    AssignPattern(AssignPatternDecision),
    /// Rejected node with the one call it reached for.
    Attempted(Box<Call>),
    /// Rejected node with reported diagnostics.
    Rejected,
    /// Poisoned node with an already-reported error.
    Poisoned,
}

/// One decided try residual transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ResidualDecision {
    /// The transfer target.
    pub target: ResidualTarget,
    /// The transferred residual type.
    pub residual: GlobalTypeId,
}

/// One residual transfer target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ResidualTarget {
    /// The enclosing try expression.
    Try(GlobalNodeId<Expression>),
    /// The enclosing callable's return.
    Callable,
}

impl WalkSelections for ResidualDecision {
    fn for_each_selection(&self, _visit: &mut dyn FnMut(&Selection)) {}
}

impl TypeFold for ResidualDecision {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.residual.map_types(map)
    }
}

/// One decided pattern coverage proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CoverageDecision {
    /// Whether the unguarded arms cover the scrutinee.
    pub is_exhaustive: bool,
    /// Whether every arm pair accepts disjoint values.
    pub is_disjoint: bool,
    /// Arms covered entirely by their preceding arms.
    pub redundant: Vec<GlobalNodeId<Pattern>>,
}

impl WalkSelections for CoverageDecision {
    fn for_each_selection(&self, _visit: &mut dyn FnMut(&Selection)) {}
}

impl TypeFold for CoverageDecision {
    fn map_types<E>(
        &mut self,
        _map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        Ok(())
    }
}

impl Decision {
    /// Return the binding uses represented at this decision's node.
    pub fn binding_uses(&self) -> Vec<(GlobalSymbolId, BindingUse)> {
        let mut uses = Vec::new();

        match self {
            // collect member reads
            Self::Member(member) => uses.extend(
                member
                    .target_symbols()
                    .into_iter()
                    .map(|symbol| (symbol, BindingUse::READ)),
            ),
            // collect assignment reads and writes
            Self::Assignment(assignment) => {
                // collect reads performed before the write
                if let Some(read) = &assignment.read {
                    uses.extend(
                        read.target_symbols()
                            .into_iter()
                            .map(|symbol| (symbol, BindingUse::READ)),
                    );
                }

                // collect writes
                uses.extend(
                    assignment
                        .write
                        .target_symbols()
                        .into_iter()
                        .map(|symbol| (symbol, BindingUse::WRITTEN)),
                );
            }
            // no binding use at this node
            Self::Receiver(_)
            | Self::Function(_)
            | Self::Transfer(_)
            | Self::Residual(_)
            | Self::Coverage(_)
            | Self::Operator(_)
            | Self::Call(_)
            | Self::Subscript(_)
            | Self::Guard(_)
            | Self::Construct(_)
            | Self::Tree(_)
            | Self::Pattern(_)
            | Self::AssignPattern(_)
            | Self::Attempted(_)
            | Self::Rejected
            | Self::Poisoned => {}
        }

        uses
    }

    /// Return the single static member access selected at this node.
    pub fn member_access(&self) -> Option<&MemberAccess> {
        match self {
            Self::Member(OperationResolution::One(access)) => Some(access),
            Self::Subscript(OperationResolution::One(subscript)) => match &subscript.target {
                SubscriptTarget::Member(access) => Some(access),
                SubscriptTarget::Call(_) | SubscriptTarget::Index(_) => None,
            },
            _ => None,
        }
    }
}

/// Cumulative inference decisions for one DIR module.
#[derive(Debug, Clone)]
pub struct DecisionTable<'a> {
    /// The module id of the decision table.
    pub module_id: ModuleId,
    /// The ordered decision table segments.
    segments: SegmentView<'a, DecisionSegment>,
}

impl DecisionTable<'static> {
    /// Create a decision table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<DecisionSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a decision table from one segment.
    pub fn from_segment(segment: Arc<DecisionSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> DecisionTable<'a> {
    /// Create a decision table from a segment view.
    pub fn from_view(segments: SegmentView<'a, DecisionSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("decision table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "decision table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a decision table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b DecisionSegment) -> DecisionTable<'b> {
        DecisionTable::from_view(self.segments.with_tail(tail))
    }

    /// Get the decision made for a node.
    pub fn decision(&self, node_id: GlobalNodeIdAny) -> Option<&Decision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.decision(node_id))
    }

    /// Get the control transfer target decided for a node.
    pub fn transfer_decision(&self, node_id: GlobalNodeIdAny) -> Option<GlobalNodeId<Expression>> {
        match self.decision(node_id) {
            Some(Decision::Transfer(target)) => Some(*target),
            _ => None,
        }
    }

    /// Iterate all control transfer target decisions.
    pub fn transfer_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, GlobalNodeId<Expression>)> + '_ {
        self.segments.iter().flat_map(|segment| {
            segment
                .decision_entries()
                .filter_map(|(node_id, decision)| match decision {
                    Decision::Transfer(target) => Some((node_id, *target)),
                    _ => None,
                })
        })
    }

    /// Get the receiver decision for a node.
    pub fn receiver_decision(&self, node_id: GlobalNodeIdAny) -> Option<&ReceiverDecision> {
        match self.decision(node_id) {
            Some(Decision::Receiver(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the member decision for a node.
    pub fn member_decision(&self, node_id: GlobalNodeIdAny) -> Option<&MemberDecision> {
        match self.decision(node_id) {
            Some(Decision::Member(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the function value selected for a node.
    pub fn function_decision(&self, node_id: GlobalNodeIdAny) -> Option<&FunctionDecision> {
        match self.decision(node_id) {
            Some(Decision::Function(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the declaration symbol the selected function value references.
    pub fn function_symbol(&self, node_id: GlobalNodeIdAny) -> Option<GlobalSymbolId> {
        match self.function_decision(node_id) {
            Some(OperationResolution::One(value)) => value.target.symbol(),
            _ => None,
        }
    }

    /// Get the operator decision for a node.
    pub fn operator_decision(&self, node_id: GlobalNodeIdAny) -> Option<&OperatorDecision> {
        match self.decision(node_id) {
            Some(Decision::Operator(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the call decision for a node.
    pub fn call_decision(&self, node_id: GlobalNodeIdAny) -> Option<&CallDecision> {
        match self.decision(node_id) {
            Some(Decision::Call(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the calls selected at a node, reading through a retained attempt.
    pub fn selected_calls(&self, node_id: GlobalNodeIdAny) -> Option<&[Call]> {
        match self.decision(node_id)? {
            Decision::Call(decision) => Some(decision.arms()),
            Decision::Attempted(attempt) => Some(slice::from_ref(attempt.as_ref())),
            _ => None,
        }
    }

    /// Get the arguments every call arm selected at a node binds identically.
    pub fn agreed_call_arguments(&self, node_id: GlobalNodeIdAny) -> Option<&[ArgumentBinding]> {
        let calls = self.selected_calls(node_id)?;
        let first = calls.first()?.arguments.as_slice();

        // require every arm to bind the same sources in the same order
        let is_agreed = calls.iter().all(|call| {
            call.arguments.len() == first.len()
                && call
                    .arguments
                    .iter()
                    .zip(first)
                    .all(|(left, right)| left.source == right.source)
        });

        is_agreed.then_some(first)
    }

    /// Get the subscript decision for a node.
    pub fn subscript_decision(&self, node_id: GlobalNodeIdAny) -> Option<&SubscriptDecision> {
        match self.decision(node_id) {
            Some(Decision::Subscript(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the assignment decision for a node.
    pub fn assignment_decision(&self, node_id: GlobalNodeIdAny) -> Option<&AssignmentDecision> {
        match self.decision(node_id) {
            Some(Decision::Assignment(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the guard decision for a node.
    pub fn guard_decision(&self, node_id: GlobalNodeIdAny) -> Option<&GuardDecision> {
        match self.decision(node_id) {
            Some(Decision::Guard(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the construct decision for a node.
    pub fn construct_decision(&self, node_id: GlobalNodeIdAny) -> Option<&ConstructDecision> {
        match self.decision(node_id) {
            Some(Decision::Construct(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the tree decision for a node.
    pub fn tree_decision(&self, node_id: GlobalNodeIdAny) -> Option<&TreeDecision> {
        match self.decision(node_id) {
            Some(Decision::Tree(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the pattern decision for a node.
    pub fn pattern_decision(&self, node_id: GlobalNodeIdAny) -> Option<&PatternDecision> {
        match self.decision(node_id) {
            Some(Decision::Pattern(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the assignment pattern decision for a node.
    pub fn assign_pattern_decision(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&AssignPatternDecision> {
        match self.decision(node_id) {
            Some(Decision::AssignPattern(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the access resolution for a node.
    pub fn access_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&AccessResolution> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.access_resolution(node_id))
    }

    /// Get the place resolution for a node.
    pub fn place_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PlaceResolution> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.place_resolution(node_id))
    }

    /// Iterate visible node decisions.
    pub fn decision_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &Decision)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.decision_entries())
    }

    /// Iterate visible access resolutions.
    pub fn access_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AccessResolution)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.access_entries())
    }

    /// Iterate visible place resolutions.
    pub fn place_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &PlaceResolution)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.place_entries())
    }

    /// Return whether this table has no decisions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(DecisionSegment::is_empty)
    }
}

/// Inference decisions added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DecisionSegment {
    /// The module id of the decision segment.
    pub module_id: ModuleId,
    /// The one decision made per DIR node.
    decisions: IndexMap<GlobalNodeIdAny, Decision>,
    /// Decided stable storage accesses keyed by DIR node.
    accesses: IndexMap<GlobalNodeIdAny, AccessResolution>,
    /// Decided place resolutions keyed by DIR node.
    places: IndexMap<GlobalNodeIdAny, PlaceResolution>,
}

/// Per-map decision counts marking one segment position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionMark {
    /// The per-map lengths at the mark.
    lengths: [usize; 3],
}

impl DecisionSegment {
    /// Create an empty decision segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            decisions: IndexMap::default(),
            accesses: IndexMap::default(),
            places: IndexMap::default(),
        }
    }

    /// Set the decision made for a node.
    pub fn set_decision(&mut self, node_id: GlobalNodeIdAny, decision: Decision) {
        self.decisions.insert(node_id, decision);
    }

    /// Get the decision made for a node.
    pub fn decision(&self, node_id: GlobalNodeIdAny) -> Option<&Decision> {
        self.decisions.get(&node_id)
    }

    /// Get the call decision for a node.
    pub fn call_decision(&self, node_id: GlobalNodeIdAny) -> Option<&CallDecision> {
        match self.decision(node_id) {
            Some(Decision::Call(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the construct decision for a node.
    pub fn construct_decision(&self, node_id: GlobalNodeIdAny) -> Option<&ConstructDecision> {
        match self.decision(node_id) {
            Some(Decision::Construct(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the receiver decision for a node.
    pub fn receiver_decision(&self, node_id: GlobalNodeIdAny) -> Option<&ReceiverDecision> {
        match self.decision(node_id) {
            Some(Decision::Receiver(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the member decision for a node.
    pub fn member_decision(&self, node_id: GlobalNodeIdAny) -> Option<&MemberDecision> {
        match self.decision(node_id) {
            Some(Decision::Member(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the operator decision for a node.
    pub fn operator_decision(&self, node_id: GlobalNodeIdAny) -> Option<&OperatorDecision> {
        match self.decision(node_id) {
            Some(Decision::Operator(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the subscript decision for a node.
    pub fn subscript_decision(&self, node_id: GlobalNodeIdAny) -> Option<&SubscriptDecision> {
        match self.decision(node_id) {
            Some(Decision::Subscript(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the assignment decision for a node.
    pub fn assignment_decision(&self, node_id: GlobalNodeIdAny) -> Option<&AssignmentDecision> {
        match self.decision(node_id) {
            Some(Decision::Assignment(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the guard decision for a node.
    pub fn guard_decision(&self, node_id: GlobalNodeIdAny) -> Option<&GuardDecision> {
        match self.decision(node_id) {
            Some(Decision::Guard(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the tree decision for a node.
    pub fn tree_decision(&self, node_id: GlobalNodeIdAny) -> Option<&TreeDecision> {
        match self.decision(node_id) {
            Some(Decision::Tree(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the pattern decision for a node.
    pub fn pattern_decision(&self, node_id: GlobalNodeIdAny) -> Option<&PatternDecision> {
        match self.decision(node_id) {
            Some(Decision::Pattern(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Get the assignment pattern decision for a node.
    pub fn assign_pattern_decision(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<&AssignPatternDecision> {
        match self.decision(node_id) {
            Some(Decision::AssignPattern(decision)) => Some(decision),
            _ => None,
        }
    }

    /// Set the access resolution for a node.
    pub fn set_access_resolution(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: AccessResolution,
    ) {
        self.accesses.insert(node_id, resolution);
    }

    /// Get the access resolution for a node.
    pub fn access_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&AccessResolution> {
        self.accesses.get(&node_id)
    }

    /// Set the place resolution for a node.
    pub fn set_place_resolution(&mut self, node_id: GlobalNodeIdAny, resolution: PlaceResolution) {
        self.places.insert(node_id, resolution);
    }

    /// Get the place resolution for a node.
    pub fn place_resolution(&self, node_id: GlobalNodeIdAny) -> Option<&PlaceResolution> {
        self.places.get(&node_id)
    }

    /// Iterate the node decisions in this segment.
    pub fn decision_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &Decision)> + '_ {
        self.decisions
            .iter()
            .map(|(node_id, decision)| (*node_id, decision))
    }

    /// Iterate the access resolutions in this segment.
    pub fn access_entries(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &AccessResolution)> + '_ {
        self.accesses
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Iterate the place resolutions in this segment.
    pub fn place_entries(&self) -> impl Iterator<Item = (GlobalNodeIdAny, &PlaceResolution)> + '_ {
        self.places
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Drop every decision an earlier sealed segment already carries identically.
    pub fn drop_carried(&mut self, sealed: &DecisionSegment) {
        self.decisions
            .retain(|node, decision| sealed.decisions.get(node) != Some(decision));
        self.accesses
            .retain(|node, resolution| sealed.accesses.get(node) != Some(resolution));
        self.places
            .retain(|node, resolution| sealed.places.get(node) != Some(resolution));
    }

    /// Mark the current segment position for later truncation.
    pub fn mark(&self) -> DecisionMark {
        DecisionMark {
            lengths: [self.decisions.len(), self.accesses.len(), self.places.len()],
        }
    }

    /// Truncate decisions back to one mark, newest first.
    pub fn truncate_to(&mut self, mark: DecisionMark) {
        let [decisions, accesses, places] = mark.lengths;
        Self::truncate_map(&mut self.decisions, decisions);
        Self::truncate_map(&mut self.accesses, accesses);
        Self::truncate_map(&mut self.places, places);
    }

    /// Drop map entries added past one length.
    fn truncate_map<K: Hash + Eq, T>(map: &mut IndexMap<K, T>, length: usize) {
        while map.len() > length {
            map.pop();
        }
    }

    /// Return whether this segment has no decisions.
    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty() && self.accesses.is_empty() && self.places.is_empty()
    }
}

impl TypeFold for DecisionSegment {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(GlobalTypeId) -> Result<GlobalTypeId, E>,
    ) -> Result<(), E> {
        for decision in self.decisions.values_mut() {
            decision.map_types(map)?;
        }
        for place in self.places.values_mut() {
            place.map_types(map)?;
        }

        Ok(())
    }
}
