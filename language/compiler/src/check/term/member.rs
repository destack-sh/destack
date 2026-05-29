use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, FunctionTerm, GenericArgument, GenericSlotId, GenericSubstitution, MemberCandidate,
    MemberFailure, MemberProtocol, MemberResolution, MemberResolutionTarget, MemberSelection,
    Origin, ShapeMember, StaticTerm, SymbolCandidate, TermId, TupleElement, TypeOperand,
    TypeOperationTerm, TypeRelation, TypeTerm, VariableId, VariableOutput,
};

use crate::check::{Decision, Reduction};

/// Type member projection term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberTerm {
    /// The work origin that introduced this member projection.
    pub(in crate::check) origin: Origin,
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
            origin: self.origin,
            owner: state.substitute_type_variable(module, substitution, self.owner)?,
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
        owner: VariableId,
        key: dir::StaticKey,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let receiver = owner;
        let Some(owner) = self.solved_type_term(owner)? else {
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
            let decision = if let Some(mut members) =
                self.member_type_candidates(origin, receiver.module, &owner, &key)?
            {
                let target = if members.len() == 1 {
                    let member = members.remove(0);

                    MemberResolutionTarget::Symbol {
                        symbol: member.symbol,
                        instance: member.instance,
                    }
                } else {
                    let members = members
                        .into_iter()
                        .map(|member| SymbolCandidate {
                            symbol: member.symbol,
                            instance: member.instance,
                        })
                        .collect();

                    MemberResolutionTarget::Select(members)
                };
                let member = MemberResolution {
                    source,
                    receiver,
                    target,
                };

                MemberSelection::Resolved(member)
            } else if self.type_term_has_field(receiver.module, &owner, &key)? {
                let member = MemberResolution {
                    source,
                    receiver,
                    target: MemberResolutionTarget::Field(key),
                };

                MemberSelection::Resolved(member)
            } else {
                MemberSelection::Rejected(MemberFailure::Missing)
            };

            self.select_member(source, decision);
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
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.resolve_member_type(origin, module, &term, key, member_arguments)
            }
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
                if let Some(parameter) = self.generic_type_reference(module, *symbol)? {
                    self.parameter_member_type(module, origin, parameter, key, member_arguments)
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
                origin: _,
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
                self.tuple_member_type(elements, key)
            }
            TypeTerm::Shape { members } if member_arguments.is_empty() => {
                Ok(self.shape_member_type(members, key))
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
        parameter: GenericSlotId,
        key: &dir::StaticKey,
        member_arguments: &[GenericArgument],
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(constraint) = self.generic_parameter_type_constraint(module, parameter)? else {
            return Ok(None);
        };
        let term = self.reduced_generic_type_constraint(origin, constraint)?;

        self.resolve_member_type(origin, module, &term, key, member_arguments)
    }

    /// Return member candidates through a generic parameter constraint.
    /// Return the reduced type term for one generic constraint variable.
    pub(in crate::check) fn reduced_generic_type_constraint(
        &mut self,
        origin: Origin,
        constraint: TypeOperand,
    ) -> CompilerResult<TypeTerm> {
        let term = constraint.to_type_term(self);
        let Some(solution) = self.type_operand_term(constraint)? else {
            return Ok(term);
        };

        // expose transparent aliases before member lookup
        let reduction = self.reduce_type_term(origin, &solution)?;
        let term = reduction.value.unwrap_or(solution);

        Ok(term)
    }

    /// Return the solver variable for one selected member symbol.
    pub(in crate::check) fn member_type_variable(
        &mut self,
        module: ModuleId,
        member: dir::GlobalSymbolId,
    ) -> VariableId {
        self.symbol_type_variable(module, member)
    }

    /// Return the solver variable for one selected static member symbol.
    pub(in crate::check) fn member_static_variable(
        &mut self,
        module: ModuleId,
        member: dir::GlobalSymbolId,
    ) -> VariableId {
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
        if self.reduce_symbol_availability(module, member, &GenericSubstitution::empty())?
            != Decision::Yes
        {
            return Ok(None);
        }
        let variable = self.member_static_variable(module, member);

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
            let substitution = self.generic_substitution(module, symbol, arguments)?;
            if self.reduce_symbol_availability(module, member, &substitution)? != Decision::Yes {
                return Ok(None);
            }
            let variable = self.member_type_variable(module, member);
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
        if self.reduce_symbol_availability(module, member, &substitution)? != Decision::Yes {
            return Ok(None);
        }
        let variable = self.member_type_variable(module, member);
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
        let substitution = self.generic_substitution(module, member, arguments)?;
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
        let substitution = self.generic_substitution(module, symbol, arguments)?;
        if self.reduce_symbol_availability(module, member, &substitution)? != Decision::Yes {
            return Ok(None);
        }
        let variable = self.member_static_variable(module, member);
        let term = StaticTerm::Variable(variable);
        if substitution.is_empty() {
            return Ok(Some(term));
        }

        term.substitute(module, &substitution, self).map(Some)
    }

    /// Instantiate a member candidate from one applied nominal declaration.
    /// Return the generic slot id for one type parameter symbol.
    pub(in crate::check) fn generic_type_reference(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericSlotId>> {
        let variable = if self.modules.contains_key(&symbol.module_id) {
            self.variables.type_by_symbol.get(&symbol).copied()
        } else {
            self.module(module)
                .imported_generics
                .values()
                .flatten()
                .copied()
                .find(|variable| {
                    let Some(VariableOutput::Generic(generic)) = &self.variable(*variable).output
                    else {
                        return false;
                    };

                    generic.is_type() && generic.slot().key == dir::GenericSlotKey::Symbol(symbol)
                })
        };
        let Some(variable) = variable else {
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
    pub(in crate::check) fn generic_parameter_type_constraint(
        &self,
        module: ModuleId,
        slot_id: GenericSlotId,
    ) -> CompilerResult<Option<TypeOperand>> {
        let constraint = self
            .generic_parameters_for_owner(module, slot_id.owner)
            .find_map(|(_, generic)| {
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
        let required = self.language_symbol(module, required);
        let actual = TypeTerm::Reference {
            origin: Origin::Symbol(actual),
            symbol: actual,
            arguments: Vec::new().into(),
        };
        let required = TypeTerm::Reference {
            origin: Origin::Symbol(required),
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
        let module = self.modules.get(&member.module_id)?;
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
            let variable = self.intern_local_node_type_variable(symbol.module_id, implemented);
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
            let variable = self.intern_local_node_type_variable(symbol.module_id, implemented);
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
                origin: _,
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
        let Some(module) = self.modules.get(&symbol.module_id) else {
            return Vec::new();
        };
        let bindings = module.binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let Some(declaration) = symbol.declaration else {
            return Vec::new();
        };
        if declaration.module_id != module.module_id
            || declaration.local_id.ty != dir::NodeType::Declaration
        {
            return Vec::new();
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(declaration.local_id.id);

        match module.view().get(declaration) {
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
    pub(in crate::check) fn reduce_structural_satisfies(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let target = self.normalize_type_pattern_term(target)?;
        let decision = match target {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.reduce_named_members_satisfied(origin, module, source, symbol, &arguments)?,
            _ => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide whether a source type has every member required by a nominal target.
    fn reduce_named_members_satisfied(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: &TypeTerm,
        target: dir::GlobalSymbolId,
        target_arguments: &[GenericArgument],
    ) -> CompilerResult<Decision> {
        let members = self.nominal_member_declarations(target)?;
        let substitution = self.generic_substitution(module, target, target_arguments)?;
        let mut decision = Decision::Yes;

        for (key, target_member) in members {
            let Some(source_member) =
                self.resolve_member_type(origin, module, source, &key, &[])?
            else {
                return Ok(Decision::No);
            };
            let target_member = self.symbol_type_variable(module, target_member);
            let mut target_member = TypeTerm::Variable(target_member);
            if !substitution.is_empty() {
                let Some(term) = target_member.substitute(module, &substitution, self)? else {
                    return Ok(Decision::Undecidable);
                };

                target_member = term;
            }
            decision = decision.and(self.reduce_member_satisfied(&source_member, &target_member)?);
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
        let binding_table = self.module(owner.module_id).binding_table();
        let Some(scope) = binding_table.scope_for_owner(owner.local_id) else {
            return Ok(Vec::new());
        };
        let scope = binding_table.get_scope(scope);
        let members = scope
            .named_symbols()
            .map(|(key, symbol)| (key, symbol.into_global(owner.module_id)))
            .collect();

        Ok(members)
    }

    /// Decide whether one member can satisfy another.
    fn reduce_member_satisfied(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let source = self.normalize_type_pattern_term(source)?;
        let target = self.normalize_type_pattern_term(target)?;
        let decision = match (&source, &target) {
            (TypeTerm::Function(source), TypeTerm::Function(target)) => {
                self.reduce_member_function_satisfied(*source, *target)?
            }
            _ => self.decide_type_term_relation(TypeRelation::Assignable, &source, &target)?,
        };

        Ok(decision)
    }

    /// Decide member function satisfaction, ignoring expected `this`.
    fn reduce_member_function_satisfied(
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
            .map(|member| self.terms.push(member).into())
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
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let mut candidates = Vec::with_capacity(elements.len());

        // collect one member candidate set per union element
        for element in elements {
            let Some(term) = self.type_operand_term(*element)? else {
                return Ok(None);
            };
            let Some(mut members) =
                self.member_type_candidates_matching(origin, module, &term, key, protocol)?
            else {
                return Ok(None);
            };

            candidates.append(&mut members);
        }
        if candidates.is_empty() {
            return Ok(None);
        }

        Ok(Some(candidates))
    }

    /// Return a member type from one type operation.
    fn operation_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operation: TermId<TypeOperationTerm>,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        match self.terms.get(operation).clone() {
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
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        match self.terms.get(operation).clone() {
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => self.conditional_member_candidates(
                origin, module, left, right, then_type, else_type, key, protocol,
            ),
            _ => Ok(None),
        }
    }

    /// Return a member type from one conditional type operation.
    fn conditional_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: VariableId,
        right: VariableId,
        then_type: VariableId,
        else_type: VariableId,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;

        // select the known branch
        if decision == Decision::Yes {
            return self.resolve_member_type(
                origin,
                module,
                &TypeTerm::Variable(then_type),
                key,
                &[],
            );
        }
        if decision == Decision::No {
            return self.resolve_member_type(
                origin,
                module,
                &TypeTerm::Variable(else_type),
                key,
                &[],
            );
        }
        let Some(then_member) =
            self.resolve_member_type(origin, module, &TypeTerm::Variable(then_type), key, &[])?
        else {
            return Ok(None);
        };
        let Some(else_member) =
            self.resolve_member_type(origin, module, &TypeTerm::Variable(else_type), key, &[])?
        else {
            return Ok(None);
        };
        if then_member == else_member {
            return Ok(Some(then_member));
        }
        let then_member = self.terms.push(then_member);
        let else_member = self.terms.push(else_member);

        Ok(Some(TypeTerm::Union {
            elements: vec![then_member.into(), else_member.into()],
        }))
    }

    /// Return member candidates from one conditional type operation.
    fn conditional_member_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: VariableId,
        right: VariableId,
        then_type: VariableId,
        else_type: VariableId,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;

        // select the known branch
        if decision == Decision::Yes {
            return self.member_type_candidates_matching(
                origin,
                module,
                &TypeTerm::Variable(then_type),
                key,
                protocol,
            );
        }
        if decision == Decision::No {
            return self.member_type_candidates_matching(
                origin,
                module,
                &TypeTerm::Variable(else_type),
                key,
                protocol,
            );
        }
        let Some(mut then_candidates) = self.member_type_candidates_matching(
            origin,
            module,
            &TypeTerm::Variable(then_type),
            key,
            protocol,
        )?
        else {
            return Ok(None);
        };
        let Some(mut else_candidates) = self.member_type_candidates_matching(
            origin,
            module,
            &TypeTerm::Variable(else_type),
            key,
            protocol,
        )?
        else {
            return Ok(None);
        };

        then_candidates.append(&mut else_candidates);
        if then_candidates.is_empty() {
            return Ok(None);
        }

        Ok(Some(then_candidates))
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

        let ty = match element.ty {
            TypeOperand::Variable(variable) => TypeTerm::Variable(variable),
            TypeOperand::Term(term) => self.terms.get(term).clone(),
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
