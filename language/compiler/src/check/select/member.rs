use destack_core::closest_string;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckEvent, CheckState, Decision, MemberCandidate, MemberLookup, Mutation, Origin,
};
use crate::{CheckError, CompilerError, CompilerResult, diagnostic_suggestion_distance};

impl CheckState<'_> {
    /// Select the member meaning of one member access node.
    pub(in crate::check) fn select_member(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        let Some(name) = name else {
            return Err(CompilerError::Internal {
                message: format!("member node {node:?} has no name"),
            });
        };
        let key = dir::StaticKey::Name(name);

        // wait for the receiver node type
        let receiver_node = left.into_global_any(module);
        let Some(receiver) = self.inputs.node_type(receiver_node) else {
            return Err(CompilerError::Internal {
                message: format!("member receiver {receiver_node:?} has no input type"),
            });
        };

        // reads through possibly nullish receivers fail loudly
        let receiver = match self.split_nullish_receiver(origin, node, receiver)? {
            Answer::Ready(receiver) => receiver,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // names of nominal declarations access their static member space
        let space = match self.member_receiver_declaration(receiver_node)? {
            Some(_) => dir::MemberSpace::Static,
            None => dir::MemberSpace::Instance,
        };

        // look the member up on the receiver
        let lookup = self.lookup_member(origin, module, receiver, space, key)?;
        match lookup {
            // structural fields bound the node directly
            MemberLookup::Field(ty) => {
                self.record_member_decision(node, receiver, key, None, ty)?;

                Ok(Answer::Ready(()))
            }
            // single candidates select directly
            MemberLookup::Found(candidates) => {
                let candidates = candidates
                    .into_iter()
                    .collect::<SmallVec<[MemberCandidate; 2]>>();
                let [candidate] = candidates.as_slice() else {
                    // overloaded members defer selection to the call site
                    let Some(first) = candidates.first() else {
                        let key = self.module(module).strings.get(name).to_string();

                        return self.reject_member(node, origin, receiver, key);
                    };
                    // union receivers record one candidate per variant
                    let is_union = match self.evaluate_root(origin, receiver)? {
                        Answer::Ready(reduced) => {
                            matches!(self.ty(reduced)?, dir::Type::Union(_))
                        }
                        Answer::Pending(_) => false,
                    };
                    self.record_overloaded_member_decision(
                        node,
                        receiver,
                        first.ty,
                        &candidates,
                        is_union,
                    )?;

                    return Ok(Answer::Ready(()));
                };

                self.record_member_decision(node, receiver, key, candidate.symbol, candidate.ty)?;

                Ok(Answer::Ready(()))
            }
            MemberLookup::Missing => {
                let key = self.module(module).strings.get(name).to_string();

                self.reject_member(node, origin, receiver, key)
            }
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Record one member decision and bound the node variable.
    fn record_member_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        symbol: Option<dir::GlobalSymbolId>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // record the committed resolution shape
        let target = match symbol {
            Some(symbol) => dir::MemberTarget::Symbol(dir::MemberCandidate {
                receiver,
                symbol,
                ty,
                arguments: Vec::new(),
            }),
            None => dir::MemberTarget::Field(key),
        };
        let resolution = dir::MemberResolution::new(receiver, target);
        self.record_decision(node, Decision::Member(resolution))?;
        self.bound_member_node(node, ty)?;

        Ok(())
    }

    /// Record one multi-candidate member decision and bound the node.
    ///
    /// Call selection picks among the candidates' applied types.
    fn record_overloaded_member_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
        candidates: &[MemberCandidate],
        is_union: bool,
    ) -> CompilerResult<()> {
        let candidates = candidates
            .iter()
            .filter_map(|candidate| {
                Some(dir::MemberCandidate {
                    receiver,
                    symbol: candidate.symbol?,
                    ty: candidate.ty,
                    arguments: Vec::new(),
                })
            })
            .collect::<Vec<_>>();
        let target = if is_union {
            dir::MemberTarget::Union(candidates)
        } else {
            dir::MemberTarget::Overloaded(candidates)
        };
        let resolution = dir::MemberResolution::new(receiver, target);
        self.record_decision(node, Decision::Member(resolution))?;
        self.bound_member_node(node, ty)?;

        Ok(())
    }

    /// Flow one selected member type into the member node variable.
    fn bound_member_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(variable) = self.inputs.node_type(node).and_then(|input| {
            self.ty(input).ok().and_then(|input| match input {
                dir::Type::Variable(variable) => Some(*variable),
                _ => None,
            })
        }) {
            self.push_lower_bound(variable, ty)?;
        }

        Ok(())
    }

    /// Reject one member access with a diagnostic.
    fn reject_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<Answer<()>> {
        self.report_missing_member(origin, receiver, key)?;
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }

    /// Report nullish receiver parts and return the value part.
    /// Member selection continues over the non-nullish elements.
    fn split_nullish_receiver(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let reduced = match self.evaluate_root(origin, receiver)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let dir::Type::Union(union) = self.ty(reduced)? else {
            return Ok(Answer::Ready(receiver));
        };

        // split nullish parts from the readable elements
        let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
        let mut has_null = false;
        let mut has_undefined = false;
        let mut values = Vec::with_capacity(elements.len());
        for element in elements {
            match self.ty(element)? {
                dir::Type::Null => has_null = true,
                dir::Type::Undefined => has_undefined = true,
                _ => values.push(element),
            }
        }
        if !has_null && !has_undefined {
            return Ok(Answer::Ready(receiver));
        }

        // name the nullish parts that block the read
        let nullish = match (has_null, has_undefined) {
            (true, true) => "null or undefined",
            (true, false) => "null",
            (false, true) => "undefined",
            (false, false) => unreachable!("nullish split requires a nullish part"),
        };
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::PossiblyNullish {
            anchor,
            module,
            nullish: nullish.to_string(),
        };
        self.module_mut(module).diagnostics.push(error.into());

        // continue selection over the readable part
        let source = self.origin_source_node(origin)?;
        let value = match values.as_slice() {
            [] => self.push_type(node.module_id, dir::Type::Never, source)?,
            [single] => *single,
            _ => self.push_type(
                node.module_id,
                dir::Type::Union(dir::UnionType { elements: values }),
                source,
            )?,
        };

        Ok(Answer::Ready(value))
    }

    /// Report one missing member with the closest visible suggestion.
    pub(in crate::check) fn report_missing_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let receiver_text = self.format_type(receiver);

        // suggest the closest visible member key
        let suggestion = self.closest_member_key(receiver, &key)?;
        let error = CheckError::MissingMember {
            anchor,
            module,
            key,
            receiver: receiver_text,
            suggestion,
        };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Return the visible member key closest to one missing key.
    fn closest_member_key(
        &mut self,
        receiver: dir::GlobalTypeId,
        key: &str,
    ) -> CompilerResult<Option<String>> {
        let keys = self.visible_member_keys(receiver)?;

        Ok(closest_string(
            key,
            keys,
            diagnostic_suggestion_distance(key),
        ))
    }

    /// Collect the member keys visible on one receiver.
    fn visible_member_keys(&mut self, receiver: dir::GlobalTypeId) -> CompilerResult<Vec<String>> {
        // look through memory forms to the carried value
        let mut current = self.resolve_root(receiver)?;
        while let dir::Type::Form(form) = self.ty(current)? {
            current = self.resolve_root(form.value)?;
        }

        let mut keys = Vec::new();
        match self.ty(current)? {
            // structural shapes list their fields
            dir::Type::Shape(shape) => {
                for field in &shape.fields {
                    keys.push(self.format_static_key(&field.key));
                }
            }
            // declarations list their keyed members
            dir::Type::Reference(instance) => {
                if let Some(definition) = self.definition(instance.symbol) {
                    for member in definition.members() {
                        if let Some(key) = member.key() {
                            keys.push(self.format_static_key(&key));
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(keys)
    }

    /// Return the nominal declaration one member receiver names (if any),
    fn member_receiver_declaration(
        &mut self,
        receiver: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let symbol = match self.decisions.get(receiver) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => *symbol,
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        // re-exporting modules bind alias symbols whose declarations
        // live on their targets
        let symbol = self.resolve_external_alias(symbol)?;

        Ok(self.symbol_kind(symbol).is_nominal().then_some(symbol))
    }

    /// Record one node decision and wake its waiters.
    pub(in crate::check) fn record_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        decision: Decision,
    ) -> CompilerResult<()> {
        let waiters = self.decisions.decide(node, decision)?;
        self.record_event(CheckEvent::Decide { node });
        self.journal.record_with(|| Mutation::DecisionSet {
            node,
            waiters: waiters.clone(),
        });

        // wake tasks parked on the decision
        for waiter in waiters {
            self.queue_task(waiter);
        }

        Ok(())
    }
}
