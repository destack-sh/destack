use destack_dir as dir;
use destack_dir::MemberRole;

use crate::sema::{
    CallableArgument, Cause, CauseKind, CheckState, DeclaredCandidate, FlowSite, InferMode,
    MemberCandidate, MemberLookup, NullishPart, Origin, PlaceUse, Relation, Settle, SignatureMatch,
    Value, ValueUse, VariableKind, is_optional_member, member_arms, member_kind,
    selected_candidates,
};
use crate::{CheckError, CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Settle the subject one member access looks its key up in, with the nullish arms it rejects.
    pub(in crate::sema) fn resolve_member_subject(
        &mut self,
        origin: Origin,
        receiver_node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<(
        dir::MemberSubject,
        Option<NullishPart>,
        [dir::GlobalTypeId; 2],
    )> {
        // split the nullish part off the lookup target
        let written = target;
        let split = self.split_nullish_type(origin, target)?;
        let rejected = split.map(|split| split.rejected);
        let target = split.map_or(target, |split| split.value);

        // resolve an open target's shape variables before member lookup
        if self.type_flags(target)?.has_variable() {
            let mut variables = self.type_variables(target)?;
            variables.retain(|variable| {
                self.root_kind(*variable)
                    .is_ok_and(|kind| kind == VariableKind::Type)
            });
            self.settle_variables(&variables, Settle::All)?;
        }
        let target = self.shallow_resolve(target)?;
        let receiver = self.shallow_resolve(receiver)?;
        let settled = [receiver, self.shallow_resolve(written)?];

        // static type aliases look up statics through their declaration reference
        if let Some(symbol) = self.receiver_declaration(receiver_node)
            && self.symbol_kind(symbol)?.is_type_alias()
        {
            let reference =
                self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
            let subject =
                self.member_subject(origin, receiver, reference, dir::MemberSpace::Static)?;

            return Ok((subject, rejected, settled));
        }

        // look the member up in the receiver's own space by default
        let space = self.member_receiver_space(receiver_node, target)?;
        let mut subject = self.member_subject(origin, receiver, target, space)?;
        if let dir::Type::Literal(_) = self.ty(self.shallow_resolve(target)?)? {
            let primitive = self.widen_type(target)?;
            subject = subject.with_key_source(primitive);
        }

        Ok((subject, rejected, settled))
    }

    /// Settle one member lookup subject searched in a decided space.
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

    /// Check every selected member arm's visibility at its access site.
    pub(in crate::sema) fn check_member_access(
        &mut self,
        origin: Origin,
        resolution: &dir::MemberDecision,
        key: &str,
    ) -> CompilerResult<()> {
        for access in resolution.arms() {
            let Some(member) = access.target.symbol() else {
                continue;
            };
            self.check_symbol_access(origin, member, key)?;
        }

        Ok(())
    }

    /// Check one selected member symbol's visibility at its access site.
    pub(in crate::sema) fn check_symbol_access(
        &mut self,
        origin: Origin,
        member: dir::GlobalSymbolId,
        key: &str,
    ) -> CompilerResult<()> {
        let Some((owner, visibility)) = self.member_visibility(member)? else {
            return Ok(());
        };

        // admit the site by the member's declared visibility
        match visibility {
            // public members admit every site
            dir::Visibility::Public => Ok(()),
            // protected members admit the owner and its derived declarations
            dir::Visibility::Protected => {
                if self.protected_access_admits(owner)? {
                    return Ok(());
                }

                self.report_inaccessible_member(origin, key.to_string(), visibility)
            }
            // private members admit their declaring module
            dir::Visibility::Private => {
                if member.module_id == self.module_id {
                    return Ok(());
                }

                self.report_inaccessible_member(origin, key.to_string(), visibility)
            }
        }
    }

    /// Check one newtype's backing visibility at a construction or unwrap site.
    pub(in crate::sema) fn check_backing_access(
        &mut self,
        origin: Origin,
        newtype: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(newtype)? else {
            return Ok(());
        };
        let visibility = definition.backing_visibility;

        // admit the sites the declared backing visibility allows
        let admits = match visibility {
            dir::Visibility::Public => true,
            dir::Visibility::Protected => self.protected_access_admits(newtype)?,
            dir::Visibility::Private => newtype.module_id == self.module_id,
        };
        if admits {
            return Ok(());
        }

        let name = self.format_symbol(newtype);

        self.report_inaccessible_newtype_backing(origin, name, visibility)
    }

    /// Return whether the checking scope derives from one protected owner.
    fn protected_access_admits(&mut self, owner: dir::GlobalSymbolId) -> CompilerResult<bool> {
        // find the declaring construct enclosing this site
        let declaration = self
            .flow
            .current_function_receiver()
            .and_then(|binding| binding.receiver.declaration);
        let Some(declaration) = declaration else {
            return Ok(false);
        };
        if declaration == owner {
            return Ok(true);
        }

        self.reaches_heritage(declaration, owner)
    }

    /// Return the receiver closing one this-polymorphic interface member, absent elsewhere.
    pub(in crate::sema) fn interface_member_receiver(
        &mut self,
        owner: dir::GlobalSymbolId,
        callable: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // interface members close their receiver into the instance identity
        let is_interface = matches!(self.definition(owner)?, Some(dir::Definition::Interface(_)));
        if !is_interface {
            return Ok(None);
        }

        // read the closed receiver off the substituted this parameter
        let Some(this_parameter) = self
            .signature_head(callable)?
            .and_then(|signature| signature.this_parameter)
        else {
            return Ok(None);
        };
        let origin = Origin::Symbol(owner);
        let receiver = self.strip_form(origin, this_parameter)?;

        Ok(Some(receiver))
    }

    /// Return the declaration one receiver expression names.
    fn receiver_declaration(&self, receiver: dir::GlobalNodeIdAny) -> Option<dir::GlobalSymbolId> {
        // read the declaration the receiver name resolves to
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
        // union the reads of every runtime arm
        let mut types = Vec::new();
        for (_, group) in member_arms(lookup) {
            let candidates = Self::read_candidates(&group);

            // intersect what every surviving candidate reads
            let mut reads = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                reads.extend(candidate.read_type(self)?);
            }
            if reads.is_empty() {
                return Ok(None);
            }
            types.push(self.normalized_intersection_type(reads)?);
        }
        if types.is_empty() {
            return Ok(None);
        }

        self.normalized_union_type(types).map(Some)
    }

    /// Keep the selected candidates one read joins: the first single-slot one, else every method.
    fn read_candidates<'candidate>(
        candidates: &[&'candidate MemberCandidate],
    ) -> Vec<&'candidate MemberCandidate> {
        let candidates = selected_candidates(candidates);
        let single_slot = candidates
            .iter()
            .position(|candidate| candidate.role != MemberRole::Method);

        // keep the single slot a field or accessor fills
        match single_slot {
            Some(first) => vec![candidates[first]],
            None => candidates,
        }
    }

    /// Return the writable type accepted by one member lookup.
    pub(in crate::sema) fn member_write_type(
        &mut self,
        lookup: &MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut types = Vec::new();
        for (_, group) in member_arms(lookup) {
            let writable = selected_candidates(&group)
                .into_iter()
                .filter_map(|candidate| candidate.access.write())
                .collect::<Vec<_>>();
            let [write] = writable.as_slice() else {
                return Ok(None);
            };
            types.push(*write);
        }
        if types.is_empty() {
            return Ok(None);
        }

        self.normalized_intersection_type(types).map(Some)
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
        let mut declarations = Vec::<dir::MemberDeclaration>::new();
        for (_, group) in member_arms(lookup) {
            for candidate in selected_candidates(&group) {
                let Some(declared) = candidate.declaration() else {
                    continue;
                };
                if declarations
                    .iter()
                    .any(|declaration| declaration.symbol == declared.symbol)
                {
                    continue;
                }

                declarations.push(dir::MemberDeclaration {
                    symbol: declared.symbol,
                    owner: declared.owner,
                    origin: declared.origin,
                    role: candidate.role,
                    callable_type: candidate.callable,
                });
            }
        }

        Ok(Some(dir::MemberBinding::new(
            key,
            member_kind(lookup),
            access,
            is_optional_member(lookup),
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
        // read every runtime arm, joining several as one union resolution
        let mut accesses = Vec::new();
        let mut types = Vec::new();
        let arms = member_arms(lookup);
        let is_union = arms.iter().any(|(arm, _)| arm.is_some());
        for (arm, group) in arms {
            let arm_receiver = match arm {
                Some(arm) => Value {
                    ty: arm.receiver,
                    ..receiver
                },
                None => receiver,
            };
            let Some(access) = self.select_arm_read(origin, arm_receiver, key, &group)? else {
                return Ok(None);
            };
            types.push(access.ty);
            accesses.push(access);
        }

        Ok(Some(match (is_union, accesses.as_slice()) {
            (false, [_]) => dir::OperationResolution::One(accesses.remove(0)),
            _ => {
                let ty = self.normalized_union_type(types)?;

                dir::OperationResolution::Union { arms: accesses, ty }
            }
        }))
    }

    /// Select the readable access one runtime arm's candidates expose.
    fn select_arm_read(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        candidates: &[&MemberCandidate],
    ) -> CompilerResult<Option<dir::MemberAccess>> {
        // keep the candidates the selection precedence ranks first
        let candidates = selected_candidates(candidates);
        let [first, ..] = candidates.as_slice() else {
            return Ok(None);
        };

        // report survivors from several blocks or interfaces as ambiguous
        let block = |candidate: &MemberCandidate| {
            candidate
                .declaration()
                .map(DeclaredCandidate::declaring_block)
        };
        // equally typed data requirements of distinct interfaces meet as one member
        let first_type = first.access.store();
        let first = block(first);
        let mut is_split = false;
        for candidate in &candidates {
            let candidate_block = block(candidate);
            if candidate_block == first {
                continue;
            }
            let merges = match (first, candidate_block) {
                (Some(known), Some(other)) => {
                    !candidate.role.is_callable()
                        && self.symbol_kind(known)?.is_interface()
                        && self.symbol_kind(other)?.is_interface()
                        && candidate.access.store() == first_type
                }
                _ => false,
            };
            is_split |= !merges;
        }
        let candidates = if is_split {
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

        // build one access per surviving candidate at this use site
        let mut targets = Vec::new();
        let mut types = Vec::new();
        for candidate in candidates {
            let candidate = &candidate.instantiate(origin, self)?;
            let Some(ty) = candidate.read_type(self)? else {
                continue;
            };

            let access = if candidate.role == MemberRole::Getter {
                // skip a rejecting receiver to the next declared candidate
                let Some(call) = self.select_getter_call(origin, receiver, candidate)? else {
                    continue;
                };

                dir::MemberAccess::new(receiver.ty, dir::MemberTarget::Call(Box::new(call)), ty)
            } else {
                candidate.access(receiver.ty, key, ty)
            };

            // commit the surviving candidate's site constraints
            if let Some(declared) = candidate.declaration() {
                for constraint in &declared.bounds {
                    self.push_relation(*constraint)?;
                }
                if let Some(target) = declared.target {
                    self.constrain_type(
                        target.origin,
                        target.cause,
                        target.relation,
                        target.source,
                        target.target,
                    )?;
                }
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

        Ok(Some(dir::MemberAccess::new(receiver.ty, target, ty)))
    }

    /// Select one call of a declared member candidate on its adjusted receiver.
    pub(in crate::sema) fn select_member_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
        arguments: &[CallableArgument],
        sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Option<dir::Call>> {
        let Some(declared) = candidate.declaration() else {
            return Ok(None);
        };
        let Some(callable) = candidate.callable else {
            return Err(CompilerError::Internal {
                message: format!("a member {:?} without a callable type", declared.symbol),
            });
        };

        // select against the adjusted receiver the lookup resolved
        let resolution = candidate.receiver.resolve(receiver.ty);
        let selection_type = match &resolution {
            dir::MemberReceiver::Direct(receiver) => receiver.ty(),
            dir::MemberReceiver::Dynamic(dispatch) => dispatch.constraint,
        };
        let selection_receiver = Value {
            ty: selection_type,
            ..receiver
        };
        let selected = self.match_callable(
            origin,
            callable,
            Some(declared.owner),
            Some(selection_receiver),
            None,
            &declared.generic_arguments,
            &[],
            arguments,
            None,
        )?;
        let SignatureMatch::Selected(signature) = selected else {
            return Ok(None);
        };

        // bind the sources and the receiver the signature selected
        let bound = self.bind_argument_sources(origin, &signature, sources)?;
        let key_receiver = self.interface_member_receiver(declared.owner, signature.callable)?;
        let call = signature.member_call(
            resolution,
            declared.owner,
            declared.symbol,
            key_receiver,
            bound,
        );

        Ok(Some(call))
    }

    /// Select one getter invocation from a readable member candidate.
    pub(in crate::sema) fn select_getter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<Option<dir::Call>> {
        self.select_member_call(origin, receiver, candidate, &[], &[])
    }

    /// Select one setter invocation from a writable member candidate, passing the written value.
    pub(in crate::sema) fn select_setter_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        candidate: &MemberCandidate,
    ) -> CompilerResult<dir::Call> {
        let source = self
            .origin_source_node(origin)?
            .into_global(origin.module());
        let arguments = [CallableArgument {
            source,
            ty: Some(candidate.access.store()),
            relation: Relation::Storable,
            use_: ValueUse::Argument,
            is_spread: false,
        }];
        let sources = [dir::ArgumentSource::Write];
        let call = self.select_member_call(origin, receiver, candidate, &arguments, &sources)?;

        call.ok_or_else(|| CompilerError::Internal {
            message: format!(
                "selected setter {:?} rejects its declared value type",
                candidate.symbol()
            ),
        })
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

        let written_receiver = self.resolve_structurally(site, written_receiver)?;
        let written_receiver = self.settle_observed_width(site, written_receiver)?;

        // strip the nullish arms the access reads through
        let (subject, rejected, [receiver, written_receiver]) =
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
        let mut lookup = self.match_member(
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

        // report a key the receiver exposes nowhere, else commit what the lookup found
        if lookup.is_empty() {
            let key = self.strings().get(name).to_string();

            return self.report_rejected_member(node, origin, written_receiver, key);
        }

        self.commit_member_lookup(node, receiver_node, origin, receiver, key, name, &lookup)
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
            if lookup
                .iter()
                .any(|candidate| candidate.role == MemberRole::Setter)
            {
                self.report_write_only_member(origin, written_key)?;
                self.commit_decision(node, dir::Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(());
            }

            return self.report_rejected_member(node, origin, receiver.ty, written_key);
        };

        // check the selected arms' visibility from this site
        self.check_member_access(origin, &resolution, &written_key)?;

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

        // read the joined value type and the stored key the resolution names
        let ty = resolution.ty();
        let stored_key = resolution.stored_key();

        // requalify stored reads at the receiver's storage placement
        let ty = match stored_key {
            Some(_) => {
                let placement = self.value_place(origin, receiver)?.placement;
                self.place_relative_type(origin, placement, ty)?
            }
            None => ty,
        };
        self.commit_decision(node, dir::Decision::Member(resolution))?;
        if let Some(key) = stored_key {
            self.commit_projected_access(node, receiver_node, key)?;
            self.commit_access_use(node, dir::BindingUse::READ);
        }

        // narrow the read through the flow state at this site
        let site = self.visit_site(node)?;
        let ty = self.flow_type_at(site, ty)?;
        self.commit_node_type(node, ty)?;

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

    /// Settle one observed numeric width at its family form.
    ///
    /// Member lookup enumerates the receiver's surface, which an open width
    /// cannot offer; the family fallback joins beside the width's bounds like
    /// any settled candidate.
    fn settle_observed_width(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let resolved = self.shallow_resolve(ty)?;
        let Some(variable) = self.root_variable(resolved)? else {
            return Ok(resolved);
        };
        let root = self.infer.alias_root(variable)?;
        if !self.root_kind(root)?.is_numeric() {
            return Ok(resolved);
        }

        // join the family fallback beside the width's bounds and settle now
        let origin = site.origin();
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        self.widen_numeric_slot(root, origin, cause)?;
        self.settle_variables(&[root], Settle::All)?;

        self.shallow_resolve(resolved)
    }

    /// Settle one selection operand structurally, reporting a head nothing decides.
    pub(in crate::sema) fn resolve_structurally(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = self.shallow_resolve(ty)?;
        let Some(variable) = self.root_variable(ty)? else {
            return Ok(ty);
        };

        // keep a numeric variable's width open for its later uses
        let root = self.infer.alias_root(variable)?;
        let widens_now = self.root_kind(variable)? == VariableKind::Type;
        if !widens_now {
            return Ok(ty);
        }

        // solve what the pending work decides to a fixpoint
        while self.solve_where_possible(Settle::Possible)? {}
        let mut root = root;
        loop {
            self.settle_variables(&[root], Settle::All)?;
            if let Some(solution) = self.infer.solution(root)? {
                return Ok(solution);
            }
            let settled = self.infer.alias_root(root)?;
            if settled == root {
                break;
            }
            root = settled;
        }
        if self.root_kind(root)?.is_numeric() {
            return self.variable_type(self.infer.alias_root(root)?);
        }

        // close an undecided head at the error type
        let error = self.report_cannot_infer(site)?;
        self.commit_error_solution(root, error)?;

        Ok(error)
    }

    /// Report one site whose type nothing infers, returning the error type.
    pub(in crate::sema) fn report_cannot_infer(
        &mut self,
        site: FlowSite,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = site.node.module_id;
        let anchor = self.diagnostic_anchor(module, site.node.local_id);
        self.report(module, CheckError::CannotInferType { anchor, module });

        self.intern_type(dir::Type::Error)
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
