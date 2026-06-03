use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericApplication, GenericArgument, GenericSlotId, Origin, Reduction,
    StaticOperand, StaticRelation, StaticTerm, TypeOperand, TypeRelation, TypeSolution, TypeTerm,
    VariableId,
};

/// One generic argument substitution entry.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericSubstitutionEntry {
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

/// Receiver binding for one selected method call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct ReceiverSubstitution {
    /// The selected receiver type.
    pub(in crate::check) receiver: TypeOperand,
}

/// Type and static term substitution context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct Substitution<'a> {
    /// Generic argument substitution, if any.
    pub(in crate::check) generic: Option<&'a GenericSubstitution>,
    /// Receiver binding, if any.
    pub(in crate::check) receiver: Option<ReceiverSubstitution>,
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

impl ReceiverSubstitution {
    /// Create one receiver substitution.
    pub(in crate::check) fn new(receiver: TypeOperand) -> Self {
        Self { receiver }
    }
}

impl<'a> Substitution<'a> {
    /// Return a generic argument substitution context.
    pub(in crate::check) fn generic(generic: &'a GenericSubstitution) -> Self {
        Self {
            generic: Some(generic),
            receiver: None,
        }
    }

    /// Return a receiver substitution context.
    pub(in crate::check) fn receiver(receiver: ReceiverSubstitution) -> Self {
        Self {
            generic: None,
            receiver: Some(receiver),
        }
    }
}

impl<'a> From<&'a GenericSubstitution> for Substitution<'a> {
    /// Convert a generic substitution into a term substitution context.
    fn from(generic: &'a GenericSubstitution) -> Self {
        Self::generic(generic)
    }
}

impl<'a> From<ReceiverSubstitution> for Substitution<'a> {
    /// Convert a receiver substitution into a term substitution context.
    fn from(receiver: ReceiverSubstitution) -> Self {
        Self::receiver(receiver)
    }
}

impl CheckState<'_> {
    /// Return the type argument operand for one generic slot.
    pub(in crate::check) fn substitution_type_operand<'a>(
        &self,
        substitution: impl Into<Substitution<'a>>,
        slot: GenericSlotId,
    ) -> Option<TypeOperand> {
        let substitution = substitution.into();
        let substitution = substitution.generic?;

        substitution.entries.iter().find_map(|entry| {
            if entry.slot == slot {
                entry.argument.type_operand()
            } else {
                None
            }
        })
    }

    /// Return the type argument for one generic slot.
    pub(in crate::check) fn substitution_type_variable<'a>(
        &self,
        substitution: impl Into<Substitution<'a>>,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        self.substitution_type_operand(substitution, slot)?
            .variable()
    }

    /// Return the static argument operand for one generic slot.
    pub(in crate::check) fn substitution_static_operand<'a>(
        &self,
        substitution: impl Into<Substitution<'a>>,
        slot: GenericSlotId,
    ) -> Option<StaticOperand> {
        let substitution = substitution.into();
        let substitution = substitution.generic?;

        substitution.entries.iter().find_map(|entry| {
            if entry.slot == slot {
                entry.argument.static_operand()
            } else {
                None
            }
        })
    }

    /// Return the static argument for one generic slot.
    pub(in crate::check) fn substitution_static_variable<'a>(
        &self,
        substitution: impl Into<Substitution<'a>>,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        self.substitution_static_operand(substitution, slot)?
            .variable()
    }

    /// Return the type argument for one generic slot.
    pub(in crate::check) fn substitution_type_slot<'a>(
        &self,
        substitution: impl Into<Substitution<'a>>,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        let substitution = substitution.into();
        let substitution = substitution.generic?;

        substitution.entries.iter().find_map(|entry| {
            if entry.slot == slot {
                entry
                    .argument
                    .type_operand()
                    .and_then(|operand| operand.variable())
            } else {
                None
            }
        })
    }

    /// Return the static argument for one generic slot.
    pub(in crate::check) fn substitution_static_slot<'a>(
        &self,
        substitution: impl Into<Substitution<'a>>,
        slot: GenericSlotId,
    ) -> Option<VariableId> {
        let substitution = substitution.into();
        let substitution = substitution.generic?;

        substitution.entries.iter().find_map(|entry| {
            if entry.slot == slot {
                entry
                    .argument
                    .static_operand()
                    .and_then(|operand| operand.variable())
            } else {
                None
            }
        })
    }

    /// Return the type argument for one explicit generic symbol.
    pub(in crate::check) fn substitution_type_symbol<'a>(
        &self,
        substitution: impl Into<Substitution<'a>>,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        let substitution = substitution.into();
        let substitution = substitution.generic?;

        substitution.entries.iter().find_map(|entry| {
            if entry.slot.key == dir::GenericSlotKey::Symbol(symbol) {
                entry
                    .argument
                    .type_operand()
                    .and_then(|operand| operand.variable())
            } else {
                None
            }
        })
    }

    /// Return the generic substitution for one applied symbol.
    pub(in crate::check) fn generic_substitution(
        &mut self,
        owner: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<GenericSubstitution> {
        if arguments.is_empty() {
            return Ok(GenericSubstitution::empty());
        }
        let mut slots = self
            .inference
            .generic_slots_for_owner(owner)
            .map(|(_, generic)| {
                let slot = generic.slot().id();

                (generic.slot().index, slot, generic.is_static())
            })
            .collect::<Vec<_>>();

        slots.sort_by_key(|(index, _, _)| *index);

        let entries = slots
            .into_iter()
            .zip(arguments.iter().cloned())
            .map(
                |((_, slot, is_static), argument)| GenericSubstitutionEntry {
                    slot,
                    argument: argument.select_for_static_slot(is_static),
                },
            )
            .collect();

        Ok(GenericSubstitution { entries })
    }

    /// Return the generic application described by one substitution.
    pub(in crate::check) fn substitution_application(
        &mut self,
        owner: dir::GlobalSymbolId,
        substitution: &GenericSubstitution,
    ) -> CompilerResult<Option<GenericApplication>> {
        let slots = self
            .inference
            .generic_slots_for_owner(owner)
            .map(|(_, generic)| {
                let slot = generic.slot().id();

                (slot, generic.slot().index, generic.is_type())
            })
            .collect::<Vec<_>>();
        let mut arguments = slots
            .into_iter()
            .filter_map(|(slot, index, is_type)| {
                let argument = if is_type {
                    self.substitution_type_operand(substitution, slot)
                        .map(GenericArgument::Type)
                } else {
                    self.substitution_static_operand(substitution, slot)
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

        Ok(Some(GenericApplication {
            owner,
            arguments: arguments.into(),
        }))
    }

    /// Substitute one type variable into a variable.
    pub(in crate::check) fn substitute_type_variable<'a>(
        &mut self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        variable: VariableId,
    ) -> CompilerResult<VariableId> {
        let operand = self.substitute_type_variable_operand(module, substitution, variable)?;
        let variable = self.substituted_type_variable(module, variable, operand)?;

        Ok(variable)
    }

    /// Return a variable for one substituted type operand.
    fn substituted_type_variable(
        &mut self,
        module: ModuleId,
        source: VariableId,
        operand: TypeOperand,
    ) -> CompilerResult<VariableId> {
        let origin = self.variable(source).source;
        if origin.module() != module {
            panic!("substituted type variable origin must be local");
        }

        let term = match operand {
            TypeOperand::Variable(variable) => return Ok(variable),
            TypeOperand::Term(term) => term,
            TypeOperand::Type(ty) => {
                let variable = self.create_type_variable(module, origin);
                let solution = TypeSolution::Type(ty);

                self.set_variable_solution(variable, solution.into())?;

                return Ok(variable);
            }
        };
        let variable = self.create_type_variable(module, origin);
        let solution = TypeSolution::Term(term);

        self.set_variable_solution(variable, solution.into())?;

        Ok(variable)
    }

    /// Substitute one type variable into an operand.
    pub(in crate::check) fn substitute_type_variable_operand<'a>(
        &mut self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        variable: VariableId,
    ) -> CompilerResult<TypeOperand> {
        let substitution = substitution.into();

        let Some(term) = self.type_solution(variable)? else {
            return Ok(variable.into());
        };
        let Some(substituted) = term.substitute(module, substitution, self)? else {
            return Ok(variable.into());
        };
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
        let term = self.inference.push_term(substituted);

        Ok(term.into())
    }

    /// Substitute one type operand.
    pub(in crate::check) fn substitute_type_operand<'a>(
        &mut self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        let substitution = substitution.into();

        let operand = match operand {
            TypeOperand::Variable(variable) => {
                self.substitute_type_variable_operand(module, substitution, variable)?
            }
            TypeOperand::Term(term) => {
                let term = self.inference.term(term).clone();
                let Some(term) = term.substitute(module, substitution, self)? else {
                    return Ok(operand);
                };
                let term = self.inference.push_term(term);

                term.into()
            }
            TypeOperand::Type(_) => operand,
        };

        Ok(operand)
    }

    /// Substitute the selected receiver through one type operand.
    pub(in crate::check) fn substitute_receiver_type_operand(
        &mut self,
        module: ModuleId,
        operand: TypeOperand,
        substitution: ReceiverSubstitution,
    ) -> CompilerResult<TypeOperand> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(operand);
        };
        if matches!(term, TypeTerm::This) {
            return Ok(substitution.receiver);
        }
        let Some(term) = term.substitute(module, substitution, self)? else {
            return Ok(operand);
        };
        let term = self.inference.push_term(term);

        Ok(term.into())
    }

    /// Substitute type operands.
    pub(in crate::check) fn substitute_type_operands<'a>(
        &mut self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        operands: &[TypeOperand],
    ) -> CompilerResult<Vec<TypeOperand>> {
        let substitution = substitution.into();

        operands
            .iter()
            .map(|operand| self.substitute_type_operand(module, substitution, *operand))
            .collect()
    }

    /// Substitute one static variable into an operand.
    pub(in crate::check) fn substitute_static_variable_operand<'a>(
        &mut self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        variable: VariableId,
    ) -> CompilerResult<StaticOperand> {
        let substitution = substitution.into();

        let Some(term) = self.static_substitution_source(variable)? else {
            return Ok(variable.into());
        };
        let substituted = term.substitute(module, substitution, self)?;
        if substituted == term {
            return Ok(variable.into());
        }
        let term = self.inference.push_term(substituted);

        Ok(term.into())
    }

    /// Return the solved or source static term available for substitution.
    fn static_substitution_source(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        if let Some(term) = self.static_solution(variable)? {
            return Ok(Some(term));
        }
        let Origin::Node(node) = self.variable(variable).source else {
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

    /// Substitute one static operand.
    pub(in crate::check) fn substitute_static_operand<'a>(
        &mut self,
        module: ModuleId,
        substitution: impl Into<Substitution<'a>> + Copy,
        operand: StaticOperand,
    ) -> CompilerResult<StaticOperand> {
        let substitution = substitution.into();

        let operand = match operand {
            StaticOperand::Variable(variable) => {
                self.substitute_static_variable_operand(module, substitution, variable)?
            }
            StaticOperand::Term(term) => {
                let term = self.inference.term(term).clone();
                let term = term.substitute(module, substitution, self)?;
                let term = self.inference.push_term(term);

                term.into()
            }
            StaticOperand::Static(_) => operand,
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
        if let Some(slot) = self.type_pattern_generic(owner, pattern)? {
            let actual = self.type_pattern_term_operand(actual);

            return self.match_type_generic(slot, actual, substitution);
        }

        let pattern = self.normalize_type_pattern_term(pattern)?;
        let actual = self.normalize_type_pattern_term(actual)?;
        if let Some(slot) = self.type_pattern_generic(owner, &pattern)? {
            let actual = self.type_pattern_term_operand(&actual);

            return self.match_type_generic(slot, actual, substitution);
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
                        is_match =
                            self.match_argument_pattern(module, owner, left, right, substitution)?;
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
        self.inference.push_term(term.clone()).into()
    }

    /// Match one generic argument pattern.
    pub(in crate::check) fn match_argument_pattern(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        pattern: &GenericArgument,
        actual: &GenericArgument,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let is_match = match (pattern, actual) {
            (GenericArgument::Type(pattern), GenericArgument::Type(actual))
            | (GenericArgument::SpreadType(pattern), GenericArgument::SpreadType(actual)) => {
                let Some(pattern) = self.type_pattern_operand_term(*pattern)? else {
                    return Ok(false);
                };
                let Some(actual) = self.type_pattern_operand_term(*actual)? else {
                    return Ok(false);
                };

                self.match_type_pattern(module, owner, &pattern, &actual, substitution)?
            }
            (GenericArgument::Static(pattern), GenericArgument::Static(actual))
            | (GenericArgument::SpreadStatic(pattern), GenericArgument::SpreadStatic(actual)) => {
                if let Some(slot) = self.static_pattern_generic(owner, *pattern)? {
                    self.match_static_generic(slot, *actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, *pattern, *actual)?
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
                if let Some(slot) = self.static_pattern_generic(owner, pattern)? {
                    self.match_static_generic(slot, actual, substitution)?
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
            TypeOperand::Variable(variable) => {
                let Some(term) = self.type_solution(variable)? else {
                    return Ok(None);
                };

                term
            }
            TypeOperand::Term(term) => self.inference.term(term).clone(),
            TypeOperand::Type(ty) => TypeTerm::Type(ty),
        };

        Ok(Some(term))
    }

    /// Reduce variables and forms while matching type patterns.
    pub(in crate::check) fn normalize_type_pattern_term(
        &self,
        term: &TypeTerm,
    ) -> CompilerResult<TypeTerm> {
        let term = match term {
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
        slot: GenericSlotId,
        actual: TypeOperand,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.substitution_type_operand(&*substitution, slot) {
            let decision = self.decide_type_relation(TypeRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            slot,
            argument: GenericArgument::Type(actual),
        });

        Ok(true)
    }

    /// Match one generic static slot against an actual static value.
    fn match_static_generic(
        &mut self,
        slot: GenericSlotId,
        actual: StaticOperand,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.substitution_static_operand(&*substitution, slot) {
            let decision = self.decide_static_relation(StaticRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            slot,
            argument: GenericArgument::Static(actual),
        });

        Ok(true)
    }

    /// Return a generic type slot represented by a pattern term.
    fn type_pattern_generic(
        &self,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<GenericSlotId>> {
        let slot = match term {
            TypeTerm::Reference {
                origin: _,
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

    /// Return a generic static slot represented by a static pattern.
    fn static_pattern_generic(
        &self,
        owner: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> CompilerResult<Option<GenericSlotId>> {
        let term = match operand {
            StaticOperand::Variable(variable) => self.static_substitution_source(variable)?,
            StaticOperand::Term(term) => Some(self.inference.term(term).clone()),
            StaticOperand::Static(value) => match self.r#static(value) {
                dir::StaticTerm::Parameter(parameter) => {
                    Some(StaticTerm::Parameter((*parameter).into()))
                }
                term => Some(StaticTerm::Literal(term.clone())),
            },
        };
        let Some(StaticTerm::Parameter(slot)) = term else {
            return Ok(None);
        };
        let generic = self.inference.generic_slot(slot);
        let is_match = generic.slot().owner == owner
            && generic.is_static()
            && self
                .inference
                .generic_slots_for_owner(owner)
                .any(|(candidate, _)| candidate == slot);

        Ok(is_match.then_some(slot))
    }

    /// Return a generic type slot represented by a symbol.
    fn symbol_type_generic(
        &self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericSlotId>> {
        let slot = self
            .inference
            .generic_slots_for_owner(owner)
            .find_map(|(slot, generic)| {
                if generic.slot().owner == owner
                    && generic.slot().key == dir::GenericSlotKey::Symbol(symbol)
                    && generic.is_type()
                {
                    Some(slot)
                } else {
                    None
                }
            });

        Ok(slot)
    }

    /// Return a generic type slot represented by a slot id.
    fn slot_type_generic(&self, slot_id: GenericSlotId) -> CompilerResult<Option<GenericSlotId>> {
        let slot = self
            .inference
            .generic_slots_for_owner(slot_id.owner)
            .find_map(|(slot, generic)| {
                if generic.slot().id() == slot_id && generic.is_type() {
                    Some(slot)
                } else {
                    None
                }
            });

        Ok(slot)
    }
}
