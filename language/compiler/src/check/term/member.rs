use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CandidateResolution, CheckState, GenericArgument, GenericParameterId, GenericSubstitution,
    MemberDecision, MemberFailure, MemberLookup, MemberResolution, MemberTargetResolution, Origin,
    ShapeMember, StaticTerm, Substitution, TermId, TupleElement, TypeOperand, TypeOperationTerm,
    TypeRelation, TypeTerm, VariableId,
};

use crate::check::{Decision, Reduction};

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
        substitution: Substitution<'_>,
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
        let Some(owner) = self.type_operand_term(owner)? else {
            return Ok(None);
        };
        let owner = match self.reduce_type_term(origin, &owner)? {
            Reduction {
                value: Some(value),
                progress: _,
            } => value,
            Reduction {
                value: None,
                progress: _,
            } => owner,
        };

        // select solved member for commit and diagnostics
        if let Origin::Node(source) = source {
            match self.lookup_type_member(origin, module, &owner, &key)? {
                MemberLookup::Pending => {}
                MemberLookup::Found(mut members) => {
                    let target = if members.len() == 1 {
                        let member = members.remove(0);

                        MemberTargetResolution::Symbol {
                            symbol: member.symbol,
                            instance: member.instance,
                        }
                    } else {
                        let members = members
                            .into_iter()
                            .map(|member| CandidateResolution {
                                symbol: member.symbol,
                                instance: member.instance,
                            })
                            .collect();

                        MemberTargetResolution::Union(members)
                    };
                    let member = MemberResolution {
                        source,
                        receiver,
                        target,
                    };

                    self.select_member(source, MemberDecision::Resolved(member))?;
                }
                MemberLookup::Missing if self.type_term_has_field(module, &owner, &key)? => {
                    let member = MemberResolution {
                        source,
                        receiver,
                        target: MemberTargetResolution::Field(key),
                    };

                    self.select_member(source, MemberDecision::Resolved(member))?;
                }
                MemberLookup::Missing => {
                    self.select_member(
                        source,
                        MemberDecision::Rejected(MemberFailure::Missing { key }),
                    )?;
                }
            }
        }

        self.resolve_member_type(origin, module, &owner, &key, arguments)
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
        match term {
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(None);
                };

                self.resolve_member_type(origin, module, &term, key, member_arguments)
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if let Some(parameter) = self.type_generic_parameter_for_symbol(*symbol)? {
                    self.parameter_member_type(module, origin, parameter, key, member_arguments)
                } else {
                    self.instantiate_symbol_member_type(
                        origin,
                        module,
                        *symbol,
                        arguments,
                        *key,
                        member_arguments,
                    )
                }
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.instantiate_symbol_member_type(
                origin,
                module,
                *symbol,
                arguments,
                *key,
                member_arguments,
            ),
            TypeTerm::Tuple { elements, .. } if member_arguments.is_empty() => {
                self.tuple_member_type(elements, key)
            }
            TypeTerm::Shape(shape) if member_arguments.is_empty() => {
                self.shape_member_type(&self.inference.term(*shape).members, key)
            }
            TypeTerm::Union { elements } if member_arguments.is_empty() => {
                self.union_member_type(origin, module, elements, key)
            }
            TypeTerm::Operation(operation) if member_arguments.is_empty() => {
                self.operation_member_type(origin, module, *operation, key)
            }
            TypeTerm::Parameter(parameter) => {
                self.parameter_member_type(module, origin, *parameter, key, member_arguments)
            }
            _ => Ok(None),
        }
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

    /// Return a member type through a generic parameter constraint.
    fn parameter_member_type(
        &mut self,
        module: ModuleId,
        origin: Origin,
        parameter: GenericParameterId,
        key: &dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(constraint) = self.type_generic_constraint(parameter)? else {
            return Ok(None);
        };
        let Some(term) = self.reduced_generic_type_constraint(origin, constraint)? else {
            return Ok(None);
        };

        self.resolve_member_type(origin, module, &term, key, member_arguments)
    }

    /// Return the reduced type term for one generic constraint variable.
    pub(in crate::check) fn reduced_generic_type_constraint(
        &mut self,
        origin: Origin,
        constraint: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(solution) = self.type_operand_term(constraint)? else {
            return Ok(None);
        };

        // expose transparent aliases before member lookup
        let reduction = self.reduce_type_term(origin, &solution)?;
        let term = match reduction.value {
            Some(reduced) => reduced,
            None => solution,
        };

        Ok(Some(term))
    }

    /// Return a member static from one nominal declaration.
    fn symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        let members = self.member_symbols(symbol, key);
        let [member] = members.as_slice() else {
            return Ok(None);
        };
        let substitution = GenericSubstitution::empty();
        if self.reduce_symbol_availability(module, *member, (&substitution).into())?
            != Decision::Yes
        {
            return Ok(None);
        }
        let operand = self.import_symbol_static_operand(module, *member)?;

        self.static_operand_term(operand)
    }

    /// Instantiate a member type from one applied nominal declaration.
    fn instantiate_symbol_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let mut members = Vec::new();

        // collect inherent member types
        for member in self.member_symbols(symbol, key) {
            let substitution = self.generic_substitution(symbol, arguments)?;
            if self.reduce_symbol_availability(module, member, (&substitution).into())?
                != Decision::Yes
            {
                continue;
            }
            let operand = self.import_symbol_type_operand(module, member)?;
            let Some(mut term) = self.type_operand_term(operand)? else {
                return Ok(None);
            };

            if !substitution.is_empty() {
                let Some(substituted) = term.substitute(module, &substitution, self)? else {
                    return Ok(None);
                };

                term = substituted;
            }

            let Some(term) =
                self.instantiate_member_type(module, member, member_arguments, term)?
            else {
                return Ok(None);
            };

            members.push(term);
        }

        // collect extension member types
        for (_, member, substitution) in
            self.lookup_extension_members(origin, module, symbol, arguments, key)?
        {
            if self.reduce_symbol_availability(module, member, (&substitution).into())?
                != Decision::Yes
            {
                continue;
            }
            let operand = self.import_symbol_type_operand(module, member)?;
            let Some(mut term) = self.type_operand_term(operand)? else {
                return Ok(None);
            };

            if !substitution.is_empty() {
                let Some(substituted) = term.substitute(module, &substitution, self)? else {
                    return Ok(None);
                };

                term = substituted;
            }

            let Some(term) =
                self.instantiate_member_type(module, member, member_arguments, term)?
            else {
                return Ok(None);
            };

            members.push(term);
        }

        match members.as_slice() {
            [] => Ok(None),
            [member] => Ok(Some(member.clone())),
            _ => {
                let elements = members
                    .into_iter()
                    .map(|member| self.inference.push_term(member).into())
                    .collect();

                Ok(Some(TypeTerm::Union { elements }))
            }
        }
    }

    /// Instantiate generic arguments declared by the selected member.
    fn instantiate_member_type(
        &mut self,
        module: ModuleId,
        member: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        term: TypeTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let substitution = self.generic_substitution(member, arguments)?;
        if substitution.is_empty() {
            return Ok(Some(term));
        }

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
        let members = self.member_symbols(symbol, key);
        let [member] = members.as_slice() else {
            return Ok(None);
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        if self.reduce_symbol_availability(module, *member, (&substitution).into())?
            != Decision::Yes
        {
            return Ok(None);
        }
        let operand = self.import_symbol_static_operand(module, *member)?;
        let Some(term) = self.static_operand_term(operand)? else {
            return Ok(None);
        };
        if substitution.is_empty() {
            return Ok(Some(term));
        }

        term.substitute(module, (&substitution).into(), self)
            .map(Some)
    }

    /// Return the generic slot id for one type parameter symbol.
    pub(in crate::check) fn type_generic_parameter_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(slot) = self.inference.symbol_generic_parameter(symbol) else {
            return Ok(None);
        };
        let generic = self.inference.generic_parameter(slot);
        if !generic.is_type() {
            return Ok(None);
        }

        Ok(Some(slot))
    }

    /// Return the type constraint for one generic type slot.
    pub(in crate::check) fn type_generic_constraint(
        &self,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<Option<TypeOperand>> {
        let generic = self.inference.generic_parameter(parameter_id);
        if !generic.is_type() {
            return Ok(None);
        }

        let constraint = generic.type_constraint();

        Ok(constraint)
    }

    /// Return a member type from one check shape term.
    fn shape_member_type(
        &self,
        members: &[ShapeMember],
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        for member in members {
            let member = member;
            let ShapeMember::Field {
                key: member_key,
                ty,
                ..
            } = member
            else {
                continue;
            };
            if member_key.matches(key) {
                return self.type_operand_term(*ty);
            }
        }

        Ok(None)
    }

    /// Return the member type shared by every union element.
    fn union_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        elements: &[TypeOperand],
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let mut members = Vec::with_capacity(elements.len());

        // collect one member type per union element
        for element in elements {
            let Some(term) = self.type_operand_term(*element)? else {
                return Ok(None);
            };
            let Some(member) = self.resolve_member_type(origin, module, &term, key, &[])? else {
                return Ok(None);
            };

            members.push(member);
        }
        if members.len() == 1 {
            return Ok(members.pop());
        }
        let members = members
            .into_iter()
            .map(|member| self.inference.push_term(member).into())
            .collect();

        Ok(Some(TypeTerm::Union { elements: members }))
    }

    /// Return the symbol-backed member candidates shared by every union element.
    pub(in crate::check) fn union_member_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        elements: &[TypeOperand],
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut candidates = Vec::with_capacity(elements.len());

        // collect one member candidate lookup per union element
        for element in elements {
            let Some(term) = self.type_operand_term(*element)? else {
                return Ok(MemberLookup::Pending);
            };
            match self.lookup_type_member_matching(origin, module, &term, key)? {
                MemberLookup::Pending => return Ok(MemberLookup::Pending),
                MemberLookup::Missing => return Ok(MemberLookup::Missing),
                MemberLookup::Found(mut members) => {
                    candidates.append(&mut members);
                }
            };
        }
        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Return a member type from one type operation.
    fn operation_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operation: TermId<TypeOperationTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        match self.inference.term(operation).clone() {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                self.conditional_member_type(origin, module, left, right, then_type, else_type, key)
            }
            _ => Ok(None),
        }
    }

    /// Return member candidates from one type operation.
    pub(in crate::check) fn operation_member_candidates(
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
            } => self.conditional_member_candidates(
                origin, module, left, right, then_type, else_type, key,
            ),
            _ => Ok(MemberLookup::Missing),
        }
    }

    /// Return a member type from one conditional type operation.
    fn conditional_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;

        // select the known branch
        if decision == Decision::Yes {
            let Some(then_type) = self.type_operand_term(then_type)? else {
                return Ok(None);
            };

            return self.resolve_member_type(origin, module, &then_type, key, &[]);
        }
        if decision == Decision::No {
            let Some(else_type) = self.type_operand_term(else_type)? else {
                return Ok(None);
            };

            return self.resolve_member_type(origin, module, &else_type, key, &[]);
        }
        let Some(then_type) = self.type_operand_term(then_type)? else {
            return Ok(None);
        };
        let Some(else_type) = self.type_operand_term(else_type)? else {
            return Ok(None);
        };
        let Some(then_member) = self.resolve_member_type(origin, module, &then_type, key, &[])?
        else {
            return Ok(None);
        };
        let Some(else_member) = self.resolve_member_type(origin, module, &else_type, key, &[])?
        else {
            return Ok(None);
        };
        if then_member == else_member {
            return Ok(Some(then_member));
        }
        let then_member = self.inference.push_term(then_member);
        let else_member = self.inference.push_term(else_member);

        Ok(Some(TypeTerm::Union {
            elements: vec![then_member.into(), else_member.into()],
        }))
    }

    /// Return member candidates from one conditional type operation.
    fn conditional_member_candidates(
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

        // select the known branch
        if decision == Decision::Yes {
            let Some(then_type) = self.type_operand_term(then_type)? else {
                return Ok(MemberLookup::Pending);
            };

            return self.lookup_type_member_matching(origin, module, &then_type, key);
        }
        if decision == Decision::No {
            let Some(else_type) = self.type_operand_term(else_type)? else {
                return Ok(MemberLookup::Pending);
            };

            return self.lookup_type_member_matching(origin, module, &else_type, key);
        }
        let Some(then_type) = self.type_operand_term(then_type)? else {
            return Ok(MemberLookup::Pending);
        };
        let Some(else_type) = self.type_operand_term(else_type)? else {
            return Ok(MemberLookup::Pending);
        };
        let then_candidates = self.lookup_type_member_matching(origin, module, &then_type, key)?;
        let else_candidates = self.lookup_type_member_matching(origin, module, &else_type, key)?;
        let (mut then_candidates, mut else_candidates) = match (then_candidates, else_candidates) {
            (MemberLookup::Pending, _) | (_, MemberLookup::Pending) => {
                return Ok(MemberLookup::Pending);
            }
            (MemberLookup::Missing, _) | (_, MemberLookup::Missing) => {
                return Ok(MemberLookup::Missing);
            }
            (MemberLookup::Found(then_candidates), MemberLookup::Found(else_candidates)) => {
                (then_candidates, else_candidates)
            }
        };

        then_candidates.append(&mut else_candidates);
        Ok(MemberLookup::from_candidates(then_candidates))
    }

    /// Return a member type from one tuple term.
    fn tuple_member_type(
        &self,
        elements: &[TupleElement],
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(index) = self.tuple_member_index(key) else {
            return Ok(None);
        };
        let Some(element) = elements.get(index) else {
            return Ok(None);
        };

        let Some(ty) = self.type_operand_term(element.ty)? else {
            return Ok(None);
        };

        Ok(Some(ty))
    }

    /// Return the tuple index selected by one member key.
    fn tuple_member_index(&self, key: &dir::StaticKey) -> Option<usize> {
        let dir::StaticKey::Index(index) = key else {
            return None;
        };

        Some(*index)
    }

    /// Return whether one reduced type has a structural field.
    fn type_term_has_field(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<bool> {
        match term {
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(false);
                };

                self.type_term_has_field(module, &term, key)
            }
            TypeTerm::Shape(shape) => Ok(self
                .shape_member_type(&self.inference.term(*shape).members, key)?
                .is_some()),
            TypeTerm::Union { elements } => {
                for element in elements {
                    let Some(term) = self.type_operand_term(*element)? else {
                        return Ok(false);
                    };
                    if !self.type_term_has_field(module, &term, key)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
