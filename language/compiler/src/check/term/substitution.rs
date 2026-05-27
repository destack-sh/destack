use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckState, Decision, GenericInstance, GenericSlotId, Reduction, StaticRelation,
    StaticTerm, TermId, TypeRelation, TypeTerm, VariableId, VariableOutput,
};

/// One generic argument substitution entry.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericSubstitutionEntry {
    /// The generic variable being substituted.
    pub(in crate::check) variable: VariableId,
    /// The generic slot being substituted.
    pub(in crate::check) slot: GenericSlotId,
    /// The applied argument.
    pub(in crate::check) argument: TermId<ArgumentTerm>,
}

/// Generic argument substitution for one applied owner.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericSubstitution {
    /// The substitution entries in declaration order.
    pub(in crate::check) entries: Vec<GenericSubstitutionEntry>,
}

impl GenericSubstitution {
    /// Return an empty substitution.
    pub(in crate::check) fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Return whether this substitution has no entries.
    pub(in crate::check) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

}

impl CheckState<'_> {
    /// Return the type argument for one generic variable.
    pub(in crate::check) fn substitution_type_variable(
        &self,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> Option<VariableId> {
        substitution.entries.iter().find_map(|entry| {
            if entry.variable == variable {
                self.argument_type_variable(entry.argument)
            } else {
                None
            }
        })
    }

    /// Return the static argument for one generic variable.
    pub(in crate::check) fn substitution_static_variable(
        &self,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> Option<VariableId> {
        substitution.entries.iter().find_map(|entry| {
            if entry.variable == variable {
                self.argument_static_variable(entry.argument)
            } else {
                None
            }
        })
    }

    /// Return the type argument for one generic slot.
    pub(in crate::check) fn substitution_type_slot(
        &self,
        substitution: &GenericSubstitution,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        substitution.entries.iter().find_map(|entry| {
            if entry.slot == slot {
                self.argument_type_variable(entry.argument)
            } else {
                None
            }
        })
    }

    /// Return the static argument for one generic slot.
    pub(in crate::check) fn substitution_static_slot(
        &self,
        substitution: &GenericSubstitution,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        substitution.entries.iter().find_map(|entry| {
            if entry.slot == slot {
                self.argument_static_variable(entry.argument)
            } else {
                None
            }
        })
    }

    /// Return the type argument for one explicit generic symbol.
    pub(in crate::check) fn substitution_type_symbol(
        &self,
        substitution: &GenericSubstitution,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        substitution.entries.iter().find_map(|entry| {
            if entry.slot.key == dir::GenericSlotKey::Symbol(symbol) {
                self.argument_type_variable(entry.argument)
            } else {
                None
            }
        })
    }

    /// Return the static argument for one explicit generic symbol.
    pub(in crate::check) fn substitution_static_symbol(
        &self,
        substitution: &GenericSubstitution,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        substitution.entries.iter().find_map(|entry| {
            if entry.slot.key == dir::GenericSlotKey::Symbol(symbol) {
                self.argument_static_variable(entry.argument)
            } else {
                None
            }
        })
    }

    /// Return the generic substitution for one applied symbol.
    pub(in crate::check) fn generic_substitution(
        &mut self,
        owner: dir::GlobalSymbolId,
        arguments: &[TermId<ArgumentTerm>],
    ) -> CompilerResult<GenericSubstitution> {
        if arguments.is_empty() {
            return Ok(GenericSubstitution::empty());
        }
        let Some(module) = self.inputs.get(&owner.module_id) else {
            return Ok(GenericSubstitution::empty());
        };
        let mut slots = module
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner == owner)
            .map(|(variable, generic)| {
                let slot = generic.slot().id();

                (generic.slot().index, variable, slot, generic.is_static())
            })
            .collect::<Vec<_>>();

        slots.sort_by_key(|(index, _, _, _)| *index);

        let entries = slots
            .into_iter()
            .zip(arguments.iter().cloned())
            .map(
                |((_, variable, slot, is_static), argument)| GenericSubstitutionEntry {
                    variable,
                    slot,
                    argument: self.select_argument_for_static_slot(argument, is_static),
                },
            )
            .collect();

        Ok(GenericSubstitution { entries })
    }

    /// Return the concrete instance described by one substitution.
    pub(in crate::check) fn substitution_instance(
        &mut self,
        owner: dir::GlobalSymbolId,
        substitution: &GenericSubstitution,
    ) -> CompilerResult<Option<GenericInstance>> {
        let Some(module) = self.inputs.get(&owner.module_id) else {
            return Ok(None);
        };
        let slots = module
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner == owner)
            .map(|(variable, generic)| (variable, generic.slot().index, generic.is_type()))
            .collect::<Vec<_>>();
        let mut arguments = slots
            .into_iter()
            .filter_map(|(variable, index, is_type)| {
                let argument = if is_type {
                    self.substitution_type_variable(substitution, variable)
                        .map(ArgumentTerm::Type)
                } else {
                    self.substitution_static_variable(substitution, variable)
                        .map(ArgumentTerm::Static)
                }?;
                let argument = self.terms.push(argument);

                Some((index, argument))
            })
            .collect::<Vec<_>>();

        if arguments.is_empty() {
            return Ok(None);
        }
        arguments.sort_by_key(|(index, _)| *index);
        let arguments = arguments
            .into_iter()
            .map(|(_, argument)| argument)
            .collect::<Vec<_>>();

        Ok(Some(GenericInstance {
            symbol: owner,
            arguments,
        }))
    }

    /// Substitute one type variable into a variable.
    pub(in crate::check) fn substitute_type_variable(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        if let Some(argument) = self.substitution_type_variable(substitution, variable) {
            return self.localize_type_variable(module, argument);
        }
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(variable);
        };
        let Some(substituted) = term.substitute(module, substitution, self)? else {
            return Ok(variable);
        };
        if let TypeTerm::Variable(variable) = substituted {
            return self.localize_type_variable(module, variable);
        }
        if substituted == term && variable.module == module {
            return Ok(variable);
        }
        let substituted = match self.reduce_type_term(module, &substituted)? {
            Reduction {
                value: Some(value),
                progress: _,
            } => value,
            Reduction {
                value: None,
                progress: _,
            } => substituted,
        };
        let origin = self.variable_origin(variable)?;

        self.solve_anonymous_type(module, origin, substituted)
    }

    /// Return a variable owned by the target module for one solved type variable.
    fn localize_type_variable(
        &mut self,
        module: ModuleId,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        if variable.module == module {
            return Ok(variable);
        }
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(variable);
        };
        let Some(term) = term.substitute(module, &GenericSubstitution::empty(), self)? else {
            return Ok(variable);
        };
        let origin = self.variable_origin(variable)?;

        self.solve_anonymous_type(module, origin, term)
    }

    /// Substitute type variables into variables.
    pub(in crate::check) fn substitute_type_variables(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variables: &[VariableId],
    ) -> CompilerResult<Vec<VariableId>> {
        variables
            .iter()
            .map(|variable| self.substitute_type_variable(module, substitution, *variable))
            .collect()
    }

    /// Substitute one static variable into a variable.
    pub(in crate::check) fn substitute_static_variable(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        if let Some(argument) = self.substitution_static_variable(substitution, variable) {
            return Ok(argument);
        }
        let Some(term) = self.static_substitution_source(variable)? else {
            return Ok(variable);
        };
        let substituted = term.substitute(module, substitution, self)?;
        if substituted == term {
            return Ok(variable);
        }
        let origin = self.variable_origin(variable)?;

        self.solve_anonymous_static(module, origin, substituted)
    }

    /// Return the solved or source static term available for substitution.
    fn static_substitution_source(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        if let Some(term) = self.solved_static_term(variable)? {
            return Ok(Some(term));
        }
        let variable_state = self.module(variable.module)?.variable(variable);
        let Some(VariableOutput::Node(node)) = variable_state.output else {
            return Ok(None);
        };
        if node.local_id.ty != dir::NodeType::Expression {
            return Ok(None);
        }
        let expression = dir::GlobalNodeId::<dir::Expression> {
            module_id: node.module_id,
            local_id: dir::LocalNodeId::new(node.local_id.id),
        };

        Ok(Some(StaticTerm::Expression(expression)))
    }

    /// Substitute static variables into variables.
    pub(in crate::check) fn substitute_static_variables(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variables: &[VariableId],
    ) -> CompilerResult<Vec<VariableId>> {
        variables
            .iter()
            .map(|variable| self.substitute_static_variable(module, substitution, *variable))
            .collect()
    }

    /// Match a type pattern and collect generic substitutions.
    pub(in crate::check) fn match_type_pattern(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        pattern: &TypeTerm,
        actual: &TypeTerm,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some((variable, slot)) = self.type_pattern_generic(owner, pattern)? {
            return self.match_type_generic(module, variable, slot, actual, substitution);
        }

        let pattern = self.normalize_type_pattern_term(pattern)?;
        let actual = self.normalize_type_pattern_term(actual)?;
        if let Some((variable, slot)) = self.type_pattern_generic(owner, &pattern)? {
            return self.match_type_generic(module, variable, slot, &actual, substitution);
        }

        let is_match = match (&pattern, &actual) {
            (
                TypeTerm::Reference {
                    source: _,
                    symbol: left,
                    arguments: left_arguments,
                },
                TypeTerm::Reference {
                    source: _,
                    symbol: right,
                    arguments: right_arguments,
                },
            ) => {
                if left != right || left_arguments.len() != right_arguments.len() {
                    false
                } else {
                    let mut is_match = true;
                    for (left, right) in left_arguments.iter().zip(right_arguments) {
                        is_match = self.match_argument_pattern(
                            module,
                            owner,
                            *left,
                            *right,
                            substitution,
                        )?;
                        if !is_match {
                            break;
                        }
                    }

                    is_match
                }
            }
            (left, right) => {
                self.decide_type_term_relation(TypeRelation::Equal, left, right)? == Decision::Yes
            }
        };

        Ok(is_match)
    }

    /// Match one generic argument pattern.
    pub(in crate::check) fn match_argument_pattern(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        pattern: TermId<ArgumentTerm>,
        actual: TermId<ArgumentTerm>,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let pattern = self.terms.get(pattern);
        let actual = self.terms.get(actual);
        let is_match = match (pattern, actual) {
            (ArgumentTerm::Type(pattern), ArgumentTerm::Type(actual))
            | (ArgumentTerm::SpreadType(pattern), ArgumentTerm::SpreadType(actual)) => {
                let pattern = TypeTerm::Variable(pattern);
                let actual = TypeTerm::Variable(actual);

                self.match_type_pattern(module, owner, &pattern, &actual, substitution)?
            }
            (ArgumentTerm::Static(pattern), ArgumentTerm::Static(actual))
            | (ArgumentTerm::SpreadStatic(pattern), ArgumentTerm::SpreadStatic(actual)) => {
                if let Some((variable, slot)) = self.variable_static_generic(owner, pattern)? {
                    self.match_static_generic(variable, slot, actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, pattern, actual)?
                        == Decision::Yes
                }
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.type_variable(), actual.type_variable()) =>
            {
                let pattern = TypeTerm::Variable(pattern);
                let actual = TypeTerm::Variable(actual);

                self.match_type_pattern(module, owner, &pattern, &actual, substitution)?
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.static_variable(), actual.static_variable()) =>
            {
                if let Some((variable, slot)) = self.variable_static_generic(owner, pattern)? {
                    self.match_static_generic(variable, slot, actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, pattern, actual)?
                        == Decision::Yes
                }
            }
            _ => false,
        };

        Ok(is_match)
    }

    /// Reduce variables and forms while matching type patterns.
    pub(in crate::check) fn normalize_type_pattern_term(
        &self,
        term: &TypeTerm,
    ) -> CompilerResult<TypeTerm> {
        let term = match term {
            TypeTerm::Variable(variable) => {
                if let Some(term) = self.solved_type_term(*variable)? {
                    term
                } else {
                    term.clone()
                }
            }
            TypeTerm::Form { payload, .. } => {
                if let Some(term) = self.solved_type_term(*payload)? {
                    term
                } else {
                    term.clone()
                }
            }
            _ => term.clone(),
        };

        Ok(term)
    }

    /// Match one generic type slot against an actual type.
    fn match_type_generic(
        &mut self,
        module: ModuleId,
        variable: VariableId,
        slot: GenericSlotId,
        actual: &TypeTerm,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let actual = match actual {
            TypeTerm::Variable(variable) => *variable,
            term => {
                let origin = self.variable_origin(variable)?;

                self.solve_anonymous_type(module, origin, term.clone())?
            }
        };
        if let Some(existing) = self.substitution_type_variable(substitution, variable) {
            let decision = self.decide_type_relation(TypeRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            slot,
            argument: self.terms.push(ArgumentTerm::Type(actual)),
        });

        Ok(true)
    }

    /// Match one generic static slot against an actual static value.
    fn match_static_generic(
        &mut self,
        variable: VariableId,
        slot: GenericSlotId,
        actual: VariableId,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.substitution_static_variable(substitution, variable) {
            let decision = self.decide_static_relation(StaticRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            slot,
            argument: self.terms.push(ArgumentTerm::Static(actual)),
        });

        Ok(true)
    }

    /// Return a generic type slot represented by a pattern term.
    fn type_pattern_generic(
        &self,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let slot = match term {
            TypeTerm::Variable(variable) => self.variable_type_generic(owner, *variable)?,
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_type_generic(owner, *symbol)?,
            TypeTerm::Parameter(slot_id) if slot_id.owner == owner => {
                self.slot_type_generic(*slot_id)?
            }
            TypeTerm::Parameter(_) => None,
            _ => None,
        };

        Ok(slot)
    }

    /// Return a generic type slot represented by a variable.
    fn variable_type_generic(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let module = self.module(variable.module)?;
        let Some(VariableOutput::Generic(generic)) = &module.variable(variable).output else {
            return Ok(None);
        };
        if generic.slot().owner != owner || !generic.is_type() {
            return Ok(None);
        }

        Ok(Some((variable, generic.slot().id())))
    }

    /// Return a generic static slot represented by a variable.
    fn variable_static_generic(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let module = self.module(variable.module)?;
        let Some(VariableOutput::Generic(generic)) = &module.variable(variable).output else {
            return Ok(None);
        };
        if generic.slot().owner != owner || !generic.is_static() {
            return Ok(None);
        }

        Ok(Some((variable, generic.slot().id())))
    }

    /// Return a generic type slot represented by a symbol.
    fn symbol_type_generic(
        &self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let module = self.module(symbol.module_id)?;
        let slot = module.generic_parameters().find_map(|(variable, generic)| {
            if generic.slot().owner == owner
                && generic.slot().key == dir::GenericSlotKey::Symbol(symbol)
                && generic.is_type()
            {
                Some((variable, generic.slot().id()))
            } else {
                None
            }
        });

        Ok(slot)
    }

    /// Return a generic type slot represented by a slot id.
    fn slot_type_generic(
        &self,
        slot_id: GenericSlotId,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let module = self.module(slot_id.owner.module_id)?;
        let slot = module.generic_parameters().find_map(|(variable, generic)| {
            if generic.slot().id() == slot_id && generic.is_type() {
                Some((variable, slot_id))
            } else {
                None
            }
        });

        Ok(slot)
    }
}
