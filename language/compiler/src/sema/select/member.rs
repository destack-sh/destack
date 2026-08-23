use std::slice;

use destack_dir as dir;
use destack_dir::MemberRole;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CallableArgument, Check, FlowSite, InferMode, MemberCandidate, MemberLookup,
    NullishPart, Origin, PlaceUse, Relation, SelectionCheck, SignatureMatch, Value, ValueUse,
    VariableRole,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Resolve the subject one member access looks its key up in, with the nullish arms it rejects.
    pub(in crate::sema) fn resolve_member_subject(
        &mut self,
        origin: Origin,
        receiver_node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::MemberSubject, Option<NullishPart>)> {
        // split the nullish part off the lookup target
        let split = self.split_nullish_type(origin, target)?;
        let rejected = split.map(|split| split.rejected);
        let target = split.map_or(target, |split| split.value);

        // resolve an open target's value variables before member lookup
        if self.type_flags(target)?.has_variable() {
            let mut variables = self.type_variables(target)?;
            variables.retain(|variable| {
                !matches!(
                    self.infer.variable_role(*variable),
                    Ok(VariableRole::Memory { .. })
                )
            });
            self.resolve_variables(&variables)?;
        }

        // static type aliases look up statics through their declaration reference
        if let Some(symbol) = self.receiver_declaration(receiver_node)
            && self.symbol_kind(symbol)?.is_type_alias()
        {
            let reference =
                self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
            let subject =
                self.member_subject(origin, receiver, reference, dir::MemberSpace::Static)?;

            return Ok((subject, rejected));
        }

        // look the member up in the receiver's own space by default
        let space = self.member_receiver_space(receiver_node, target)?;
        let subject = self.member_subject(origin, receiver, target, space)?;

        Ok((subject, rejected))
    }

    /// Resolve one member lookup subject searched in a decided space.
    pub(in crate::sema) fn member_subject(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        space: dir::MemberSpace,
    ) -> CompilerResult<dir::MemberSubject> {
        let mut subject = target;
        let mut space = space;

        // subject a Type<T> receiver to the statics of T
        if let dir::Type::Application(instance) = self.ty(target)?
            && self.language_item(instance.symbol)? == Some(dir::LanguageItem::Type)
            && let [argument] = self.type_ids(target.module_id, instance.arguments)?
        {
            subject = *argument;
            space = dir::MemberSpace::Static;
        }

        let subject = dir::MemberSubject::new(receiver, subject, space)
            .with_scope(self.assuming_scope(origin)?);

        Ok(subject)
    }

    /// Return the member space one receiver expression selects.
    pub(in crate::sema) fn member_receiver_space(
        &mut self,
        receiver: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::MemberSpace> {
        // declaration references name their own static space
        if matches!(self.ty(ty)?, dir::Type::Reference(_)) {
            return Ok(dir::MemberSpace::Static);
        }

        // select static members for names that resolve to types
        let Some(symbol) = self.receiver_declaration(receiver) else {
            return Ok(dir::MemberSpace::Instance);
        };
        let kind = self.symbol_kind(symbol)?;
        let is_type_name =
            kind.is_nominal() || matches!(kind, dir::SymbolKind::GenericTypeParameter);
        let space = if is_type_name {
            dir::MemberSpace::Static
        } else {
            dir::MemberSpace::Instance
        };

        Ok(space)
    }

    /// Return the declaration one receiver expression names.
    fn receiver_declaration(&self, receiver: dir::GlobalNodeIdAny) -> Option<dir::GlobalSymbolId> {
        match self.name_decision(receiver) {
            Some(resolution) => match resolution.symbols() {
                [symbol] => Some(*symbol),
                _ => None,
            },
            None => match self.decision(receiver) {
                Some(dir::Decision::Function(dir::OperationResolution::One(value))) => {
                    value.target.symbol()
                }
                _ => None,
            },
        }
    }

    /// Return the readable type exposed by one member lookup.
    pub(in crate::sema) fn member_read_type(
        &mut self,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Missing | MemberLookup::Ambiguous => Ok(None),
            MemberLookup::Field(field) => field.read_type(self),
            MemberLookup::Found(candidates) => {
                let candidates = MemberLookup::selected_candidates(candidates);
                let single_slot = candidates
                    .iter()
                    .position(|candidate| candidate.role != MemberRole::Method);
                let candidates = match single_slot {
                    Some(first) => vec![candidates[first]],
                    None => candidates,
                };

                let mut types = Vec::with_capacity(candidates.len());
                for candidate in candidates {
                    types.extend(candidate.read_type(self)?);
                }

                if types.is_empty() {
                    return Ok(None);
                }

                self.normalized_intersection_type(types).map(Some)
            }
            MemberLookup::Union(arms) => {
                let mut types = Vec::with_capacity(arms.len());
                for arm in arms {
                    let Some(ty) = self.member_read_type(&arm.lookup)? else {
                        return Ok(None);
                    };
                    types.push(ty);
                }

                self.normalized_union_type(types).map(Some)
            }
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    types.extend(self.member_read_type(lookup)?);
                }

                if types.is_empty() {
                    return Ok(None);
                }

                self.normalized_intersection_type(types).map(Some)
            }
        }
    }

    /// Return the writable type accepted by one member lookup.
    pub(in crate::sema) fn member_write_type(
        &mut self,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Missing | MemberLookup::Ambiguous => Ok(None),
            MemberLookup::Field(field) => Ok(field.write_type()),
            MemberLookup::Found(candidates) => {
                let writable = MemberLookup::selected_candidates(candidates)
                    .into_iter()
                    .filter(|candidate| candidate.is_writable)
                    .collect::<Vec<_>>();
                let [candidate] = writable.as_slice() else {
                    return Ok(None);
                };

                Ok(Some(candidate.access_type))
            }
            MemberLookup::Union(arms) => {
                let mut types = Vec::with_capacity(arms.len());
                for arm in arms {
                    let Some(type_id) = self.member_write_type(&arm.lookup)? else {
                        return Ok(None);
                    };
                    types.push(type_id);
                }

                self.normalized_intersection_type(types).map(Some)
            }
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(type_id) = self.member_write_type(lookup)? else {
                        return Ok(None);
                    };
                    types.push(type_id);
                }

                self.normalized_intersection_type(types).map(Some)
            }
        }
    }

    /// Return the durable binding represented by one completed member lookup.
    pub(in crate::sema) fn member_binding(
        &mut self,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::MemberBinding>> {
        // compose the operations this lookup exposes
        let read = self.member_read_type(lookup)?;
        let write = self.member_write_type(lookup)?;
        let access = match (read, write) {
            (Some(read), Some(write)) => dir::PropertyAccess::ReadWrite { read, write },
            (Some(read), None) => dir::PropertyAccess::Read(read),
            (None, Some(write)) => dir::PropertyAccess::Write(write),
            (None, None) => return Ok(None),
        };

        // keep each selected declaration once
        let mut declarations = Vec::new();
        for candidate in lookup.declaration_candidates() {
            if declarations
                .iter()
                .any(|declaration: &dir::MemberDeclaration| declaration.symbol == candidate.symbol)
            {
                continue;
            }

            declarations.push(dir::MemberDeclaration {
                symbol: candidate.symbol,
                owner: candidate.owner,
                origin: candidate.origin,
                role: candidate.role,
                callable_type: candidate.callable,
            });
        }

        Ok(Some(dir::MemberBinding::new(
            key,
            lookup.kind(),
            access,
            lookup.is_optional(),
            declarations,
        )))
    }

    /// Select the readable resolution one member lookup exposes.
    pub(in crate::sema) fn select_member_read(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::MemberDecision>> {
        match lookup {
            MemberLookup::Missing | MemberLookup::Ambiguous => Ok(None),
            MemberLookup::Field(field) => Ok(field
                .read_access(receiver.ty, key, self)?
                .map(dir::OperationResolution::One)),
            MemberLookup::Found(candidates) => {
                // keep the candidates the selection precedence ranks first
                let best = candidates.iter().map(MemberCandidate::precedence).min();
                let candidates = candidates
                    .iter()
                    .filter(|candidate| Some(candidate.precedence()) == best)
                    .collect::<Vec<_>>();

                // report survivors from several blocks or interfaces as ambiguous
                let block = |candidate: &MemberCandidate| match candidate.requirement {
                    Some(interface) => interface,
                    None => candidate.owner,
                };
                let first = block(candidates[0]);
                let candidates = if candidates.iter().any(|candidate| block(candidate) != first) {
                    let key = self.format_static_key(&key);
                    self.report_ambiguous_member(origin, key)?;
                    candidates
                        .into_iter()
                        .filter(|candidate| block(candidate) == first)
                        .collect::<Vec<_>>()
                } else {
                    candidates
                };

                // keep the first single-slot declaration among the survivors
                let single_slot = candidates
                    .iter()
                    .position(|candidate| candidate.role != MemberRole::Method);
                let candidates = match single_slot {
                    Some(first) => vec![candidates[first]],
                    None => candidates,
                };

                // build one access per surviving candidate
                let mut targets = Vec::new();
                let mut types = Vec::new();
                for candidate in candidates {
                    let Some(ty) = candidate.read_type(self)? else {
                        continue;
                    };

                    let access = if candidate.role == MemberRole::Getter {
                        // rejecting receivers skip to the next declared candidate
                        let Some(call) = self.select_getter_call(origin, receiver, candidate)?
                        else {
                            continue;
                        };

                        dir::MemberAccess::new(
                            receiver.ty,
                            dir::MemberTarget::Call(Box::new(call)),
                            ty,
                        )
                    } else {
                        candidate.access(receiver.ty, key, ty)
                    };

                    // commit the surviving candidate's site constraints
                    for constraint in &candidate.bounds {
                        self.check.push_relation(*constraint)?;
                    }
                    if let Some(target) = candidate.target {
                        self.constrain_type(
                            target.origin,
                            target.cause,
                            target.relation,
                            target.source,
                            target.target,
                        )?;
                    }

                    targets.push(access.target);
                    types.push(access.ty);
                }

                // several accesses on one key read as their intersection
                let target = match targets.as_slice() {
                    [] => return Ok(None),
                    [target] => target.clone(),
                    _ => dir::MemberTarget::OverloadSet(targets),
                };
                let ty = self.normalized_intersection_type(types)?;
                let access = dir::MemberAccess::new(receiver.ty, target, ty);

                Ok(Some(dir::OperationResolution::One(access)))
            }
            MemberLookup::Union(lookups) => {
                let mut accesses = Vec::with_capacity(lookups.len());
                let mut types = Vec::with_capacity(lookups.len());
                for arm in lookups {
                    let arm_receiver = Value {
                        ty: arm.receiver,
                        ..receiver
                    };
                    let Some(resolution) =
                        self.select_member_read(origin, arm_receiver, key, &arm.lookup)?
                    else {
                        return Ok(None);
                    };

                    let dir::OperationResolution::One(access) = resolution else {
                        return Err(CompilerError::Internal {
                            message: "union member lookup contains a nested union".to_string(),
                        });
                    };

                    types.push(access.ty);
                    accesses.push(access);
                }

                let ty = self.normalized_union_type(types)?;

                Ok(Some(dir::OperationResolution::Union { arms: accesses, ty }))
            }
            MemberLookup::Intersection(lookups) => {
                let mut resolutions = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(resolution) =
                        self.select_member_read(origin, receiver, key, lookup)?
                    else {
                        return Ok(None);
                    };

                    resolutions.push(resolution);
                }

                let resolution = self.intersect_member_decisions(resolutions)?;

                Ok(Some(resolution))
            }
        }
    }

    /// Intersect simultaneous member resolutions, distributing runtime union arms.
    pub(in crate::sema) fn intersect_member_decisions(
        &mut self,
        resolutions: Vec<dir::MemberDecision>,
    ) -> CompilerResult<dir::MemberDecision> {
        // expand each resolution into the runtime combinations it contributes
        let has_union = resolutions
            .iter()
            .any(|resolution| matches!(resolution, dir::OperationResolution::Union { .. }));
        let mut combinations = vec![Vec::new()];
        for resolution in resolutions {
            let accesses = match &resolution {
                dir::OperationResolution::One(access) => slice::from_ref(access),
                dir::OperationResolution::Union { arms, .. } => arms.as_slice(),
            };

            let mut next = Vec::with_capacity(combinations.len() * accesses.len());
            for combination in combinations {
                for access in accesses {
                    let mut combined = combination.clone();
                    combined.push(access.clone());
                    next.push(combined);
                }
            }

            combinations = next;
        }

        // intersect the accesses of every combination into one arm
        let mut arms = Vec::with_capacity(combinations.len());
        for accesses in combinations {
            arms.push(self.intersect_member_accesses(accesses)?);
        }

        if !has_union && arms.len() == 1 {
            return Ok(dir::OperationResolution::One(arms.remove(0)));
        }

        // join the surviving arms into one runtime union
        let types = arms.iter().map(|access| access.ty).collect::<Vec<_>>();
        let ty = self.normalized_union_type(types)?;

        Ok(dir::OperationResolution::Union { arms, ty })
    }

    /// Intersect simultaneous member accesses into one runtime access.
    fn intersect_member_accesses(
        &mut self,
        mut accesses: Vec<dir::MemberAccess>,
    ) -> CompilerResult<dir::MemberAccess> {
        // take a lone access without intersecting
        if accesses.len() == 1 {
            return Ok(accesses.remove(0));
        }

        // flatten every access into one intersected receiver, target, and type
        let mut receivers = Vec::with_capacity(accesses.len());
        let mut targets = Vec::with_capacity(accesses.len());
        let mut types = Vec::with_capacity(accesses.len());
        for access in accesses {
            receivers.push(access.receiver);
            types.push(access.ty);
            match access.target {
                dir::MemberTarget::Intersection(nested) => targets.extend(nested),
                target => targets.push(target),
            }
        }

        let receiver = self.normalized_intersection_type(receivers)?;
        let ty = self.normalized_intersection_type(types)?;
        let target = dir::MemberTarget::Intersection(targets);

        Ok(dir::MemberAccess::new(receiver, target, ty))
    }

    /// Select one getter invocation from a readable member candidate.
    pub(in crate::sema) fn select_getter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<Option<dir::Call>> {
        // select against the adjusted receiver the lookup resolved
        let symbol = candidate.symbol;
        let resolution = candidate.receiver.resolve(receiver.ty);
        let selection_type = match &resolution {
            dir::MemberReceiver::Direct(receiver) => receiver.ty(),
            dir::MemberReceiver::Dynamic(dispatch) => dispatch.constraint,
        };
        let selection_receiver = Value {
            ty: selection_type,
            ..receiver
        };

        // call a getter without arguments
        let arguments = SmallVec::<[CallableArgument; 4]>::new();
        let callable = candidate.callable.ok_or_else(|| CompilerError::Internal {
            message: format!("getter {symbol:?} has no callable type"),
        })?;
        let selected = self.probe_callable(
            origin,
            callable,
            Some(candidate.owner),
            Some(selection_receiver),
            &candidate.generic_arguments,
            &[],
            &arguments,
            None,
        )?;

        // skip a rejecting receiver to the next declared candidate
        let SignatureMatch::Selected(signature) = selected else {
            return Ok(None);
        };

        let arguments = self.bind_argument_sources(origin, &signature, &[])?;
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(Some(resolution))
    }

    /// Select one setter invocation from a writable member candidate.
    pub(in crate::sema) fn select_setter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<dir::Call> {
        // select against the adjusted receiver the lookup resolved
        let symbol = candidate.symbol;
        let resolution = candidate.receiver.resolve(receiver.ty);
        let selection_type = match &resolution {
            dir::MemberReceiver::Direct(receiver) => receiver.ty(),
            dir::MemberReceiver::Dynamic(dispatch) => dispatch.constraint,
        };
        let selection_receiver = Value {
            ty: selection_type,
            ..receiver
        };

        // pass the written value as a setter's sole argument
        let sources = [dir::ArgumentSource::Write];
        let source = self
            .origin_source_node(origin)?
            .into_global(origin.module());
        let arguments = [CallableArgument {
            source,
            ty: Some(candidate.access_type),
            relation: Relation::Assignable,
            use_: ValueUse::Argument,
            is_spread: false,
        }];
        let selected = self.probe_callable(
            origin,
            candidate.callable.ok_or_else(|| CompilerError::Internal {
                message: format!("setter {symbol:?} has no callable type"),
            })?,
            Some(candidate.owner),
            Some(selection_receiver),
            &candidate.generic_arguments,
            &[],
            &arguments,
            None,
        )?;
        let SignatureMatch::Selected(signature) = selected else {
            return Err(CompilerError::Internal {
                message: format!("selected setter {symbol:?} rejects its declared value type"),
            });
        };

        let arguments = self.bind_argument_sources(origin, &signature, &sources)?;
        let resolution = signature.member_call(resolution, candidate.owner, symbol, arguments);

        Ok(resolution)
    }

    /// Select the member meaning of one member access node.
    pub(in crate::sema) fn select_member(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
        is_optional: bool,
    ) -> CompilerResult<()> {
        // read the access node and its typing origin
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // infer the receiver before member lookup
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.visit_site(receiver_node)?;
        let receiver = self.infer_node(receiver_site, PlaceUse::Read, InferMode::Regular)?;
        let written_receiver = self.flow_type_at(receiver_site, receiver)?;
        self.commit_expression_place(receiver_site, written_receiver)?;

        // unknown receivers defer selection until their value settles
        if let Some(stalled_on) = self.check.root_variable(written_receiver)? {
            self.defer_selection(site, stalled_on)?;

            return Ok(());
        }

        // strip the nullish arms the access reads through
        let (subject, rejected) =
            self.resolve_member_subject(origin, receiver_node, receiver, written_receiver)?;
        if let Some(rejected) = rejected
            && !is_optional
        {
            self.report_possibly_nullish(origin, rejected.label().to_string())?;
        }

        // keep the lookup subject at this source site
        self.module_mut(module)
            .members_tail
            .commit_subject(dir::MemberSite::Node(node), subject);

        // commit an error for an omitted member name
        let Some(name) = name else {
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        };

        // probe the written key over the receiver's dereference steps
        let key = dir::StaticKey::Name(name);
        let receiver_site = self.visit_site(receiver_node)?;
        let receiver_value = self.expression_value(receiver_site, receiver)?;
        let mut lookup = self.probe_member(
            origin,
            module,
            receiver_value,
            subject,
            key,
            dir::Access::Readonly,
        )?;

        // materialize the answer for narrowed receivers
        if receiver != subject.target {
            self.adjust_narrowed_lookup(origin, receiver, subject.target, &mut lookup)?;
        }

        match lookup {
            // report a key the receiver exposes nowhere
            MemberLookup::Missing => {
                let key = self.strings().get(name).to_string();

                self.report_rejected_member(node, origin, written_receiver, key)
            }
            // commit whatever the lookup found
            found => {
                self.commit_member_lookup(node, receiver_node, origin, receiver, key, name, &found)
            }
        }
    }

    /// Commit one member lookup.
    fn commit_member_lookup(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver_node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        name: dir::StringId,
        lookup: &MemberLookup,
    ) -> CompilerResult<()> {
        // select the readable resolution, or report why the read fails
        let written_key = self.strings().get(name).to_string();
        let receiver_site = self.visit_site(receiver_node)?;
        let receiver = self.expression_value(receiver_site, receiver)?;
        let Some(resolution) = self.select_member_read(origin, receiver, key, lookup)? else {
            if lookup.has_setter() {
                self.report_write_only_member(origin, written_key)?;
                self.commit_decision(node, dir::Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(());
            }

            return self.report_rejected_member(node, origin, receiver.ty, written_key);
        };

        // reject instance methods read as values outside call positions
        let extracts_method = resolution
            .arms()
            .iter()
            .any(|access| is_bound_method(&access.target));
        if extracts_method && !self.is_callee_position(node) {
            self.report_bound_method_extraction(origin, written_key)?;
            self.commit_decision(node, dir::Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(());
        }

        // commit the exact runtime target tree and joined value type
        let ty = resolution.ty();
        let stored_key = resolution.stored_key();
        self.commit_decision(node, dir::Decision::Member(resolution))?;
        if let Some(key) = stored_key {
            self.commit_projected_access(node, receiver_node, key)?;
            self.commit_access_use(node, dir::BindingUse::READ);
        }

        // narrow the read through the flow state at this site
        let site = self.visit_site(node)?;
        let ty = self.flow_type_at(site, ty)?;
        self.commit_node_type(node, ty)?;

        // an enum variant reads as a fresh literal of its enum
        if matches!(self.ty(ty)?, dir::Type::Variant(_)) {
            self.check.fresh_nodes.insert(node, None);
        }

        Ok(())
    }

    /// Return whether one expression is read as a call callee.
    fn is_callee_position(&self, node: dir::GlobalNodeIdAny) -> bool {
        let module = node.module_id;
        let view = self.module(module).view();
        let tree = view.tree();

        // climb explicit applications to the enclosing call
        let mut current = node.local_id.id;
        while let Some(parent) = tree.get_parent(current) {
            if parent.ty != dir::NodeType::Expression {
                return false;
            }

            let parent_id = dir::LocalNodeId::<dir::Expression>::new(parent.id);
            match view.get(parent_id) {
                dir::Expression::Instantiation { left, .. } if left.id == current => {
                    current = parent.id;
                }
                dir::Expression::Call { left, .. } => return left.id == current,
                _ => return false,
            }
        }

        false
    }

    /// Defer one selection until the stalled variable solves, returning the committed open hole.
    pub(in crate::sema) fn defer_selection(
        &mut self,
        site: FlowSite,
        stalled_on: dir::TypeVariableId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // commit an open hole so enclosing checks proceed
        let node = site.node;
        let hole = match self.committed_node_type(node) {
            // reuse the hole this node already committed
            Some(hole) => hole,
            // allocate one for a node that has none
            None => {
                let variable = self.open_variable(site.origin(), VariableRole::Regular);
                let hole = self.variable_type(variable)?;
                self.commit_node_type(node, hole)?;

                hole
            }
        };

        // re-select once the stalled variable solves
        self.check.queue_check(Check::Selection(SelectionCheck {
            site,
            use_: PlaceUse::Read,
            stalled_on: Some(stalled_on),
        }))?;

        Ok(hole)
    }

    /// Report one rejected member access with a diagnostic.
    fn report_rejected_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<()> {
        // poison when the receiver already reported an error
        if self.has_error_operand(&[receiver])? {
            self.poison_node(node)?;

            return Ok(());
        }

        // report the missing member with the closest reachable key
        let key_span = self.module(node.module_id).diagnostic_span(node.local_id);
        let best = self.closest_member_key(origin, receiver, &key)?;
        self.report_missing_member(origin, receiver, key, key_span, best)?;
        self.commit_decision(node, dir::Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(())
    }
}
/// Return whether one member target selects an instance method.
fn is_bound_method(target: &dir::MemberTarget) -> bool {
    match target {
        dir::MemberTarget::Symbol(candidate) => {
            candidate.space == dir::MemberSpace::Instance && candidate.callable_type.is_some()
        }
        dir::MemberTarget::OverloadSet(targets) | dir::MemberTarget::Intersection(targets) => {
            targets.iter().any(is_bound_method)
        }
        dir::MemberTarget::Call(_)
        | dir::MemberTarget::Field(_)
        | dir::MemberTarget::Projection { .. }
        | dir::MemberTarget::Index(_) => false,
    }
}
