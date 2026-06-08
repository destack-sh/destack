use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CandidateResolution, CheckState, GenericArgument, GenericParameterId, MemberDecision,
    MemberFailure, MemberLookup, MemberResolution, MemberTargetResolution, Origin, StaticTerm,
    SubstitutionSet, TermId, TypeOperand, TypeOperationTerm, TypeRelation, TypeTerm, VariableId,
};

use crate::check::Decision;

/// Type member projection term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberTerm {
    /// The work origin that introduced this member projection.
    pub(in crate::check) origin: Origin,
    /// The owner type.
    pub(in crate::check) owner: TypeOperand,
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The applied static arguments.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 2]>,
}

impl MemberTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 2]> {
        let mut variables = smallvec::SmallVec::new();

        variables.extend(self.owner.referenced_variables(state));
        variables.extend(
            self.arguments
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );

        variables
    }

    /// Substitute generic arguments through this member projection.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let member = Self {
            origin: self.origin,
            owner: state.substitute_type_operand(module, substitution, self.owner)?,
            key: self.key,
            arguments: state.substitute_arguments(module, substitution, &self.arguments)?,
        };

        Ok(member)
    }
}

impl CheckState<'_> {
    /// Reduce one member projection to its type.
    pub(in crate::check) fn reduce_member_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: Origin,
        owner: TypeOperand,
        key: dir::StaticKey,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let receiver = owner;
        let Some(owner) = self.reduce_type_operand(origin, owner)? else {
            return Ok(None);
        };

        let lookup = self.resolve_type_member(origin, module, owner, &key)?;

        // select solved member for commit and diagnostics
        if let Origin::Node(source) = source
            && self.inference.member(source).is_none()
        {
            self.select_member_resolution(source, receiver, key, &lookup)?;
        }

        self.member_type_from_lookup(module, arguments, lookup)
    }

    /// Resolve a member type from one reduced type term.
    pub(in crate::check) fn resolve_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let receiver = self.inference.push_term(term.clone());
        let lookup = self.resolve_type_member(origin, module, receiver.into(), key)?;

        self.member_type_from_lookup(module, member_arguments, lookup)
    }

    /// Return a member type from a resolved member lookup.
    fn member_type_from_lookup(
        &mut self,
        module: ModuleId,
        member_arguments: &[GenericArgument],
        lookup: MemberLookup,
    ) -> CompilerResult<Option<TypeTerm>> {
        match lookup {
            MemberLookup::Pending => Ok(None),
            MemberLookup::Field(member) => {
                if member_arguments.is_empty() {
                    self.type_operand_term(member)
                } else {
                    Ok(None)
                }
            }
            MemberLookup::Found(candidates) => {
                self.member_candidate_types(module, candidates, member_arguments)
            }
            MemberLookup::Missing => Ok(None),
        }
    }

    /// Select the committed member resolution for one lookup.
    fn select_member_resolution(
        &mut self,
        source: dir::GlobalNodeIdAny,
        receiver: TypeOperand,
        key: dir::StaticKey,
        lookup: &MemberLookup,
    ) -> CompilerResult<()> {
        match lookup {
            // wait for a later solve step
            MemberLookup::Pending => Ok(()),
            // select structural fields
            MemberLookup::Field(_) => {
                let member = MemberResolution {
                    source,
                    receiver,
                    target: MemberTargetResolution::Field(key),
                };

                self.inference
                    .select_member(source, MemberDecision::Resolved(member))
            }
            // select symbol-backed member candidates
            MemberLookup::Found(candidates) => {
                let target = self.member_target_resolution(candidates);
                let member = MemberResolution {
                    source,
                    receiver,
                    target,
                };

                self.inference
                    .select_member(source, MemberDecision::Resolved(member))
            }
            // report missing member candidates
            MemberLookup::Missing => self.inference.select_member(
                source,
                MemberDecision::Rejected(MemberFailure::Missing { key }),
            ),
        }
    }

    /// Return the committed target from resolved member candidates.
    fn member_target_resolution(
        &self,
        candidates: &[crate::check::MemberCandidate],
    ) -> MemberTargetResolution {
        if candidates.len() == 1 {
            let candidate = &candidates[0];

            return MemberTargetResolution::Symbol {
                symbol: candidate.symbol,
                instance: candidate.instance.clone(),
            };
        }

        let candidates = candidates
            .iter()
            .map(|candidate| CandidateResolution {
                symbol: candidate.symbol,
                instance: candidate.instance.clone(),
            })
            .collect();

        MemberTargetResolution::Union(candidates)
    }

    /// Return a member static from one reduced type term.
    pub(in crate::check) fn member_static_term(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        match term {
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(None);
                };

                self.member_static_term(module, &term, key)
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_member_static(module, *symbol, *key),
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.instantiate_symbol_member_static(module, *symbol, arguments, *key),
            _ => Ok(None),
        }
    }

    /// Return the reduced type operand for one generic constraint variable.
    pub(in crate::check) fn reduced_generic_type_constraint(
        &mut self,
        origin: Origin,
        constraint: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        self.reduce_type_operand(origin, constraint)
    }

    /// Return a member static from one nominal declaration.
    fn symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        let Some(definition) = self.definition(module, symbol)? else {
            return Ok(None);
        };
        let members = definition.static_members(&key);
        let [member] = members.as_slice() else {
            return Ok(None);
        };
        let substitution = SubstitutionSet::empty();
        if self.reduce_symbol_availability(module, member.symbol, &substitution)? != Decision::Yes {
            return Ok(None);
        }

        self.static_operand_term(member.value)
    }

    /// Return the type terms carried by resolved member candidates.
    fn member_candidate_types(
        &mut self,
        module: ModuleId,
        candidates: Vec<crate::check::MemberCandidate>,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let mut members = Vec::with_capacity(candidates.len());

        // apply member generic arguments to every candidate
        for candidate in candidates {
            let Some(member) = self.member_candidate_type(module, candidate, arguments)? else {
                return Ok(None);
            };

            members.push(member);
        }

        if members.len() == 1 {
            return Ok(members.pop());
        }

        let elements = members
            .into_iter()
            .map(|member| self.inference.push_term(member).into())
            .collect();

        Ok(Some(TypeTerm::Union { elements }))
    }

    /// Return the type term carried by one resolved member candidate.
    fn member_candidate_type(
        &mut self,
        module: ModuleId,
        candidate: crate::check::MemberCandidate,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let substitution = self.generic_substitution(candidate.symbol, arguments)?;
        if substitution.is_empty() {
            return self.type_operand_term(candidate.ty);
        }

        let Some(term) = self.type_operand_term(candidate.ty)? else {
            return Ok(None);
        };

        term.substitute(module, &substitution, self)
    }

    /// Instantiate a member static from one applied nominal declaration.
    fn instantiate_symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        let Some(definition) = self.definition(module, symbol)? else {
            return Ok(None);
        };
        let members = definition.static_members(&key);
        let [member] = members.as_slice() else {
            return Ok(None);
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        if self.reduce_symbol_availability(module, member.symbol, &substitution)? != Decision::Yes {
            return Ok(None);
        }
        let Some(term) = self.static_operand_term(member.value)? else {
            return Ok(None);
        };
        if substitution.is_empty() {
            return Ok(Some(term));
        }

        term.substitute(module, &substitution, self).map(Some)
    }

    /// Return the type constraint for one generic type parameter.
    pub(in crate::check) fn type_generic_constraint(
        &self,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<Option<TypeOperand>> {
        let generic = self.inference.require_generic_parameter(parameter_id);
        if !generic.is_type() {
            return Ok(None);
        }

        let constraint = generic.type_constraint();

        Ok(constraint)
    }

    /// Return the member lookup shared by every union element.
    pub(in crate::check) fn union_member_lookup(
        &mut self,
        origin: Origin,
        module: ModuleId,
        elements: &[TypeOperand],
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut candidates = Vec::with_capacity(elements.len());
        let mut fields = Vec::with_capacity(elements.len());

        // collect one member lookup per union element
        for element in elements {
            match self.resolve_type_member(origin, module, *element, key)? {
                MemberLookup::Pending => return Ok(MemberLookup::Pending),
                MemberLookup::Missing => return Ok(MemberLookup::Missing),
                MemberLookup::Found(mut members) => {
                    candidates.append(&mut members);
                }
                MemberLookup::Field(member) => fields.push(member),
            };
        }

        // return symbol-backed members when every branch had symbols
        if fields.is_empty() {
            return Ok(MemberLookup::from_candidates(candidates));
        }

        // return a structural union when every branch had a field
        if candidates.is_empty() {
            return Ok(self.union_field_lookup(fields));
        }

        // return an effective field when branch origins are mixed
        let mut fields = fields;
        fields.extend(candidates.into_iter().map(|candidate| candidate.ty));

        Ok(self.union_field_lookup(fields))
    }

    /// Return a union lookup from structural field types.
    fn union_field_lookup(&mut self, fields: Vec<TypeOperand>) -> MemberLookup {
        let mut fields = fields;
        if fields.len() == 1 {
            let field = fields.remove(0);

            return MemberLookup::Field(field);
        }

        let field = self
            .inference
            .push_term(TypeTerm::Union { elements: fields });

        MemberLookup::Field(field.into())
    }

    /// Return member lookup from one type operation.
    pub(in crate::check) fn operation_member_lookup(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operation: TermId<TypeOperationTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        match self.inference.term(operation).clone() {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => self
                .conditional_member_lookup(origin, module, left, right, then_type, else_type, key),
            _ => Ok(MemberLookup::Missing),
        }
    }

    /// Return member lookup from one conditional type operation.
    fn conditional_member_lookup(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;

        match decision {
            // use the known true branch
            Decision::Yes => self.resolve_type_member(origin, module, then_type, key),
            // use the known false branch
            Decision::No => self.resolve_type_member(origin, module, else_type, key),
            // merge both possible branches
            Decision::Undecidable => {
                let then_lookup = self.resolve_type_member(origin, module, then_type, key)?;
                let else_lookup = self.resolve_type_member(origin, module, else_type, key)?;

                Ok(self.merge_member_lookups(then_lookup, else_lookup))
            }
        }
    }

    /// Return a lookup from two conditional branches.
    fn merge_member_lookups(&mut self, left: MemberLookup, right: MemberLookup) -> MemberLookup {
        match (left, right) {
            // preserve pending branches
            (MemberLookup::Pending, _) | (_, MemberLookup::Pending) => MemberLookup::Pending,
            // require both branches to define the member
            (MemberLookup::Missing, _) | (_, MemberLookup::Missing) => MemberLookup::Missing,
            // merge symbol-backed branches
            (MemberLookup::Found(mut left), MemberLookup::Found(mut right)) => {
                left.append(&mut right);

                MemberLookup::from_candidates(left)
            }
            // merge structural branches
            (MemberLookup::Field(left), MemberLookup::Field(right)) => {
                self.merge_field_lookups(left, right)
            }
            // merge symbol and field branches as an effective field
            (MemberLookup::Found(candidates), MemberLookup::Field(member))
            | (MemberLookup::Field(member), MemberLookup::Found(candidates)) => {
                self.merge_symbol_and_field_lookup(candidates, member)
            }
        }
    }

    /// Return a lookup from two structural field types.
    fn merge_field_lookups(&mut self, left: TypeOperand, right: TypeOperand) -> MemberLookup {
        if left == right {
            return MemberLookup::Field(left);
        }

        let field = self.inference.push_term(TypeTerm::Union {
            elements: vec![left, right],
        });

        MemberLookup::Field(field.into())
    }

    /// Return a lookup from symbol candidates and one structural field.
    fn merge_symbol_and_field_lookup(
        &mut self,
        candidates: Vec<crate::check::MemberCandidate>,
        member: TypeOperand,
    ) -> MemberLookup {
        let mut members = candidates
            .into_iter()
            .map(|candidate| candidate.ty)
            .collect::<Vec<_>>();

        members.push(member);

        self.union_field_lookup(members)
    }
}
