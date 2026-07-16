use destack_dir as dir;
use destack_source::{ModuleId, Span};
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CheckState, Decision, FlowSite, Origin, PlaceUse, ReceiverSteps, Relation,
    TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

/// Result of looking up one member on a receiver type.
#[derive(Debug, Clone)]
pub(in crate::check) enum MemberLookup {
    /// No member exists.
    Missing,
    /// One structural field exists.
    Field(dir::GlobalTypeId),
    /// One or more declaration-backed members exist.
    Found(Vec<MemberCandidate>),
}

/// How one declaration member behaves at a use site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum MemberRole {
    /// Field members use regular assignability and project their own type.
    Field,
    /// Method members use receiver-free method assignability.
    Method,
    /// Getter members project their return type.
    Getter,
    /// Setter members accept their first parameter type.
    Setter,
    /// Associated members use regular assignability.
    Associated,
    /// Variant members carry a static constructor value.
    Variant,
}

/// One declaration member visible to member lookup.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct DeclaredMember {
    /// The declaring member symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The member space.
    pub(in crate::check) space: dir::MemberSpace,
    /// The member key.
    pub(in crate::check) key: Option<dir::StaticKey>,
    /// The member type when the declaration carries a type.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// The member static value when it carries one.
    pub(in crate::check) value: Option<dir::GlobalStaticId>,
    /// How the member behaves at a use site.
    pub(in crate::check) role: MemberRole,
}

impl MemberLookup {
    /// Return a lookup from collected candidates.
    pub(in crate::check) fn from_candidates(candidates: Vec<MemberCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Missing
        } else {
            Self::Found(candidates)
        }
    }

    /// Return one field type when the lookup names exactly one field.
    pub(in crate::check) fn field_type(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Field(ty) => Some(*ty),
            Self::Found(candidates) => match candidates.as_slice() {
                [candidate] if candidate.role == MemberRole::Field => Some(candidate.ty),
                _ => None,
            },
            Self::Missing => None,
        }
    }

    /// Return the first type exposed by this lookup.
    pub(in crate::check) fn value_type(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Field(ty) => Some(*ty),
            Self::Found(candidates) => candidates.first().map(|candidate| candidate.ty),
            Self::Missing => None,
        }
    }

    /// Return whether lookup found at least one member.
    pub(in crate::check) fn is_found(&self) -> bool {
        !matches!(self, Self::Missing)
    }
}

impl MemberRole {
    /// Return the use-site role of one definition member.
    pub(in crate::check) fn from_definition(member: &dir::DefinitionMember) -> Option<Self> {
        match member {
            dir::DefinitionMember::Field(_) => Some(Self::Field),
            dir::DefinitionMember::Method(method)
                if method.role == Some(dir::FunctionRole::Getter) =>
            {
                Some(Self::Getter)
            }
            dir::DefinitionMember::Method(method)
                if method.role == Some(dir::FunctionRole::Setter) =>
            {
                Some(Self::Setter)
            }
            dir::DefinitionMember::Method(_) => Some(Self::Method),
            dir::DefinitionMember::AssociatedType(_)
            | dir::DefinitionMember::AssociatedConst(_) => Some(Self::Associated),
            dir::DefinitionMember::Variant(_) => Some(Self::Variant),
            dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_)
            | dir::DefinitionMember::IndexSignature(_) => None,
        }
    }

    /// Return the relation one implementing member must hold against its requirement.
    pub(in crate::check) fn conformance_relation(self) -> Relation {
        match self {
            Self::Method | Self::Getter | Self::Setter => Relation::MethodAssignable,
            _ => Relation::Assignable,
        }
    }

    /// Return whether this role can be read by member access.
    pub(in crate::check) fn is_readable(self) -> bool {
        !matches!(self, Self::Setter)
    }
}

impl DeclaredMember {
    /// Return whether this member matches one lookup key.
    pub(in crate::check) fn matches(&self, space: dir::MemberSpace, key: dir::StaticKey) -> bool {
        self.space == space && self.key == Some(key)
    }

    /// Return the value type exposed by this member at a use site.
    pub(in crate::check) fn value_type(
        &self,
        check: &CheckState<'_>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match (self.role, check.ty(ty)?) {
            (MemberRole::Getter, dir::Type::FunctionSignature(function)) => {
                let function = check.type_signature(ty.module_id, function)?;

                Ok(function.return_type.unwrap_or(ty))
            }
            (MemberRole::Setter, dir::Type::FunctionSignature(function)) => {
                let function = check.type_signature(ty.module_id, function)?;

                Ok(check
                    .signature_parameters(ty.module_id, function.parameters)?
                    .first()
                    .map_or(ty, |parameter| parameter.ty))
            }
            _ => Ok(ty),
        }
    }
}

/// One declaration-backed member candidate.
#[derive(Debug, Clone)]
pub(in crate::check) struct MemberCandidate {
    /// The declaring member symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The declaration that exposed the member.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The member space that selected this candidate.
    pub(in crate::check) space: dir::MemberSpace,
    /// How the member behaves at a use site.
    pub(in crate::check) role: MemberRole,
    /// The substituted member type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The generic arguments matched through the owner.
    pub(in crate::check) generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The member static value when it carries one.
    pub(in crate::check) value: Option<dir::GlobalStaticId>,
    /// The substituted static value of the member, when it has one.
    pub(in crate::check) value_type: Option<dir::GlobalTypeId>,
    /// The projection steps lookup applied to the use site receiver.
    pub(in crate::check) steps: ReceiverSteps,
    /// The narrowed runtime receiver binding this candidate, like one union arm.
    pub(in crate::check) receiver: Option<dir::GlobalTypeId>,
}

impl MemberCandidate {
    /// Return the durable member candidate for this lookup candidate.
    pub(in crate::check) fn resolution_candidate(
        &self,
        receiver: dir::GlobalTypeId,
    ) -> Option<dir::MemberCandidate> {
        // dereferenced members expect the receiver their steps reach
        let receiver = self
            .steps
            .last()
            .map(dir::Projection::ty)
            .unwrap_or(self.receiver.unwrap_or(receiver));

        Some(dir::MemberCandidate {
            receiver,
            adjustments: self.steps.to_vec(),
            space: self.space,
            owner: self.owner,
            symbol: self.symbol?,
            ty: self.ty,
            generic_arguments: self.generic_arguments.clone(),
        })
    }

    /// Return the durable member resolution for this lookup candidate.
    pub(in crate::check) fn resolution(
        &self,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> dir::MemberResolution {
        let target = match self.resolution_candidate(receiver) {
            Some(candidate) => dir::MemberTarget::Symbol(candidate),
            None => dir::MemberTarget::Field(key),
        };

        dir::MemberResolution::new(receiver, target)
    }

    /// Return the stored field selected by this candidate.
    pub(in crate::check) fn field(&self, key: dir::StaticKey) -> Option<dir::ProjectionField> {
        if self.role != MemberRole::Field {
            return None;
        }

        Some(
            self.symbol
                .map(dir::ProjectionField::Member)
                .unwrap_or(dir::ProjectionField::Key(key)),
        )
    }

    /// Return the getter selected by this candidate.
    pub(in crate::check) fn getter(
        &self,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> Option<dir::MemberResolution> {
        if self.role != MemberRole::Getter {
            return None;
        }

        Some(self.resolution(receiver, key))
    }

    /// Return the setter selected by this candidate.
    pub(in crate::check) fn setter(
        &self,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> Option<dir::MemberResolution> {
        if self.role != MemberRole::Setter {
            return None;
        }

        Some(self.resolution(receiver, key))
    }
}

impl BodyState<'_, '_> {
    /// Return the lookup member represented by one definition member.
    pub(in crate::check) fn declared_member(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Answer<Option<DeclaredMember>>> {
        let Some(role) = MemberRole::from_definition(member) else {
            return Ok(Answer::Ready(None));
        };
        let ty = answer!(self.definition_member_type(member)?);

        Ok(Answer::Ready(Some(DeclaredMember {
            symbol: member.symbol(),
            space: member.space(),
            key: member.key(),
            ty,
            value: member.value(),
            role,
        })))
    }

    /// Select the member meaning of one member access node.
    pub(in crate::check) fn select_member(
        &mut self,
        site: FlowSite,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
        is_optional: bool,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        let Some(name) = name else {
            return Err(CompilerError::Internal {
                message: format!("member node {node:?} has no name"),
            });
        };
        let key = dir::StaticKey::Name(name);

        // reduce the receiver before member lookup
        let receiver_node = left.into_global_any(module);
        let receiver_site = self.node_site(receiver_node)?;
        let mut receiver = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
        if let Some(split) = answer!(self.split_nullish_type(origin, receiver)?) {
            // optional accesses short-circuit their nullish arm to the chain
            if !is_optional {
                self.report_possibly_nullish(origin, split.rejected.label().to_string())?;
            }
            receiver = split.value;
        }
        // diagnostics show the receiver as written, lookup reduces it
        let written_receiver = receiver;
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);

        // choose static or instance member space from the receiver expression
        let mut space = self.member_receiver_space(receiver_node, receiver)?;

        // static access names a declaration
        let mut lookup_receiver = receiver;
        let receiver_name = self
            .resolutions(receiver_node.module_id)
            .name_resolution(receiver_node)
            .cloned();
        if let Some(resolution) = receiver_name
            && let [symbol] = resolution.symbols()
        {
            let symbol = self.resolve_symbol_alias(*symbol)?;
            if self.symbol_kind(symbol).is_type_alias() {
                space = dir::MemberSpace::Static;
                lookup_receiver =
                    self.intern_type(module, dir::Type::Reference(dir::TypeReference { symbol }))?;
            }
        }
        let lookup = answer!(self.lookup_member(origin, module, lookup_receiver, space, key)?);

        match lookup {
            MemberLookup::Field(ty) => self.commit_field_member(node, receiver, key, ty),
            MemberLookup::Found(candidates) => {
                self.commit_member_candidates(node, origin, module, receiver, key, name, candidates)
            }
            MemberLookup::Missing => {
                let key = self.strings().get(name).to_string();

                self.reject_member(node, origin, written_receiver, key)
            }
        }
    }

    /// Commit one structural field member access.
    fn commit_field_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        // commit structural field target
        let target = dir::MemberTarget::Field(key);
        let resolution = dir::MemberResolution::new(receiver, target);
        self.commit_decision(node, Decision::Member(resolution))?;
        let site = self.node_site(node)?;
        let ty = answer!(self.flow_type_at(site, ty)?);
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Commit one declaration-backed member access.
    fn commit_member_candidates(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        name: dir::StringId,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<()>> {
        let candidates = candidates
            .into_iter()
            .collect::<SmallVec<[MemberCandidate; 2]>>();

        match candidates.as_slice() {
            // reject empty candidate sets
            [] => {
                let key = self.strings().get(name).to_string();

                self.reject_member(node, origin, receiver, key)
            }

            // commit one readable declaration member
            [candidate] if candidate.role.is_readable() => {
                let resolution = candidate.resolution(receiver, key);
                self.commit_decision(node, Decision::Member(resolution))?;
                let site = self.node_site(node)?;
                let ty = answer!(self.flow_type_at(site, candidate.ty)?);
                self.commit_node_type(node, ty)?;

                Ok(Answer::Ready(()))
            }

            // report single write-only declaration member
            [candidate] if candidate.role == MemberRole::Setter => {
                let key = self.strings().get(name).to_string();
                self.report_write_only_member(origin, key)?;
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                Ok(Answer::Ready(()))
            }

            // reject single unreadable declaration member
            [_candidate] => {
                let key = self.strings().get(name).to_string();

                self.reject_member(node, origin, receiver, key)
            }

            // commit overload or union candidate set
            many => self.commit_member_candidate_set(node, origin, module, receiver, name, many),
        }
    }

    /// Commit one multi-candidate member access.
    fn commit_member_candidate_set(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        name: dir::StringId,
        candidates: &[MemberCandidate],
    ) -> CompilerResult<Answer<()>> {
        // collect readable declaration candidates
        let is_union = matches!(self.ty(receiver)?, dir::Type::Union(_));
        let mut resolution_candidates = Vec::new();
        let mut types = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if !candidate.role.is_readable() {
                continue;
            }
            types.push(candidate.ty);
            let Some(candidate) = candidate.resolution_candidate(receiver) else {
                continue;
            };
            resolution_candidates.push(candidate);
        }

        // report sets that only expose write-only members
        let has_setter = candidates
            .iter()
            .any(|candidate| candidate.role == MemberRole::Setter);
        if resolution_candidates.is_empty() && has_setter {
            let key = self.strings().get(name).to_string();
            self.report_write_only_member(origin, key)?;
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        }

        // reject sets with no readable member
        if resolution_candidates.is_empty() {
            let key = self.strings().get(name).to_string();

            return self.reject_member(node, origin, receiver, key);
        }

        // commit the member candidate set
        let target = if is_union {
            dir::MemberTarget::Universal(resolution_candidates)
        } else {
            dir::MemberTarget::Existential(resolution_candidates)
        };
        let ty = self.normalized_union_type(module, types)?;
        let resolution = dir::MemberResolution::new(receiver, target);
        self.commit_decision(node, Decision::Member(resolution))?;
        let site = self.node_site(node)?;
        let ty = answer!(self.flow_type_at(site, ty)?);
        self.commit_node_type(node, ty)?;

        Ok(Answer::Ready(()))
    }

    /// Return the span of one member access key, when it is exact.
    fn member_key_span(&self, node: dir::GlobalNodeIdAny) -> Option<Span> {
        let state = self.module(node.module_id);
        if node.local_id.ty != dir::NodeType::Expression {
            return None;
        }
        let expression = node.local_id.into_typed::<dir::Expression>();
        let dir::Expression::Member {
            left, is_optional, ..
        } = *state.view().get(expression)
        else {
            return None;
        };
        let node_span = state.diagnostic_span(node.local_id)?;
        let left_span = state.diagnostic_span(left.into_any())?;
        if node_span.file != left_span.file {
            return None;
        }
        let start = left_span.end + if is_optional { 2 } else { 1 };
        (start < node_span.end).then(|| Span::new(node_span.file, start, node_span.end))
    }

    /// Reject one member access with a diagnostic.
    fn reject_member(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: String,
    ) -> CompilerResult<Answer<()>> {
        if self.type_flags(receiver)?.has_error() {
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        }
        let key_span = self.member_key_span(node);
        self.report_missing_member(origin, receiver, key, key_span)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}

impl BodyState<'_, '_> {
    /// Return the use-site type projected by one declaration member.
    pub(in crate::check) fn projected_member_type(
        &mut self,
        origin: Origin,
        receiver: Option<dir::GlobalTypeId>,
        role: MemberRole,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if role != MemberRole::Field {
            return Ok(Answer::Ready(ty));
        }
        let Some(receiver) = receiver else {
            return Ok(Answer::Ready(ty));
        };
        let ty = answer!(self.project_member_place(origin, receiver, ty)?);

        // readonly receivers project deep readonly views onto stored fields
        if !answer!(self.receiver_projects_readonly(origin, receiver)?) {
            return Ok(Answer::Ready(ty));
        }
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        if matches!(self.ty(ty)?, dir::Type::Form(form) if form.form == dir::Form::Readonly) {
            return Ok(Answer::Ready(ty));
        }
        if !answer!(self.type_projects_readonly(origin, ty)?) {
            return Ok(Answer::Ready(ty));
        }

        let projected = self.intern_type(
            origin.module(),
            dir::Type::Form(dir::FormType {
                form: dir::Form::Readonly,
                value: ty,
            }),
        )?;

        self.reduce_type_head(origin, projected)
    }

    /// Return whether one receiver projects stored fields as readonly.
    pub(in crate::check) fn receiver_projects_readonly(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut current = receiver;

        loop {
            current = answer!(self.reduce_type_head(origin, current)?);
            let form = match self.ty(current)? {
                dir::Type::Form(form) => form,
                _ => return Ok(Answer::Ready(false)),
            };

            // explicit readonly views make every stored field readonly
            if form.form == dir::Form::Readonly {
                return Ok(Answer::Ready(true));
            }

            // readonly borrows expose only readonly stored fields
            if let dir::Form::Borrowed(borrow) = form.form
                && let access = self.type_borrow(current.module_id, borrow)?.access
                && answer!(self.access_is_readonly(origin, access)?)
            {
                return Ok(Answer::Ready(true));
            }

            current = form.value;
        }
    }

    /// Project one type-level member access through its owner.
    pub(in crate::check) fn project_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = origin.module();
        let lookup = answer!(self.lookup_member(
            origin,
            module,
            member.owner,
            dir::MemberSpace::Static,
            member.key,
        )?);

        // qualified projections keep only their declaring scope's members
        let lookup = match (lookup, member.qualifier) {
            (MemberLookup::Found(candidates), Some(qualifier)) => {
                let mut kept = Vec::new();
                for candidate in candidates {
                    if answer!(self.candidate_declared_by(
                        origin,
                        &candidate,
                        qualifier,
                        member.owner
                    )?) {
                        kept.push(candidate);
                    }
                }

                // implementations shadow the scope's own abstract member
                let scope = self.type_symbol(qualifier)?;
                if let Some(scope) = scope
                    && kept.iter().any(|candidate| candidate.owner != scope)
                {
                    kept.retain(|candidate| candidate.owner != scope);
                }

                MemberLookup::Found(kept)
            }
            (lookup, _) => lookup,
        };

        // implementations shadow abstract associated members
        let lookup = match lookup {
            MemberLookup::Found(candidates) if candidates.len() > 1 => {
                let mut kept = Vec::with_capacity(candidates.len());
                for candidate in candidates {
                    let is_abstract = self.member_head(candidate.ty)?.is_some_and(|projected| {
                        projected.owner == member.owner && projected.key == member.key
                    });
                    if !is_abstract {
                        kept.push(candidate);
                    }
                }

                MemberLookup::from_candidates(kept)
            }
            lookup => lookup,
        };

        match lookup {
            // single projections substitute member arguments
            MemberLookup::Field(ty) => Ok(Answer::Ready(Some(ty))),
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    // comptime const projections yield their static values
                    if let Some(written) = candidate.value_type {
                        return Ok(Answer::Ready(Some(written)));
                    }
                    if let Some(value) = candidate.value {
                        let ty = self.intern_type(module, dir::Type::Static(value))?;

                        return Ok(Answer::Ready(Some(ty)));
                    }

                    // projections over abstract owners are rigid
                    if let Some(projected) = self.member_head(candidate.ty)?
                        && projected.owner == member.owner
                        && projected.key == member.key
                    {
                        return Ok(Answer::Ready(None));
                    }

                    Ok(Answer::Ready(Some(candidate.ty)))
                }
                _ => Ok(Answer::Ready(None)),
            },
            MemberLookup::Missing => self.project_heritage_default(origin, member),
        }
    }

    /// Project one defaulted interface member through the owner's heritage.
    fn project_heritage_default(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let owner = self.settled_root(member.owner)?;
        let dir::Type::Instance(instance) = self.ty(owner)? else {
            return Ok(Answer::Ready(None));
        };
        let implements = match self.definition(instance.symbol)? {
            Some(dir::Definition::Class(definition)) => definition.implements.clone(),
            Some(dir::Definition::Struct(definition)) => definition.implements.clone(),
            _ => return Ok(Answer::Ready(None)),
        };

        // instantiate each implemented interface with the owner's arguments
        let owner_substitution = self.instance_substitution(owner.module_id, &instance)?;
        for heritage in implements {
            let applied = answer!(self.substituted_heritage(
                origin,
                owner.module_id,
                &owner_substitution,
                &heritage
            )?);
            let requirements =
                answer!(self.interface_requirements(origin, origin.module(), &applied, owner,)?);
            for required in requirements.members {
                if !required.has_default || required.key != member.key {
                    continue;
                }
                if let Some(ty) = required.ty {
                    return Ok(Answer::Ready(Some(ty)));
                }
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Return the unique bound application declaring one projected member.
    pub(in crate::check) fn projection_qualifier(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut qualifier = None;
        for bound in self.parameter_bounds(origin, parameter)? {
            // only interface bounds can declare projected members
            let bound = answer!(self.reduce_type_head(origin, bound)?);
            let dir::Type::Instance(instance) = self.ty(bound)? else {
                continue;
            };
            let members = match self.definition(instance.symbol)? {
                Some(dir::Definition::Interface(definition)) => definition.members.clone(),
                _ => continue,
            };

            // remember the declaring interface, rejecting ambiguity
            for member in &members {
                let declared = match self.declared_member(member)? {
                    Answer::Ready(Some(declared)) => declared,
                    Answer::Ready(None) => continue,
                    Answer::Pending(pending) => return Ok(Answer::Pending(pending)),
                };
                if !declared.matches(dir::MemberSpace::Static, key) {
                    continue;
                }
                if qualifier.is_some_and(|previous| previous != bound) {
                    return Ok(Answer::Ready(None));
                }
                qualifier = Some(bound);
            }
        }

        Ok(Answer::Ready(qualifier))
    }

    /// Return the substituted constraint declared for one rigid projection.
    pub(in crate::check) fn projection_constraint(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let owner = self.value_beneath_forms(origin, member.owner)?;
        let dir::Type::Parameter(parameter) = self.ty(owner)? else {
            return Ok(Answer::Ready(None));
        };

        for bound in self.parameter_bounds(origin, parameter)? {
            // only interface bounds can declare projected members
            let bound = answer!(self.reduce_type_head(origin, bound)?);
            let dir::Type::Instance(instance) = self.ty(bound)? else {
                continue;
            };
            let members = match self.definition(instance.symbol)? {
                Some(dir::Definition::Interface(definition)) => definition.members.clone(),
                _ => continue,
            };

            // read the declared constraint under the bound's application
            for declared in &members {
                let dir::DefinitionMember::AssociatedType(associated) = declared else {
                    continue;
                };
                if associated.key != member.key {
                    continue;
                }
                let Some(constraint) = associated.constraint else {
                    continue;
                };
                let substitution = self
                    .instance_substitution(bound.module_id, &instance)?
                    .with_receiver(owner);
                let constraint =
                    self.substitute_type(origin.module(), constraint, &substitution)?;

                return Ok(Answer::Ready(Some(constraint)));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Decide whether one candidate projects through one qualifying scope.
    fn candidate_declared_by(
        &mut self,
        origin: Origin,
        candidate: &MemberCandidate,
        qualifier: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let instance = match self.ty(qualifier)? {
            // extension scopes bind lexically to their declaration
            dir::Type::Reference(reference) => {
                return Ok(Answer::Ready(candidate.owner == reference.symbol));
            }
            dir::Type::Instance(instance) => instance,
            _ => return Ok(Answer::Ready(false)),
        };

        // the declaring interface carries its own abstract members
        if candidate.owner == instance.symbol {
            return Ok(Answer::Ready(true));
        }
        let heritages = match self.definition(candidate.owner)? {
            Some(definition) => definition
                .heritages()
                .into_iter()
                .filter(|heritage| heritage.symbol == instance.symbol)
                .cloned()
                .collect::<Vec<_>>(),
            None => return Ok(Answer::Ready(false)),
        };

        // match any implemented application against the applied scope,
        //  binding the implementer's own parameters as pattern holes
        let module = origin.module();
        let parameters = self
            .symbol_template(candidate.owner)?
            .map(|template| self.generic_template_parameters(template))
            .unwrap_or_default();
        let receiver_only = TypeSubstitution::default().with_receiver(receiver);
        for heritage in heritages {
            // resolve receiver-relative heritage arguments at this application
            let mut arguments = heritage.arguments.clone();
            for argument in arguments.iter_mut() {
                *argument = self.substitute_type(module, *argument, &receiver_only)?;
            }
            let arguments = self.intern_type_ids(module, &arguments)?;
            let pattern = self.intern_type(
                module,
                dir::Type::Instance(dir::GenericInstance {
                    symbol: heritage.symbol,
                    arguments,
                }),
            )?;
            let matched =
                answer!(self.match_generic_pattern(origin, &parameters, pattern, qualifier)?);
            if matched.is_some() {
                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Return one field type projected through the receiver placement.
    fn project_member_place(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let Some(place) = answer!(self.receiver_projected_place(origin, receiver)?) else {
            return Ok(Answer::Ready(ty));
        };

        // bare members of local receivers stay bare
        let place_root = self.settled_root(place)?;
        if self.check.is_memory_component(place_root, "local")? {
            return Ok(Answer::Ready(ty));
        }

        let ty = self.resolve_relative_place(origin, ty, place)?;

        self.reduce_type_head(origin, ty)
    }

    /// Return the place projected by one receiver type.
    pub(in crate::check) fn receiver_projected_place(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let mut current = receiver;
        loop {
            current = answer!(self.reduce_type_head(origin, current)?);
            let dir::Type::Form(form) = self.ty(current)? else {
                // bare nominal instances live in their declared or inherited space
                if let dir::Type::Instance(instance) = self.ty(current)?
                    && let Some(space) = self.check.nominal_space(instance.symbol)?
                {
                    let place = self.intern_type(
                        current.module_id,
                        dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(space))),
                    )?;

                    return Ok(Answer::Ready(Some(place)));
                }

                return Ok(Answer::Ready(None));
            };

            match form.form {
                dir::Form::Placed { place } => return Ok(Answer::Ready(Some(place))),
                _ => current = form.value,
            }
        }
    }

    /// Return whether one projected value should retain a readonly view.
    fn type_projects_readonly(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        if answer!(self.satisfies_auto_interface(origin, ty, dir::AutoInterface::Copy)?) {
            return Ok(Answer::Ready(false));
        }

        // unions project a view only when one element requires it
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);
            for element in elements {
                if answer!(self.type_projects_readonly(origin, element)?) {
                    return Ok(Answer::Ready(true));
                }
            }

            return Ok(Answer::Ready(false));
        }

        let result = match self.ty(ty)? {
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Intrinsic
            | dir::Type::This
            | dir::Type::Memory(_)
            | dir::Type::Static(_) => false,
            dir::Type::Union(_)
            | dir::Type::Refined(_)
            | dir::Type::Object
            | dir::Type::Reference(_)
            | dir::Type::Instance(_)
            | dir::Type::Member(_)
            | dir::Type::EnumMember(_)
            | dir::Type::Form(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Operation(_)
            | dir::Type::Array(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Range(_)
            | dir::Type::Slice(_)
            | dir::Type::Tuple(_)
            | dir::Type::Shape(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Variable(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Intersection(_) => true,
        };

        Ok(Answer::Ready(result))
    }

    /// Return whether one memory access component is readonly.
    pub(in crate::check) fn access_is_readonly(
        &mut self,
        origin: Origin,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let access = answer!(self.reduce_type_head(origin, access)?);
        let is_readonly = matches!(
            self.ty(access)?,
            dir::Type::Memory(dir::MemoryLiteral::Access(dir::Access::Readonly))
        );

        Ok(Answer::Ready(is_readonly))
    }
}
