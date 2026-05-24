use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckComponentState, Decision, GenericInstance, StaticRelation, TypeRelation,
    TypeTerm, VariableId, VariableOrigin,
};

/// One generic argument substitution entry.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct GenericSubstitutionEntry {
    /// The generic variable being substituted.
    pub(in crate::check) variable: VariableId,
    /// The source symbol for explicit generic slots.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The applied argument.
    pub(in crate::check) argument: ArgumentTerm,
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

    /// Return the type argument for one generic variable.
    pub(in crate::check) fn type_variable(&self, variable: VariableId) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.variable == variable
                && let ArgumentTerm::Type(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }

    /// Return the static argument for one generic variable.
    pub(in crate::check) fn static_variable(&self, variable: VariableId) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.variable == variable
                && let ArgumentTerm::Static(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }

    /// Return the type argument for one explicit generic symbol.
    pub(in crate::check) fn type_symbol(&self, symbol: dir::GlobalSymbolId) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.symbol == Some(symbol)
                && let ArgumentTerm::Type(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }

    /// Return the static argument for one explicit generic symbol.
    pub(in crate::check) fn static_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        self.entries.iter().find_map(|entry| {
            if entry.symbol == Some(symbol)
                && let ArgumentTerm::Static(variable) = entry.argument
            {
                Some(variable)
            } else {
                None
            }
        })
    }
}

impl CheckComponentState<'_> {
    /// Return the generic substitution for one applied symbol.
    pub(in crate::check) fn generic_substitution(
        &self,
        owner: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<GenericSubstitution> {
        if arguments.is_empty() {
            return Ok(GenericSubstitution::empty());
        }
        let Some(module) = self.modules.get(&owner.module_id) else {
            return Ok(GenericSubstitution::empty());
        };
        let mut slots = module
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner == owner)
            .map(|(variable, generic)| {
                let symbol = match generic.slot().key {
                    dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
                    dir::GenericSlotKey::Generated(_) => None,
                };

                (generic.slot().index, variable, symbol)
            })
            .collect::<Vec<_>>();

        slots.sort_by_key(|(index, _, _)| *index);

        let entries = slots
            .into_iter()
            .zip(arguments.iter().cloned())
            .map(
                |((_, variable, symbol), argument)| GenericSubstitutionEntry {
                    variable,
                    symbol,
                    argument,
                },
            )
            .collect();

        Ok(GenericSubstitution { entries })
    }

    /// Return the concrete instance described by one substitution.
    pub(in crate::check) fn substitution_instance(
        &self,
        owner: dir::GlobalSymbolId,
        substitution: &GenericSubstitution,
    ) -> CompilerResult<Option<GenericInstance>> {
        let Some(module) = self.modules.get(&owner.module_id) else {
            return Ok(None);
        };
        let mut arguments = module
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner == owner)
            .filter_map(|(variable, generic)| {
                let argument = if generic.is_type() {
                    substitution.type_variable(variable).map(ArgumentTerm::Type)
                } else {
                    substitution
                        .static_variable(variable)
                        .map(ArgumentTerm::Static)
                }?;

                Some((generic.slot().index, argument))
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
        if let Some(argument) = substitution.type_variable(variable) {
            return Ok(argument);
        }
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(variable);
        };
        let Some(substituted) = term.substitute(module, substitution, self)? else {
            return Ok(variable);
        };
        if let TypeTerm::Variable(variable) = substituted {
            return Ok(variable);
        }
        if substituted == term {
            return Ok(variable);
        }

        self.push_solved_type_variable(module, substituted)
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
        if let Some(argument) = substitution.static_variable(variable) {
            return Ok(argument);
        }
        let Some(term) = self.solved_static_term(variable)? else {
            return Ok(variable);
        };
        let substituted = term.substitute(module, substitution, self)?;
        if substituted == term {
            return Ok(variable);
        }

        self.push_solved_static_term_variable(module, substituted)
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
        if let Some((variable, symbol)) = self.type_pattern_generic(owner, pattern)? {
            return self.match_type_generic(module, variable, symbol, actual, substitution);
        }

        let pattern = self.reduce_pattern_type(pattern)?;
        let actual = self.reduce_pattern_type(actual)?;
        if let Some((variable, symbol)) = self.type_pattern_generic(owner, &pattern)? {
            return self.match_type_generic(module, variable, symbol, &actual, substitution);
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

    /// Match one generic argument pattern.
    pub(in crate::check) fn match_argument_pattern(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        pattern: &ArgumentTerm,
        actual: &ArgumentTerm,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let is_match = match (pattern, actual) {
            (ArgumentTerm::Type(pattern), ArgumentTerm::Type(actual))
            | (ArgumentTerm::SpreadType(pattern), ArgumentTerm::SpreadType(actual)) => {
                let pattern = TypeTerm::Variable(*pattern);
                let actual = TypeTerm::Variable(*actual);

                self.match_type_pattern(module, owner, &pattern, &actual, substitution)?
            }
            (ArgumentTerm::Static(pattern), ArgumentTerm::Static(actual))
            | (ArgumentTerm::SpreadStatic(pattern), ArgumentTerm::SpreadStatic(actual)) => {
                if let Some((variable, symbol)) = self.variable_static_generic(owner, *pattern)? {
                    self.match_static_generic(variable, symbol, *actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, *pattern, *actual)?
                        == Decision::Yes
                }
            }
            _ => false,
        };

        Ok(is_match)
    }

    /// Reduce variables and forms while matching type patterns.
    pub(in crate::check) fn reduce_pattern_type(
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
        symbol: Option<dir::GlobalSymbolId>,
        actual: &TypeTerm,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let actual = match actual {
            TypeTerm::Variable(variable) => *variable,
            term => self.push_solved_type_variable(module, term.clone())?,
        };
        if let Some(existing) = substitution.type_variable(variable) {
            let decision = self.decide_type_relation(TypeRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            symbol,
            argument: ArgumentTerm::Type(actual),
        });

        Ok(true)
    }

    /// Match one generic static slot against an actual static value.
    fn match_static_generic(
        &mut self,
        variable: VariableId,
        symbol: Option<dir::GlobalSymbolId>,
        actual: VariableId,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some(existing) = substitution.static_variable(variable) {
            let decision = self.decide_static_relation(StaticRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            symbol,
            argument: ArgumentTerm::Static(actual),
        });

        Ok(true)
    }

    /// Return a generic type slot represented by a pattern term.
    fn type_pattern_generic(
        &self,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let slot = match term {
            TypeTerm::Variable(variable) => self.variable_type_generic(owner, *variable)?,
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_type_generic(owner, *symbol)?,
            TypeTerm::Parameter { symbol } => self.symbol_type_generic(owner, *symbol)?,
            _ => None,
        };

        Ok(slot)
    }

    /// Return a generic type slot represented by a variable.
    fn variable_type_generic(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let module = self.module(variable.module)?;
        let VariableOrigin::Generic(generic) = &module.variable(variable).origin else {
            return Ok(None);
        };
        if generic.slot().owner != owner || !generic.is_type() {
            return Ok(None);
        }
        let symbol = match generic.slot().key {
            dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
            dir::GenericSlotKey::Generated(_) => None,
        };

        Ok(Some((variable, symbol)))
    }

    /// Return a generic static slot represented by a variable.
    fn variable_static_generic(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let module = self.module(variable.module)?;
        let VariableOrigin::Generic(generic) = &module.variable(variable).origin else {
            return Ok(None);
        };
        if generic.slot().owner != owner || !generic.is_static() {
            return Ok(None);
        }
        let symbol = match generic.slot().key {
            dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
            dir::GenericSlotKey::Generated(_) => None,
        };

        Ok(Some((variable, symbol)))
    }

    /// Return a generic type slot represented by a symbol.
    fn symbol_type_generic(
        &self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let module = self.module(symbol.module_id)?;
        let slot = module.generic_parameters().find_map(|(variable, generic)| {
            if generic.slot().owner == owner
                && generic.slot().key == dir::GenericSlotKey::Symbol(symbol)
                && generic.is_type()
            {
                Some((variable, Some(symbol)))
            } else {
                None
            }
        });

        Ok(slot)
    }
}
