use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericArgument, GenericInstance, GenericSlotId, Reduction, Solution,
    StaticOperand, StaticRelation, StaticTerm, TypeOperand, TypeRelation, TypeTerm, VariableId,
    VariableKind, VariableOutput,
};

/// One generic argument substitution entry.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericSubstitutionEntry {
    /// The generic variable being substituted.
    pub(in crate::check) variable: VariableId,
    /// The generic slot being substituted.
    pub(in crate::check) slot: GenericSlotId,
    /// The applied argument.
    pub(in crate::check) argument: GenericArgument,
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
    /// Return the type argument operand for one generic variable.
    pub(in crate::check) fn substitution_type_operand(
        &self,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> Option<TypeOperand> {
        substitution.entries.iter().find_map(|entry| {
            if entry.variable == variable {
                entry.argument.type_operand()
            } else {
                None
            }
        })
    }

    /// Return the type argument for one generic variable.
    pub(in crate::check) fn substitution_type_variable(
        &self,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> Option<VariableId> {
        self.substitution_type_operand(substitution, variable)?
            .variable()
    }

    /// Return the static argument operand for one generic variable.
    pub(in crate::check) fn substitution_static_operand(
        &self,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> Option<StaticOperand> {
        substitution.entries.iter().find_map(|entry| {
            if entry.variable == variable {
                entry.argument.static_operand()
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
        self.substitution_static_operand(substitution, variable)?
            .variable()
    }

    /// Return the type argument for one generic slot.
    pub(in crate::check) fn substitution_type_slot(
        &self,
        substitution: &GenericSubstitution,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        substitution.entries.iter().find_map(|entry| {
            if entry.slot == slot {
                self.argument_type_variable(&entry.argument)
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
                self.argument_static_variable(&entry.argument)
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
                self.argument_type_variable(&entry.argument)
            } else {
                None
            }
        })
    }

    /// Return the generic substitution for one applied symbol.
    pub(in crate::check) fn generic_substitution(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<GenericSubstitution> {
        if arguments.is_empty() {
            return Ok(GenericSubstitution::empty());
        }
        let mut slots = self
            .generic_parameters_for_owner(module, owner)
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
                    argument: self.select_argument_for_static_slot(&argument, is_static),
                },
            )
            .collect();

        Ok(GenericSubstitution { entries })
    }

    /// Return the concrete instance described by one substitution.
    pub(in crate::check) fn substitution_instance(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        substitution: &GenericSubstitution,
    ) -> CompilerResult<Option<GenericInstance>> {
        let slots = self
            .generic_parameters_for_owner(module, owner)
            .map(|(variable, generic)| (variable, generic.slot().index, generic.is_type()))
            .collect::<Vec<_>>();
        let mut arguments = slots
            .into_iter()
            .filter_map(|(variable, index, is_type)| {
                let argument = if is_type {
                    self.substitution_type_operand(substitution, variable)
                        .map(GenericArgument::Type)
                } else {
                    self.substitution_static_operand(substitution, variable)
                        .map(GenericArgument::Static)
                }?;

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
            arguments: arguments.into(),
        }))
    }

    /// Substitute one type variable into a variable.
    pub(in crate::check) fn substitute_type_variable(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        let operand = self.substitute_type_variable_operand(module, substitution, variable)?;
        let variable = self.substituted_type_variable(module, variable, operand);

        Ok(variable)
    }

    /// Return a variable for one substituted type operand.
    fn substituted_type_variable(
        &mut self,
        module: ModuleId,
        source: VariableId,
        operand: TypeOperand,
    ) -> VariableId {
        let term = match operand {
            TypeOperand::Variable(variable) => return variable,
            TypeOperand::Term(term) => term,
        };
        let origin = self.variable(source).source;
        assert_eq!(
            origin.module(),
            module,
            "substituted type variable origin must be local"
        );
        let variable = self.allocate_inference_variable(module, VariableKind::Type, origin);

        self.insert_known_solution(variable, Solution::Type(term));

        variable
    }

    /// Substitute one type variable into an operand.
    pub(in crate::check) fn substitute_type_variable_operand(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<TypeOperand> {
        if let Some(argument) = self.substitution_type_operand(substitution, variable) {
            return Ok(argument);
        }
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(variable.into());
        };
        let Some(substituted) = term.substitute(module, substitution, self)? else {
            return Ok(variable.into());
        };
        if let TypeTerm::Variable(variable) = substituted {
            return Ok(variable.into());
        }
        if substituted == term {
            return Ok(variable.into());
        }
        let origin = self.variable(variable).source;
        let substituted = match self.reduce_type_term(origin, &substituted)? {
            Reduction {
                value: Some(value),
                progress: _,
            } => value,
            Reduction {
                value: None,
                progress: _,
            } => substituted,
        };
        let term = self.terms.push(substituted);

        Ok(term.into())
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

    /// Substitute one type operand.
    pub(in crate::check) fn substitute_type_operand(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        let operand = match operand {
            TypeOperand::Variable(variable) => {
                self.substitute_type_variable_operand(module, substitution, variable)?
            }
            TypeOperand::Term(term) => {
                let term = self.terms.get(term).clone();
                let Some(term) = term.substitute(module, substitution, self)? else {
                    return Ok(operand);
                };
                let term = self.terms.push(term);

                term.into()
            }
        };

        Ok(operand)
    }

    /// Substitute type operands.
    pub(in crate::check) fn substitute_type_operands(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        operands: &[TypeOperand],
    ) -> CompilerResult<Vec<TypeOperand>> {
        operands
            .iter()
            .map(|operand| self.substitute_type_operand(module, substitution, *operand))
            .collect()
    }

    /// Substitute one static variable into a variable.
    pub(in crate::check) fn substitute_static_variable(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        let operand = self.substitute_static_variable_operand(module, substitution, variable)?;
        let variable = self.substituted_static_variable(module, variable, operand);

        Ok(variable)
    }

    /// Return a variable for one substituted static operand.
    fn substituted_static_variable(
        &mut self,
        module: ModuleId,
        source: VariableId,
        operand: StaticOperand,
    ) -> VariableId {
        let term = match operand {
            StaticOperand::Variable(variable) => return variable,
            StaticOperand::Term(term) => term,
        };
        let origin = self.variable(source).source;
        assert_eq!(
            origin.module(),
            module,
            "substituted static variable origin must be local"
        );
        let variable = self.allocate_inference_variable(module, VariableKind::Static, origin);

        self.insert_known_solution(variable, Solution::Static(term));

        variable
    }

    /// Substitute one static variable into an operand.
    pub(in crate::check) fn substitute_static_variable_operand(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        variable: VariableId,
    ) -> CompilerResult<StaticOperand> {
        if let Some(argument) = self.substitution_static_operand(substitution, variable) {
            return Ok(argument);
        }
        let Some(term) = self.static_substitution_source(variable)? else {
            return Ok(variable.into());
        };
        let substituted = term.substitute(module, substitution, self)?;
        if substituted == term {
            return Ok(variable.into());
        }
        let term = self.terms.push(substituted);

        Ok(term.into())
    }

    /// Return the solved or source static term available for substitution.
    fn static_substitution_source(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        if let Some(term) = self.solved_static_term(variable)? {
            return Ok(Some(term));
        }
        let variable_state = self.variable(variable);
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

    /// Substitute one static operand.
    pub(in crate::check) fn substitute_static_operand(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        operand: StaticOperand,
    ) -> CompilerResult<StaticOperand> {
        let operand = match operand {
            StaticOperand::Variable(variable) => {
                self.substitute_static_variable_operand(module, substitution, variable)?
            }
            StaticOperand::Term(term) => {
                let term = self.terms.get(term).clone();
                let term = term.substitute(module, substitution, self)?;
                let term = self.terms.push(term);

                term.into()
            }
        };

        Ok(operand)
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
        if let Some((variable, slot)) = self.type_pattern_generic(module, owner, pattern)? {
            let actual = self.type_pattern_term_operand(actual);

            return self.match_type_generic(variable, slot, actual, substitution);
        }

        let pattern = self.normalize_type_pattern_term(pattern)?;
        let actual = self.normalize_type_pattern_term(actual)?;
        if let Some((variable, slot)) = self.type_pattern_generic(module, owner, &pattern)? {
            let actual = self.type_pattern_term_operand(&actual);

            return self.match_type_generic(variable, slot, actual, substitution);
        }

        let is_match = match (&pattern, &actual) {
            (
                TypeTerm::Reference {
                    origin: _,
                    symbol: left,
                    arguments: left_arguments,
                },
                TypeTerm::Reference {
                    origin: _,
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

    /// Return an operand for one type pattern term.
    fn type_pattern_term_operand(&mut self, term: &TypeTerm) -> TypeOperand {
        match term {
            TypeTerm::Variable(variable) => (*variable).into(),
            term => self.terms.push(term.clone()).into(),
        }
    }

    /// Match one generic argument pattern.
    pub(in crate::check) fn match_argument_pattern(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        pattern: GenericArgument,
        actual: GenericArgument,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let is_match = match (pattern, actual) {
            (GenericArgument::Type(pattern), GenericArgument::Type(actual))
            | (GenericArgument::SpreadType(pattern), GenericArgument::SpreadType(actual)) => {
                let Some(pattern) = self.type_pattern_operand_term(pattern)? else {
                    return Ok(false);
                };
                let Some(actual) = self.type_pattern_operand_term(actual)? else {
                    return Ok(false);
                };

                self.match_type_pattern(module, owner, &pattern, &actual, substitution)?
            }
            (GenericArgument::Static(pattern), GenericArgument::Static(actual))
            | (GenericArgument::SpreadStatic(pattern), GenericArgument::SpreadStatic(actual)) => {
                if let Some(pattern) = pattern.variable()
                    && let Some((variable, slot)) = self.variable_static_generic(owner, pattern)?
                {
                    self.match_static_generic(variable, slot, actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, pattern, actual)?
                        == Decision::Yes
                }
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.type_operand(), actual.type_operand()) =>
            {
                let Some(pattern) = self.type_pattern_operand_term(pattern)? else {
                    return Ok(false);
                };
                let Some(actual) = self.type_pattern_operand_term(actual)? else {
                    return Ok(false);
                };

                self.match_type_pattern(module, owner, &pattern, &actual, substitution)?
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.static_operand(), actual.static_operand()) =>
            {
                if let Some(pattern_variable) = pattern.variable()
                    && let Some((variable, slot)) =
                        self.variable_static_generic(owner, pattern_variable)?
                {
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

    /// Return a type pattern term for one operand.
    fn type_pattern_operand_term(&self, operand: TypeOperand) -> CompilerResult<Option<TypeTerm>> {
        let term = match operand {
            TypeOperand::Variable(variable) => self
                .solved_type_term(variable)?
                .unwrap_or(TypeTerm::Variable(variable)),
            TypeOperand::Term(term) => self.terms.get(term).clone(),
        };

        Ok(Some(term))
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
                if let Some(term) = self.type_operand_term(*payload)? {
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
        variable: VariableId,
        slot: GenericSlotId,
        actual: TypeOperand,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.substitution_type_operand(substitution, variable) {
            let decision = self.decide_type_relation(TypeRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            slot,
            argument: GenericArgument::Type(actual),
        });

        Ok(true)
    }

    /// Match one generic static slot against an actual static value.
    fn match_static_generic(
        &mut self,
        variable: VariableId,
        slot: GenericSlotId,
        actual: StaticOperand,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.substitution_static_operand(substitution, variable) {
            let decision = self.decide_static_relation(StaticRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            slot,
            argument: GenericArgument::Static(actual),
        });

        Ok(true)
    }

    /// Return a generic type slot represented by a pattern term.
    fn type_pattern_generic(
        &self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let slot = match term {
            TypeTerm::Variable(variable) => self.variable_type_generic(owner, *variable)?,
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_type_generic(module, owner, *symbol)?,
            TypeTerm::Parameter(slot_id) if slot_id.owner == owner => {
                self.slot_type_generic(module, *slot_id)?
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
        let Some(VariableOutput::Generic(generic)) = &self.variable(variable).output else {
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
        let Some(VariableOutput::Generic(generic)) = &self.variable(variable).output else {
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
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let slot =
            self.generic_parameters_for_owner(module, owner)
                .find_map(|(variable, generic)| {
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
        module: ModuleId,
        slot_id: GenericSlotId,
    ) -> CompilerResult<Option<(VariableId, GenericSlotId)>> {
        let slot = self
            .generic_parameters_for_owner(module, slot_id.owner)
            .find_map(|(variable, generic)| {
                if generic.slot().id() == slot_id && generic.is_type() {
                    Some((variable, slot_id))
                } else {
                    None
                }
            });

        Ok(slot)
    }
}
