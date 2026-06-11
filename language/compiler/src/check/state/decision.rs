use destack_dir as dir;
use indexmap::IndexMap;

use super::{InferenceSegment, InferenceTable};
use crate::check::{
    CallDecision, ConstructDecision, DecisionKey, Dependency, IdentityDecision, LayoutDecision,
    MemberDecision, OperatorDecision, PatternDecision, ReceiverResolution,
};
use crate::{CompilerError, CompilerResult};

impl InferenceTable {
    /// Return active call decision.
    pub(in crate::check) fn call(&self, source: dir::GlobalNodeIdAny) -> Option<&CallDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.calls.get(&source))
    }

    /// Select one call decision in the current segment.
    pub(in crate::check) fn select_call(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: CallDecision,
    ) -> CompilerResult<()> {
        let inserted = self.insert_once(
            source,
            "call",
            decision,
            |segment| &segment.calls,
            |segment| &mut segment.calls,
        )?;
        if inserted {
            self.wake_dependency(Dependency::Decision(DecisionKey::Call(source)));
            self.wake_dependency(Dependency::Decision(DecisionKey::Callable(source)));
        }

        Ok(())
    }

    /// Return active construct decision.
    pub(in crate::check) fn construct(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<&ConstructDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.constructs.get(&source))
    }

    /// Select one construct decision in the current segment.
    pub(in crate::check) fn select_construct(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: ConstructDecision,
    ) -> CompilerResult<()> {
        let inserted = self.insert_once(
            source,
            "construct",
            decision,
            |segment| &segment.constructs,
            |segment| &mut segment.constructs,
        )?;
        if inserted {
            self.wake_dependency(Dependency::Decision(DecisionKey::Construct(source)));
            self.wake_dependency(Dependency::Decision(DecisionKey::Callable(source)));
        }

        Ok(())
    }

    /// Return active operator decision.
    pub(in crate::check) fn operator(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<&OperatorDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.operators.get(&source))
    }

    /// Select one operator decision in the current segment.
    pub(in crate::check) fn select_operator(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: OperatorDecision,
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "operator",
            DecisionKey::Operator(source),
            decision,
            |segment| &segment.operators,
            |segment| &mut segment.operators,
        )
    }

    /// Return active identity decision.
    pub(in crate::check) fn identity(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<&IdentityDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.identities.get(&source))
    }

    /// Select one identity decision in the current segment.
    pub(in crate::check) fn select_identity(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: IdentityDecision,
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "identity",
            DecisionKey::Identity(source),
            decision,
            |segment| &segment.identities,
            |segment| &mut segment.identities,
        )
    }

    /// Return active layout decision.
    pub(in crate::check) fn layout(&self, source: dir::GlobalNodeIdAny) -> Option<&LayoutDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.layouts.get(&source))
    }

    /// Select one layout decision in the current segment.
    pub(in crate::check) fn select_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: LayoutDecision,
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "layout",
            DecisionKey::Layout(source),
            decision,
            |segment| &segment.layouts,
            |segment| &mut segment.layouts,
        )
    }

    /// Return active member decision.
    pub(in crate::check) fn member(&self, source: dir::GlobalNodeIdAny) -> Option<&MemberDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.members.get(&source))
    }

    /// Select one member decision in the current segment.
    pub(in crate::check) fn select_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: MemberDecision,
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "member",
            DecisionKey::Member(source),
            decision,
            |segment| &segment.members,
            |segment| &mut segment.members,
        )
    }

    /// Select one pattern decision in the current segment.
    pub(in crate::check) fn select_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: PatternDecision,
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "pattern",
            DecisionKey::Pattern(source),
            decision,
            |segment| &segment.patterns,
            |segment| &mut segment.patterns,
        )
    }

    /// Return active pattern decision.
    pub(in crate::check) fn pattern(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<&PatternDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.patterns.get(&source))
    }

    /// Select one receiver resolution in the current segment.
    pub(in crate::check) fn select_receiver(
        &mut self,
        receiver: ReceiverResolution,
    ) -> CompilerResult<()> {
        self.select_once(
            receiver.source,
            "receiver",
            DecisionKey::Receiver(receiver.source),
            receiver,
            |segment| &segment.receivers,
            |segment| &mut segment.receivers,
        )
    }

    /// Return active receiver resolution.
    pub(in crate::check) fn receiver(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<ReceiverResolution> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.receivers.get(&source).copied())
    }

    /// Return active name resolution.
    pub(in crate::check) fn name(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<&dir::NameResolution> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.names.get(&source))
    }

    /// Select one name resolution in the current segment.
    pub(in crate::check) fn select_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: dir::NameResolution,
    ) -> CompilerResult<()> {
        self.insert_once(
            source,
            "name",
            resolution,
            |segment| &segment.names,
            |segment| &mut segment.names,
        )?;

        Ok(())
    }

    /// Return the total number of decisions.
    pub(in crate::check) fn decision_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| {
                segment.calls.len()
                    + segment.constructs.len()
                    + segment.operators.len()
                    + segment.identities.len()
                    + segment.layouts.len()
                    + segment.members.len()
                    + segment.patterns.len()
                    + segment.receivers.len()
                    + segment.names.len()
            })
            .sum()
    }

    /// Return active names in component order.
    pub(in crate::check) fn names(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, &dir::NameResolution)> {
        self.segments.iter().flat_map(|segment| {
            segment
                .names
                .iter()
                .map(|(source, resolution)| (*source, resolution))
        })
    }

    /// Return active receivers in component order.
    pub(in crate::check) fn receivers(&self) -> impl Iterator<Item = ReceiverResolution> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.receivers.values().copied())
    }

    /// Return active member decisions in component order.
    pub(in crate::check) fn members(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, &MemberDecision)> {
        self.segments.iter().flat_map(|segment| {
            segment
                .members
                .iter()
                .map(|(source, decision)| (*source, decision))
        })
    }

    /// Return active pattern decisions in component order.
    pub(in crate::check) fn patterns(&self) -> impl Iterator<Item = &PatternDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.patterns.values())
    }

    /// Return active call decisions in component order.
    pub(in crate::check) fn calls(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, &CallDecision)> {
        self.segments.iter().flat_map(|segment| {
            segment
                .calls
                .iter()
                .map(|(source, decision)| (*source, decision))
        })
    }

    /// Return active construct decisions in component order.
    pub(in crate::check) fn constructs(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, &ConstructDecision)> {
        self.segments.iter().flat_map(|segment| {
            segment
                .constructs
                .iter()
                .map(|(source, decision)| (*source, decision))
        })
    }

    /// Return active operator decisions in component order.
    pub(in crate::check) fn operators(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, &OperatorDecision)> {
        self.segments.iter().flat_map(|segment| {
            segment
                .operators
                .iter()
                .map(|(source, decision)| (*source, decision))
        })
    }

    /// Return active layout decisions in component order.
    pub(in crate::check) fn layouts(&self) -> impl Iterator<Item = &LayoutDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.layouts.values())
    }

    /// Select one stable value in the current segment.
    fn select_once<T: std::fmt::Debug + PartialEq>(
        &mut self,
        source: dir::GlobalNodeIdAny,
        label: &str,
        decision: DecisionKey,
        value: T,
        select: impl Fn(&InferenceSegment) -> &IndexMap<dir::GlobalNodeIdAny, T>,
        select_mut: impl Fn(&mut InferenceSegment) -> &mut IndexMap<dir::GlobalNodeIdAny, T>,
    ) -> CompilerResult<()> {
        let inserted = self.insert_once(source, label, value, select, select_mut)?;
        if inserted {
            self.wake_dependency(Dependency::Decision(decision));
        }

        Ok(())
    }

    /// Insert one stable value in the current segment.
    fn insert_once<T: std::fmt::Debug + PartialEq>(
        &mut self,
        source: dir::GlobalNodeIdAny,
        label: &str,
        value: T,
        select: impl Fn(&InferenceSegment) -> &IndexMap<dir::GlobalNodeIdAny, T>,
        select_mut: impl Fn(&mut InferenceSegment) -> &mut IndexMap<dir::GlobalNodeIdAny, T>,
    ) -> CompilerResult<bool> {
        for segment in &self.segments {
            let Some(existing) = select(segment).get(&source) else {
                continue;
            };
            if existing == &value {
                return Ok(false);
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "check {label} {source:?} received two different selections: {existing:?} and {value:?}"
                ),
            });
        }

        select_mut(self.current_mut()).insert(source, value);

        Ok(true)
    }
}
