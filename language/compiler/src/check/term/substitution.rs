use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, GenericArgument, GenericParameterId, Origin, StaticOperand, StaticTerm,
    TermId, TypeOperand, TypeTerm, VariableId,
};

/// One term substitution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Substitution {
    /// Replace one generic parameter with an argument.
    Generic {
        /// The generic parameter being substituted.
        parameter: GenericParameterId,
        /// The applied argument.
        argument: GenericArgument,
    },
    /// Replace `this` with a receiver type.
    Receiver {
        /// The receiver type.
        ty: TypeOperand,
    },
}

/// Type and static term substitution set.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct SubstitutionSet {
    /// The substitutions in application order.
    pub(in crate::check) entries: SmallVec<[Substitution; 4]>,
}

impl SubstitutionSet {
    /// Return an empty substitution.
    pub(in crate::check) fn empty() -> Self {
        Self {
            entries: SmallVec::new(),
        }
    }

    /// Return a substitution set with one receiver substitution.
    pub(in crate::check) fn with_receiver(ty: TypeOperand) -> Self {
        let mut substitutions = Self::empty();

        substitutions.receiver(ty);

        substitutions
    }

    /// Return whether this substitution has no entries.
    pub(in crate::check) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return whether this substitution set includes generic substitutions.
    pub(in crate::check) fn has_generics(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| matches!(entry, Substitution::Generic { .. }))
    }

    /// Add one generic substitution.
    pub(in crate::check) fn generic(
        &mut self,
        parameter: GenericParameterId,
        argument: GenericArgument,
    ) {
        self.entries.push(Substitution::Generic {
            parameter,
            argument,
        });
    }

    /// Add one receiver substitution.
    pub(in crate::check) fn receiver(&mut self, ty: TypeOperand) {
        self.entries.push(Substitution::Receiver { ty });
    }

    /// Return whether one generic parameter is substituted.
    pub(in crate::check) fn has_generic(&self, parameter: GenericParameterId) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                entry,
                Substitution::Generic {
                    parameter: candidate,
                    argument: _,
                } if *candidate == parameter
            )
        })
    }
}

impl CheckState<'_> {
    /// Return the type argument operand for one generic parameter.
    pub(in crate::check) fn substitution_type_operand(
        &self,
        substitution: &SubstitutionSet,
        parameter: GenericParameterId,
    ) -> Option<TypeOperand> {
        substitution.entries.iter().find_map(|entry| {
            let Substitution::Generic {
                parameter: candidate,
                argument,
            } = entry
            else {
                return None;
            };

            if *candidate == parameter {
                argument.type_operand()
            } else {
                None
            }
        })
    }

    /// Return the static argument operand for one generic parameter.
    pub(in crate::check) fn substitution_static_operand(
        &self,
        substitution: &SubstitutionSet,
        parameter: GenericParameterId,
    ) -> Option<StaticOperand> {
        substitution.entries.iter().find_map(|entry| {
            let Substitution::Generic {
                parameter: candidate,
                argument,
            } = entry
            else {
                return None;
            };

            if *candidate == parameter {
                argument.static_operand()
            } else {
                None
            }
        })
    }

    /// Return the type argument for one explicit generic symbol.
    pub(in crate::check) fn substitution_type_symbol_operand(
        &self,
        substitution: &SubstitutionSet,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<TypeOperand>> {
        for entry in &substitution.entries {
            let Substitution::Generic {
                parameter,
                argument,
            } = entry
            else {
                continue;
            };
            let generic = self.inference.generic_parameter_binding(*parameter)?;

            if generic.parameter().key == dir::GenericParameterKey::Symbol(symbol) {
                return Ok(argument.type_operand());
            }
        }

        Ok(None)
    }

    /// Return the receiver type operand in one substitution set.
    pub(in crate::check) fn substitution_receiver_operand(
        &self,
        substitution: &SubstitutionSet,
    ) -> Option<TypeOperand> {
        substitution.entries.iter().find_map(|entry| match entry {
            Substitution::Receiver { ty } => Some(*ty),
            Substitution::Generic { .. } => None,
        })
    }

    /// Substitute one type variable into an operand.
    pub(in crate::check) fn substitute_type_variable_operand(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        variable: VariableId,
    ) -> CompilerResult<TypeOperand> {
        if substitution.is_empty() {
            return Ok(variable.into());
        }

        let Some(solution) = self.solved_type_operand(variable) else {
            return Ok(variable.into());
        };
        let term = match solution {
            TypeOperand::Variable(_) => return Ok(solution),
            TypeOperand::Term(term) => {
                if let Some(operand) =
                    self.substitute_stored_type_term_directly(substitution, term)?
                {
                    return Ok(operand);
                }
                if !self
                    .inference
                    .term(term)
                    .needs_substitution(substitution, self)?
                {
                    return Ok(solution);
                }

                self.inference.term(term).clone()
            }
            TypeOperand::Type(ty) => TypeTerm::Type(ty),
        };
        if let Some(operand) = term.direct_substitution(substitution, self)? {
            return Ok(operand);
        }

        let Some(substituted) = term.substitute(module, substitution, self)? else {
            return Ok(solution);
        };
        let origin = self.variable(variable).source;
        let Answer::Ready(substituted) = self.reduce_type_operand(origin, substituted)? else {
            return Ok(solution);
        };

        Ok(substituted)
    }

    /// Substitute one stored type term when it is wholly replaced.
    fn substitute_stored_type_term_directly(
        &mut self,
        substitution: &SubstitutionSet,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let parameter = match self.inference.term(term) {
            TypeTerm::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            if let Some(argument) = self.substitution_type_operand(substitution, parameter) {
                return Ok(Some(argument));
            }
            if let Some(argument) = self.substitution_static_operand(substitution, parameter) {
                let term = self
                    .inference
                    .push_term(TypeTerm::StaticValue { value: argument });

                return Ok(Some(term.into()));
            }
        }

        let is_receiver = matches!(self.inference.term(term), TypeTerm::This);
        if is_receiver && let Some(receiver) = self.substitution_receiver_operand(substitution) {
            return Ok(Some(receiver));
        }

        Ok(None)
    }

    /// Substitute one type operand.
    pub(in crate::check) fn substitute_type_operand(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        operand: TypeOperand,
    ) -> CompilerResult<TypeOperand> {
        if substitution.is_empty() {
            return Ok(operand);
        }

        let operand = match operand {
            TypeOperand::Variable(variable) => {
                self.substitute_type_variable_operand(module, substitution, variable)?
            }
            TypeOperand::Term(term) => {
                if let Some(operand) =
                    self.substitute_stored_type_term_directly(substitution, term)?
                {
                    return Ok(operand);
                }
                if !self
                    .inference
                    .term(term)
                    .needs_substitution(substitution, self)?
                {
                    return Ok(operand);
                }

                let term = self.inference.term(term).clone();
                let Some(term) = term.substitute(module, substitution, self)? else {
                    return Ok(operand);
                };

                term
            }
            TypeOperand::Type(ty) => {
                let term = self.import_type_term(Origin::Type(ty), ty)?;
                if let Some(operand) = term.direct_substitution(substitution, self)? {
                    return Ok(operand);
                }

                let Some(term) = term.substitute(module, substitution, self)? else {
                    return Ok(operand);
                };

                term
            }
        };

        Ok(operand)
    }

    /// Substitute type operands.
    pub(in crate::check) fn substitute_type_operands(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        operands: &[TypeOperand],
    ) -> CompilerResult<Vec<TypeOperand>> {
        operands
            .iter()
            .map(|operand| self.substitute_type_operand(module, substitution, *operand))
            .collect()
    }

    /// Substitute one static variable into an operand.
    pub(in crate::check) fn substitute_static_variable_operand(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        variable: VariableId,
    ) -> CompilerResult<StaticOperand> {
        if substitution.is_empty() {
            return Ok(variable.into());
        }

        let Some(solution) = self.solved_static_operand(variable) else {
            return Ok(variable.into());
        };
        let operand = match solution {
            StaticOperand::Variable(_) => return Ok(solution),
            StaticOperand::Term(term) => {
                if let Some(operand) = self
                    .inference
                    .term(term)
                    .direct_substitution(substitution, self)
                {
                    return Ok(operand);
                }
                if !self
                    .inference
                    .term(term)
                    .needs_substitution(substitution, self)?
                {
                    return Ok(solution);
                }

                return self.substitute_static_term(module, substitution, term);
            }
            StaticOperand::Static(_) => solution,
        };

        Ok(operand)
    }

    /// Substitute one static operand.
    pub(in crate::check) fn substitute_static_operand(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        operand: StaticOperand,
    ) -> CompilerResult<StaticOperand> {
        if substitution.is_empty() {
            return Ok(operand);
        }

        let operand = match operand {
            StaticOperand::Variable(variable) => {
                self.substitute_static_variable_operand(module, substitution, variable)?
            }
            StaticOperand::Term(term) => {
                if let Some(operand) = self
                    .inference
                    .term(term)
                    .direct_substitution(substitution, self)
                {
                    return Ok(operand);
                }
                if !self
                    .inference
                    .term(term)
                    .needs_substitution(substitution, self)?
                {
                    return Ok(operand);
                }

                self.substitute_static_term(module, substitution, term)?
            }
            StaticOperand::Static(_) => operand,
        };

        Ok(operand)
    }

    /// Substitute one stored static term.
    fn substitute_static_term(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        term: TermId<StaticTerm>,
    ) -> CompilerResult<StaticOperand> {
        if let Some(operand) = self
            .inference
            .term(term)
            .direct_substitution(substitution, self)
        {
            return Ok(operand);
        }

        // substitute source expressions after static evaluation
        let expression = match self.inference.term(term) {
            StaticTerm::Expression(expression) => Some(*expression),
            _ => None,
        };
        if let Some(expression) = expression {
            let Some(term) = self.static_expression_term(expression)? else {
                return Ok(term.into());
            };
            let term = self.inference.push_term(term);

            return self.substitute_static_term(module, substitution, term);
        }

        // substitute union elements by indexed arena reads
        let union_len = match self.inference.term(term) {
            StaticTerm::Union { elements } => Some(elements.len()),
            _ => None,
        };
        if let Some(union_len) = union_len {
            let mut elements = Vec::with_capacity(union_len);

            for index in 0..union_len {
                let element = match self.inference.term(term) {
                    StaticTerm::Union { elements } => elements[index],
                    _ => return Ok(term.into()),
                };
                let element = self.substitute_static_operand(module, substitution, element)?;

                elements.push(element);
            }
            let term = self.inference.push_term(StaticTerm::Union { elements });

            return Ok(term.into());
        }

        // substitute layout target type
        let layout = match self.inference.term(term) {
            StaticTerm::Layout(layout) => Some(*layout),
            _ => None,
        };
        if let Some(layout) = layout {
            let layout = layout.substitute(module, substitution, self)?;
            let term = self.inference.push_term(StaticTerm::Layout(layout));

            return Ok(term.into());
        }

        // substitute member owner and arguments
        let member = match self.inference.term(term) {
            StaticTerm::Member {
                source,
                owner,
                key,
                arguments,
            } => Some((
                *source,
                *owner,
                *key,
                arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>(),
            )),
            _ => None,
        };
        if let Some((source, owner, key, arguments)) = member {
            let owner = self.substitute_type_operand(module, substitution, owner)?;
            let arguments = self
                .substitute_arguments(module, substitution, &arguments)?
                .into_iter()
                .collect();
            let term = self.inference.push_term(StaticTerm::Member {
                source,
                owner,
                key,
                arguments,
            });

            return Ok(term.into());
        }

        // substitute intrinsic arguments
        let intrinsic = match self.inference.term(term) {
            StaticTerm::Intrinsic { item, arguments } => Some((
                *item,
                arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>(),
            )),
            _ => None,
        };
        if let Some((item, arguments)) = intrinsic {
            let arguments = self
                .substitute_arguments(module, substitution, &arguments)?
                .into_iter()
                .collect();
            let term = self
                .inference
                .push_term(StaticTerm::Intrinsic { item, arguments });

            return Ok(term.into());
        }

        // substitute equality operands
        let equality = match self.inference.term(term) {
            StaticTerm::Equal {
                left,
                right,
                is_negated,
            } => Some((*left, *right, *is_negated)),
            _ => None,
        };
        if let Some((left, right, is_negated)) = equality {
            let left = self.substitute_static_operand(module, substitution, left)?;
            let right = self.substitute_static_operand(module, substitution, right)?;
            let term = self.inference.push_term(StaticTerm::Equal {
                left,
                right,
                is_negated,
            });

            return Ok(term.into());
        }

        // substitute type relation operands
        let relation = match self.inference.term(term) {
            StaticTerm::TypeRelation {
                relation,
                left,
                right,
            } => Some((*relation, *left, *right)),
            _ => None,
        };
        if let Some((relation, left, right)) = relation {
            let left = self.substitute_type_operand(module, substitution, left)?;
            let right = self.substitute_type_operand(module, substitution, right)?;
            let term = self.inference.push_term(StaticTerm::TypeRelation {
                relation,
                left,
                right,
            });

            return Ok(term.into());
        }

        // substitute conditional operands
        let conditional = match self.inference.term(term) {
            StaticTerm::Conditional {
                condition,
                then_value,
                else_value,
            } => Some((*condition, *then_value, *else_value)),
            _ => None,
        };
        if let Some((condition, then_value, else_value)) = conditional {
            let condition = self.substitute_static_operand(module, substitution, condition)?;
            let then_value = self.substitute_static_operand(module, substitution, then_value)?;
            let else_value = self.substitute_static_operand(module, substitution, else_value)?;
            let term = self.inference.push_term(StaticTerm::Conditional {
                condition,
                then_value,
                else_value,
            });

            return Ok(term.into());
        }

        // substitute static parameters
        let parameter = match self.inference.term(term) {
            StaticTerm::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            let Some(argument) = self.substitution_static_operand(substitution, parameter) else {
                return Ok(term.into());
            };
            let Some(argument) = self.resolved_static_operand(argument) else {
                return Ok(term.into());
            };

            return Ok(argument);
        }

        Ok(term.into())
    }
}
