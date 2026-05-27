use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, FunctionTerm, GenericArgument, GenericInstance, GenericSlotId, GenericSubstitution,
    MemberDecision, MemberFailure, MemberProtocol, MemberResolution, MemberResolutionTarget,
    ShapeMember, StaticTerm, TermId, TupleElement, TypeOperand, TypeRelation, TypeTerm, VariableId,
    VariableOutput,
};

use crate::check::{Decision, Reduction};

/// Type member projection term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberTerm {
    /// The source member expression when this term came from runtime syntax.
    pub(in crate::check) source: Option<dir::GlobalNodeIdAny>,
    /// The owner type.
    pub(in crate::check) owner: VariableId,
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The applied static arguments.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 4]>,
}

impl MemberTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.owner);
        variables.extend(
            self.arguments
                .iter()
                .flat_map(|argument| state.argument_variables(argument)),
        );

        variables
    }

    /// Substitute generic arguments through this member projection.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let member = Self {
            source: self.source,
            owner: state.substitute_type_variable(module, substitution, self.owner)?,
            key: self.key,
            arguments: state.substitute_arguments(module, substitution, &self.arguments)?,
        };

        Ok(member)
    }
}

/// A symbol-backed member candidate found by lookup.
pub(in crate::check) struct MemberCandidate {
    /// The resolved member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The resolved member type.
    pub(in crate::check) ty: TypeTerm,
    /// The resolved generic instance, when lookup instantiated an owner.
    pub(in crate::check) instance: Option<GenericInstance>,
}

impl CheckState<'_> {
    /// Reduce one member projection to its type.
    pub(in crate::check) fn reduce_member_term(
        &mut self,
        module: ModuleId,
        source: Option<dir::GlobalNodeIdAny>,
        owner: VariableId,
        key: dir::StaticKey,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let receiver = owner;
        let Some(owner) = self.solved_type_term(owner)? else {
            return Ok(None);
        };
        let owner = match self.reduce_type_term(module, &owner)? {
            Reduction {
                value: Some(value),
                progress: _,
            } => value,
            Reduction {
                value: None,
                progress: _,
            } => owner,
        };
        self.record_member_decision(source, receiver, &key, &owner)?;

        self.resolve_member_type(module, &owner, &key, arguments)
    }

    /// Record one member decision for commit and diagnostics.
    fn record_member_decision(
        &mut self,
        source: Option<dir::GlobalNodeIdAny>,
        receiver: VariableId,
        key: &dir::StaticKey,
        owner: &TypeTerm,
    ) -> CompilerResult<()> {
        let Some(source) = source else {
            return Ok(());
        };
        let decision =
            if let Some(member) = self.member_type_candidate(receiver.module, owner, key)? {
                let target = MemberResolutionTarget::Symbol {
                    symbol: member.symbol,
                    instance: member.instance,
                };
                let member = MemberResolution {
                    source,
                    receiver,
                    target,
                };

                MemberDecision::Resolved(member)
            } else if self.type_term_has_field(receiver.module, owner, key)? {
                let member = MemberResolution {
                    source,
                    receiver,
                    target: MemberResolutionTarget::Field(*key),
                };

                MemberDecision::Resolved(member)
            } else {
                MemberDecision::Rejected(MemberFailure::Missing)
            };

        self.record_member_solution(source, decision);

        Ok(())
    }

    /// Resolve a member type from one reduced type term.
    pub(in crate::check) fn resolve_member_type(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.resolve_member_type(module, &term, key, member_arguments)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(None);
                };

                self.resolve_member_type(module, &term, key, member_arguments)
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if let Some(parameter) = self.generic_type_reference(*symbol)? {
                    self.parameter_member_type(parameter, key, member_arguments)
                } else {
                    self.instantiate_symbol_member_type(
                        module,
                        *symbol,
                        arguments,
                        *key,
                        member_arguments,
                    )
                }
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.instantiate_symbol_member_type(
                module,
                *symbol,
                arguments,
                *key,
                member_arguments,
            ),
            TypeTerm::Tuple { elements, .. } if member_arguments.is_empty() => {
                self.tuple_member_type(module, elements, key)
            }
            TypeTerm::Shape { members } if member_arguments.is_empty() => {
                Ok(self.shape_member_type(members, key))
            }
            TypeTerm::Parameter(parameter) => {
                self.parameter_member_type(*parameter, key, member_arguments)
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
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.member_static_term(module, &term, key)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(None);
                };

                self.member_static_term(module, &term, key)
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_member_static(module, *symbol, *key),
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.instantiate_symbol_member_static(module, *symbol, arguments, *key),
            _ => Ok(None),
        }
    }

    /// Return a symbol-backed member candidate from one reduced type term.
    pub(in crate::check) fn member_type_candidate(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let reduction = self.reduce_type_term(module, term)?;
        let term = reduction.value.as_ref().unwrap_or(term);

        self.member_type_candidate_matching(module, term, key, None)
    }

    /// Return a symbol-backed member candidate that satisfies one protocol.
    pub(in crate::check) fn member_type_candidate_for_protocol(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let reduction = self.reduce_type_term(module, term)?;
        let term = reduction.value.as_ref().unwrap_or(term);

        self.member_type_candidate_matching(module, term, key, Some(protocol))
    }

    /// Return a symbol-backed member candidate from one reduced type term.
    fn member_type_candidate_matching(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.member_type_candidate_matching(module, &term, key, protocol)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(None);
                };

                self.member_type_candidate_matching(module, &term, key, protocol)
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if let Some(parameter) = self.generic_type_reference(*symbol)? {
                    self.parameter_member_candidate(parameter, key, protocol)
                } else {
                    self.instantiate_symbol_member_candidate(
                        module, *symbol, arguments, *key, protocol,
                    )
                }
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => {
                self.instantiate_symbol_member_candidate(module, *symbol, arguments, *key, protocol)
            }
            TypeTerm::Parameter(parameter) => {
                self.parameter_member_candidate(*parameter, key, protocol)
            }
            TypeTerm::Shape { members } => self.shape_member_candidate(members, key, protocol),
            _ => Ok(None),
        }
    }

    /// Return a member type through a generic parameter constraint.
    fn parameter_member_type(
        &mut self,
        parameter: GenericSlotId,
        key: &dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(constraint) = self.generic_parameter_type_constraint(parameter)? else {
            return Ok(None);
        };
        let term = self.reduced_generic_type_constraint(constraint)?;

        self.resolve_member_type(constraint.module, &term, key, member_arguments)
    }

    /// Return a member candidate through a generic parameter constraint.
    fn parameter_member_candidate(
        &mut self,
        parameter: GenericSlotId,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let Some(constraint) = self.generic_parameter_type_constraint(parameter)? else {
            return Ok(None);
        };
        let term = self.reduced_generic_type_constraint(constraint)?;

        self.member_type_candidate_matching(constraint.module, &term, key, protocol)
    }

    /// Return a symbol-backed candidate from one shape method field.
    fn shape_member_candidate(
        &self,
        members: &[ShapeMember],
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        if protocol.is_some() {
            return Ok(None);
        }

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
            if !member_key.matches(key) {
                continue;
            }
            let Some(variable) = ty.variable() else {
                return Ok(None);
            };
            let Some(VariableOutput::Symbol(symbol)) = self.variable(variable).output else {
                return Ok(None);
            };

            return Ok(Some(MemberCandidate {
                symbol,
                ty: TypeTerm::Variable(variable),
                instance: None,
            }));
        }

        Ok(None)
    }

    /// Return the reduced type term for one generic constraint variable.
    fn reduced_generic_type_constraint(
        &mut self,
        constraint: VariableId,
    ) -> CompilerResult<TypeTerm> {
        let term = TypeTerm::Variable(constraint);
        let Some(solution) = self.solved_type_term(constraint)? else {
            return Ok(term);
        };

        // expose transparent aliases before member lookup
        let reduction = self.reduce_type_term(constraint.module, &solution)?;
        let term = reduction.value.unwrap_or(solution);

        Ok(term)
    }

    /// Return a member from a component module or checked dependency.
    fn visible_member_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let symbols = self.visible_member_symbols(symbol, dir::MemberSlot::Key(key))?;
        let symbol = symbols.first().copied();

        Ok(symbol)
    }

    /// Return role members from a component module or checked dependency.
    pub(in crate::check) fn visible_role_member_symbols(
        &mut self,
        symbol: dir::GlobalSymbolId,
        slots: &[dir::MemberSlot],
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        let mut symbols = Vec::new();

        // collect each role slot in caller-specified priority order
        for slot in slots {
            symbols.extend(self.visible_member_symbols(symbol, *slot)?);
        }

        Ok(symbols)
    }

    /// Return members from a component module or checked dependency.
    fn visible_member_symbols(
        &mut self,
        symbol: dir::GlobalSymbolId,
        slot: dir::MemberSlot,
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        if let Some(module) = self.inputs.get(&symbol.module_id) {
            let bindings = module.binding_table();
            let view = module.view();
            let symbols = Self::binding_member_symbols(&bindings, Some(&view), symbol, slot);

            return Ok(symbols);
        }

        let dependency = self.load_dependency_input(symbol.module_id)?;
        let view = dependency.view();
        let symbols = Self::binding_member_symbols(&dependency.bindings, Some(&view), symbol, slot);

        Ok(symbols)
    }

    /// Return member symbols from one binding table.
    pub(in crate::check) fn binding_member_symbols(
        bindings: &dir::BindingTable<'_>,
        view: Option<&dir::View<'_>>,
        owner: dir::GlobalSymbolId,
        slot: dir::MemberSlot,
    ) -> Vec<dir::GlobalSymbolId> {
        for scope_id in bindings.scope_ids() {
            let scope = bindings.get_scope_by_id(scope_id);
            if scope.owner != Some(owner.local_id) {
                continue;
            }

            return match slot {
                dir::MemberSlot::Key(key) => scope
                    .find_symbol(key)
                    .map(|symbol| vec![symbol.into_global(owner.module_id)])
                    .unwrap_or_default(),
                role_slot => Self::binding_role_member_symbols(
                    bindings,
                    view,
                    owner.module_id,
                    scope,
                    role_slot,
                ),
            };
        }

        Vec::new()
    }

    /// Return role member symbols from one owner scope.
    fn binding_role_member_symbols(
        bindings: &dir::BindingTable<'_>,
        view: Option<&dir::View<'_>>,
        module: ModuleId,
        scope: &dir::Scope,
        role_slot: dir::MemberSlot,
    ) -> Vec<dir::GlobalSymbolId> {
        let Some(view) = view else {
            return Vec::new();
        };
        let mut symbols = Vec::new();

        // scan anonymous owner members in source order
        for symbol in scope.anonymous_symbols() {
            let entry = bindings.get_symbol(symbol);
            let Some(member_slot) = Self::symbol_member_slot(view, entry) else {
                continue;
            };
            if member_slot == role_slot {
                symbols.push(symbol.into_global(module));
            }
        }

        symbols
    }

    /// Return the slot carried by one anonymous member symbol.
    fn symbol_member_slot(view: &dir::View<'_>, symbol: &dir::Symbol) -> Option<dir::MemberSlot> {
        let declaration = symbol.declaration?;
        if declaration.local_id.ty != dir::NodeType::Member {
            return None;
        }
        let member_id = dir::LocalNodeId::<dir::Member>::new(declaration.local_id.id);

        view.get(member_id).slot()
    }

    /// Return the solver variable for one selected member symbol.
    pub(in crate::check) fn member_type_variable(
        &mut self,
        module: ModuleId,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<VariableId> {
        self.symbol_type_variable(module, member)
    }

    /// Return the solver variable for one selected static member symbol.
    pub(in crate::check) fn member_static_variable(
        &mut self,
        module: ModuleId,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<VariableId> {
        self.symbol_static_variable(module, member)
    }

    /// Return a member static from one nominal declaration.
    fn symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        let Some(member) = self.visible_member_symbol(symbol, key)? else {
            return Ok(None);
        };
        if self.decide_symbol_availability(module, member, &GenericSubstitution::empty())?
            != Decision::Yes
        {
            return Ok(None);
        }
        let variable = self.member_static_variable(module, member)?;

        Ok(Some(StaticTerm::Variable(variable)))
    }

    /// Instantiate a member type from one applied nominal declaration.
    fn instantiate_symbol_member_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        if let Some(member) = self.visible_member_symbol(symbol, key)? {
            let substitution = self.generic_substitution(symbol, arguments)?;
            if self.decide_symbol_availability(module, member, &substitution)? != Decision::Yes {
                return Ok(None);
            }
            let variable = self.member_type_variable(module, member)?;
            let mut term = TypeTerm::Variable(variable);

            if !substitution.is_empty() {
                let Some(substituted) = term.substitute(module, &substitution, self)? else {
                    return Ok(None);
                };

                term = substituted;
            }

            return self.instantiate_member_type(module, member, member_arguments, term);
        }

        let Some((_, member, substitution)) =
            self.extension_member(module, symbol, arguments, key, None)?
        else {
            return Ok(None);
        };
        if self.decide_symbol_availability(module, member, &substitution)? != Decision::Yes {
            return Ok(None);
        }
        let variable = self.member_type_variable(module, member)?;
        let mut term = TypeTerm::Variable(variable);

        if !substitution.is_empty() {
            let Some(substituted) = term.substitute(module, &substitution, self)? else {
                return Ok(None);
            };

            term = substituted;
        }

        self.instantiate_member_type(module, member, member_arguments, term)
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
        let Some(member) = self.visible_member_symbol(symbol, key)? else {
            return Ok(None);
        };
        let substitution = self.generic_substitution(symbol, arguments)?;
        if self.decide_symbol_availability(module, member, &substitution)? != Decision::Yes {
            return Ok(None);
        }
        let variable = self.member_static_variable(module, member)?;
        let term = StaticTerm::Variable(variable);
        if substitution.is_empty() {
            return Ok(Some(term));
        }

        term.substitute(module, &substitution, self).map(Some)
    }

    /// Instantiate a member candidate from one applied nominal declaration.
    fn instantiate_symbol_member_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        if let Some(member) = self.visible_member_symbol(symbol, key)? {
            let mut substitution = self.generic_substitution(symbol, arguments)?;
            if self.decide_symbol_availability(module, member, &substitution)? != Decision::Yes {
                return Ok(None);
            }
            if let Some(protocol) = protocol
                && !self.symbol_matches_protocol(symbol, protocol, &mut substitution)?
            {
                return Ok(None);
            }
            let variable = self.member_type_variable(module, member)?;
            if substitution.is_empty() {
                return Ok(Some(MemberCandidate {
                    symbol: member,
                    ty: TypeTerm::Variable(variable),
                    instance: None,
                }));
            }
            let term = TypeTerm::Variable(variable);
            let Some(term) = term.substitute(module, &substitution, self)? else {
                return Ok(None);
            };
            let instance = Some(GenericInstance {
                symbol,
                arguments: arguments.to_vec().into(),
            });

            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: term,
                instance,
            }));
        }

        let Some((extension, member, substitution)) =
            self.extension_member(module, symbol, arguments, key, protocol)?
        else {
            return Ok(None);
        };
        if self.decide_symbol_availability(module, member, &substitution)? != Decision::Yes {
            return Ok(None);
        }
        let variable = self.symbol_type_variable(module, member)?;
        if substitution.is_empty() {
            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: TypeTerm::Variable(variable),
                instance: None,
            }));
        }
        let term = TypeTerm::Variable(variable);
        let Some(term) = term.substitute(module, &substitution, self)? else {
            return Ok(None);
        };
        let instance = self.substitution_instance(extension, &substitution)?;

        Ok(Some(MemberCandidate {
            symbol: member,
            ty: term,
            instance,
        }))
    }

    /// Return the generic slot id for one type parameter symbol.
    fn generic_type_reference(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericSlotId>> {
        let Some(variable) = self.variables.type_by_symbol.get(&symbol).copied() else {
            return Ok(None);
        };
        let Some(VariableOutput::Generic(generic)) = &self.variable(variable).output else {
            return Ok(None);
        };
        if !generic.is_type() {
            return Ok(None);
        }

        Ok(Some(generic.slot().id()))
    }

    /// Return the type constraint for one generic type slot.
    fn generic_parameter_type_constraint(
        &self,
        slot_id: GenericSlotId,
    ) -> CompilerResult<Option<VariableId>> {
        let constraint = self.generic_parameters().find_map(|(_, generic)| {
            if generic.slot().id() == slot_id && generic.is_type() {
                generic.type_constraint()
            } else {
                None
            }
        });

        Ok(constraint)
    }

    /// Return whether one resolved member belongs to an owner implementing a language item.
    pub(in crate::check) fn member_implements_language_item(
        &mut self,
        member: dir::GlobalSymbolId,
        item: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let Some(owner) = self.member_owner_symbol(member) else {
            return Ok(false);
        };

        self.symbol_implements_language_item(owner, item)
    }

    /// Return whether one language item symbol satisfies a required protocol item.
    fn language_item_symbol_satisfies(
        &self,
        module: ModuleId,
        actual: dir::GlobalSymbolId,
        required: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        if self.environment.language.item(actual).is_none() {
            return Ok(false);
        }
        let required = self.language_symbol(module, required)?;
        let actual = TypeTerm::Reference {
            source: None,
            symbol: actual,
            arguments: Vec::new().into(),
        };
        let required = TypeTerm::Reference {
            source: None,
            symbol: required,
            arguments: Vec::new().into(),
        };

        Ok(
            self.decide_type_term_relation(TypeRelation::Extends, &actual, &required)?
                == Decision::Yes,
        )
    }

    /// Return the owning declaration symbol for one resolved member.
    fn member_owner_symbol(&self, member: dir::GlobalSymbolId) -> Option<dir::GlobalSymbolId> {
        let module_id = member.module_id;
        let module = self.inputs.get(&member.module_id)?;
        let bindings = module.binding_table();
        let member = bindings.get_symbol(member.local_id);
        let scope = bindings.get_scope(member.scope);

        scope
            .owner
            .map(|owner: dir::LocalSymbolId| owner.into_global(module_id))
    }

    /// Return whether one declaration implements a language item.
    fn symbol_implements_language_item(
        &mut self,
        symbol: dir::GlobalSymbolId,
        item: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let implemented = self.implemented_type_expressions(symbol);

        for implemented in implemented {
            let variable = self.intern_local_type_variable(symbol.module_id, implemented);
            let Some(term) = self.solved_type_term(variable)? else {
                continue;
            };
            if self.type_term_is_language_item(symbol.module_id, &term, item)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one declaration implements the required protocol.
    pub(in crate::check) fn symbol_matches_protocol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        protocol: &MemberProtocol,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let implemented = self.implemented_type_expressions(symbol);

        for implemented in implemented {
            let variable = self.intern_local_type_variable(symbol.module_id, implemented);
            let Some(term) = self.solved_type_term(variable)? else {
                continue;
            };
            let mut candidate = substitution.clone();
            if self.match_protocol_term(
                symbol.module_id,
                symbol,
                &term,
                protocol,
                &mut candidate,
            )? {
                *substitution = candidate;

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Match one implemented protocol type against a required protocol.
    fn match_protocol_term(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
        protocol: &MemberProtocol,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let is_match = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                return self.match_protocol_term(
                    variable.module,
                    owner,
                    &term,
                    protocol,
                    substitution,
                );
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(false);
                };

                return self.match_protocol_term(module, owner, &term, protocol, substitution);
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => {
                self.language_item_symbol_satisfies(module, *symbol, protocol.item)?
                    && self.match_protocol_arguments(
                        module,
                        owner,
                        arguments,
                        protocol,
                        substitution,
                    )?
            }
            _ => false,
        };

        Ok(is_match)
    }

    /// Match required protocol generic arguments.
    fn match_protocol_arguments(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        actual: &[GenericArgument],
        protocol: &MemberProtocol,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if protocol.arguments.is_empty() {
            return Ok(true);
        }
        if actual.len() != protocol.arguments.len() {
            return Ok(false);
        }

        for (actual, required) in actual.iter().zip(&protocol.arguments) {
            if !self.match_argument_pattern(module, owner, *actual, *required, substitution)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return implemented type expressions for one declaration symbol.
    fn implemented_type_expressions(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::LocalNodeId<dir::TypeExpression>> {
        let Some(input) = self.inputs.get(&symbol.module_id) else {
            return Vec::new();
        };
        let bindings = input.binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let Some(declaration) = symbol.declaration else {
            return Vec::new();
        };
        if declaration.module_id != input.module_id
            || declaration.local_id.ty != dir::NodeType::Declaration
        {
            return Vec::new();
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(declaration.local_id.id);

        match input.view().get(declaration) {
            dir::Declaration::Class(declaration) => declaration.implements_types.clone(),
            dir::Declaration::Struct(declaration) => declaration.implements_types.clone(),
            dir::Declaration::Enum(declaration) => declaration.implements_types.clone(),
            dir::Declaration::Extension(declaration) => declaration.implements_types.clone(),
            _ => Vec::new(),
        }
    }

    /// Return whether one type term names a language item.
    fn type_term_is_language_item(
        &self,
        module: ModuleId,
        term: &TypeTerm,
        item: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let is_item = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                return self.type_term_is_language_item(variable.module, &term, item);
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(false);
                };

                return self.type_term_is_language_item(module, &term, item);
            }
            TypeTerm::Reference { symbol, .. } => {
                self.language_item_symbol_satisfies(module, *symbol, item)?
            }
            _ => false,
        };

        Ok(is_item)
    }

    /// Decide structural satisfaction after ordinary relation checks are inconclusive.
    pub(in crate::check) fn decide_structural_satisfies(
        &mut self,
        module: ModuleId,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let target = self.normalize_type_pattern_term(target)?;
        let decision = match target {
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.decide_named_members_satisfied(module, source, symbol, &arguments)?,
            _ => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide whether a source type has every member required by a nominal target.
    fn decide_named_members_satisfied(
        &mut self,
        module: ModuleId,
        source: &TypeTerm,
        target: dir::GlobalSymbolId,
        target_arguments: &[GenericArgument],
    ) -> CompilerResult<Decision> {
        let members = self.nominal_member_declarations(target)?;
        let substitution = self.generic_substitution(target, target_arguments)?;
        let mut decision = Decision::Yes;

        for (key, target_member) in members {
            let Some(source_member) = self.resolve_member_type(module, source, &key, &[])? else {
                return Ok(Decision::No);
            };
            let target_member = self.symbol_type_variable(module, target_member)?;
            let mut target_member = TypeTerm::Variable(target_member);
            if !substitution.is_empty() {
                let Some(term) = target_member.substitute(module, &substitution, self)? else {
                    return Ok(Decision::Undecidable);
                };

                target_member = term;
            }
            decision = decision.and(self.decide_member_satisfied(&source_member, &target_member)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return nominal member declarations in source order.
    fn nominal_member_declarations(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<(dir::StaticKey, dir::GlobalSymbolId)>> {
        let binding_table = self.input(owner.module_id).binding_table();
        let mut members = Vec::new();

        for scope_id in binding_table.scope_ids() {
            let scope = binding_table.get_scope_by_id(scope_id);
            if scope.owner != Some(owner.local_id) {
                continue;
            }

            members.extend(scope.named_symbols().map(
                |(key, symbol): (dir::StaticKey, dir::LocalSymbolId)| {
                    (key, symbol.into_global(owner.module_id))
                },
            ));

            break;
        }

        Ok(members)
    }

    /// Decide whether one member can satisfy another.
    fn decide_member_satisfied(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let source = self.normalize_type_pattern_term(source)?;
        let target = self.normalize_type_pattern_term(target)?;
        let decision = match (&source, &target) {
            (TypeTerm::Function(source), TypeTerm::Function(target)) => {
                self.decide_member_function_satisfied(*source, *target)?
            }
            _ => self.decide_type_term_relation(TypeRelation::Assignable, &source, &target)?,
        };

        Ok(decision)
    }

    /// Decide member function satisfaction, ignoring expected `this`.
    fn decide_member_function_satisfied(
        &self,
        source: TermId<FunctionTerm>,
        target: TermId<FunctionTerm>,
    ) -> CompilerResult<Decision> {
        let source = self.terms.get(source);
        let target = self.terms.get(target);

        if source.asynchrony != target.asynchrony
            || source.is_generator != target.is_generator
            || source.parameters.len() != target.parameters.len()
        {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        for (source, target) in source.parameters.iter().zip(&target.parameters) {
            if source.is_optional != target.is_optional || source.is_rest != target.is_rest {
                return Ok(Decision::No);
            }
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                target.ty,
                source.ty,
            )?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        if let (Some(source), Some(target)) = (source.return_type, target.return_type) {
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                source,
                target,
            )?);
        } else if source.return_type != target.return_type {
            decision = Decision::No;
        }

        Ok(decision)
    }

    /// Return a member type from one check shape term.
    fn shape_member_type(&self, members: &[ShapeMember], key: &dir::StaticKey) -> Option<TypeTerm> {
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
                return Some(match *ty {
                    TypeOperand::Variable(variable) => TypeTerm::Variable(variable),
                    TypeOperand::Term(term) => self.terms.get(term).clone(),
                });
            }
        }

        None
    }

    /// Return a member type from one tuple term.
    fn tuple_member_type(
        &self,
        module: ModuleId,
        elements: &[TupleElement],
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(index) = self.tuple_member_index(module, key)? else {
            return Ok(None);
        };
        let Some(element) = elements.get(index) else {
            return Ok(None);
        };

        let ty = match element.ty {
            TypeOperand::Variable(variable) => TypeTerm::Variable(variable),
            TypeOperand::Term(term) => self.terms.get(term).clone(),
        };

        Ok(Some(ty))
    }

    /// Return the tuple index selected by one member key.
    fn tuple_member_index(
        &self,
        module: ModuleId,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<usize>> {
        let dir::StaticKey::Number(number) = key else {
            return Ok(None);
        };
        let strings = &self.input(module).strings;
        let index = strings.get(*number).parse().ok();

        Ok(index)
    }

    /// Return whether one reduced type has a structural field.
    fn type_term_has_field(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<bool> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                self.type_term_has_field(variable.module, &term, key)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(false);
                };

                self.type_term_has_field(module, &term, key)
            }
            TypeTerm::Shape { members } => Ok(self.shape_member_type(members, key).is_some()),
            _ => Ok(false),
        }
    }
}
