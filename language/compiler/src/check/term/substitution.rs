use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CheckState, Decision, GenericArgument, GenericInstance, GenericParameterBinding,
    GenericParameterId, Origin, StaticOperand, StaticRelation, StaticTerm, TypeOperand,
    TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

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
    ) -> Option<TypeOperand> {
        substitution.entries.iter().find_map(|entry| {
            let Substitution::Generic {
                parameter,
                argument,
            } = entry
            else {
                return None;
            };
            let generic = self.inference.require_generic_parameter(*parameter);

            if generic.parameter().key == dir::GenericParameterKey::Symbol(symbol) {
                argument.type_operand()
            } else {
                None
            }
        })
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

    /// Return the generic substitution for one applied symbol.
    pub(in crate::check) fn generic_substitution(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<SubstitutionSet> {
        if arguments.is_empty() {
            return Ok(SubstitutionSet::empty());
        }
        let Some(template) = self.inference.symbol_generic_template(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("generic arguments supplied for non-generic symbol {symbol:?}"),
            });
        };
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(_, generic)| {
                let parameter = generic.parameter().id();

                (parameter, generic.clone())
            })
            .collect::<Vec<_>>();

        let mut substitution = SubstitutionSet::empty();

        // collect already resolved generic entries in declaration order
        for ((parameter, generic), argument) in parameters.into_iter().zip(arguments.iter()) {
            let argument = self.resolve_substitution_argument(template, &generic, argument)?;

            substitution.generic(parameter, argument);
        }

        Ok(substitution)
    }

    /// Resolve one generic argument for substitution.
    fn resolve_substitution_argument(
        &self,
        template: dir::GlobalGenericTemplateId,
        generic: &GenericParameterBinding,
        argument: &GenericArgument,
    ) -> CompilerResult<GenericArgument> {
        let argument = match (generic, argument) {
            (
                GenericParameterBinding::Type { .. } | GenericParameterBinding::VariadicType { .. },
                GenericArgument::TypeOrStatic { source },
            ) => {
                let operand = self.node_type_operand(source.value.clone().into_any())?;

                GenericArgument::Type(operand)
            }
            (
                GenericParameterBinding::Type { .. } | GenericParameterBinding::VariadicType { .. },
                GenericArgument::SpreadTypeOrStatic { source },
            ) => {
                let operand = self.node_type_operand(source.value.clone().into_any())?;

                GenericArgument::SpreadType(operand)
            }
            (
                GenericParameterBinding::Static { .. }
                | GenericParameterBinding::VariadicStatic { .. },
                GenericArgument::TypeOrStatic { .. } | GenericArgument::SpreadTypeOrStatic { .. },
            ) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "static generic argument for {template:?} reached substitution before static lowering"
                    ),
                });
            }
            (_, argument) => argument.clone(),
        };

        Ok(argument)
    }

    /// Return the generic instance described by one substitution.
    pub(in crate::check) fn substitution_application(
        &mut self,
        symbol: dir::GlobalSymbolId,
        substitution: &SubstitutionSet,
    ) -> CompilerResult<Option<GenericInstance>> {
        let Some(template) = self.inference.symbol_generic_template(symbol) else {
            return Ok(None);
        };
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(_, generic)| {
                let parameter = generic.parameter().id();

                (parameter, generic.is_type())
            })
            .collect::<Vec<_>>();
        let arguments = parameters
            .into_iter()
            .filter_map(|(parameter, is_type)| {
                let argument = if is_type {
                    self.substitution_type_operand(substitution, parameter)
                        .map(GenericArgument::Type)
                } else {
                    self.substitution_static_operand(substitution, parameter)
                        .map(GenericArgument::Static)
                }?;

                Some(argument)
            })
            .collect::<Vec<_>>();

        if arguments.is_empty() {
            return Ok(None);
        }

        Ok(Some(GenericInstance::new(template, arguments.into())))
    }

    /// Substitute one type variable into an operand.
    pub(in crate::check) fn substitute_type_variable_operand(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        variable: VariableId,
    ) -> CompilerResult<TypeOperand> {
        let Some(term) = self.type_solution(variable)? else {
            return Ok(variable.into());
        };
        if let Some(operand) = self.direct_type_term_substitution(substitution, &term)? {
            return Ok(operand);
        }

        let Some(substituted) = term.substitute(module, substitution, self)? else {
            return Ok(variable.into());
        };
        if substituted == term {
            return Ok(variable.into());
        }
        let origin = self.variable(variable).source;
        let substituted = self.inference.push_term(substituted);
        let Some(substituted) = self.reduce_type_operand(origin, substituted.into())? else {
            return Ok(variable.into());
        };

        Ok(substituted)
    }

    /// Return the direct operand substitution for one type term.
    fn direct_type_term_substitution(
        &mut self,
        substitution: &SubstitutionSet,
        term: &TypeTerm,
    ) -> CompilerResult<Option<TypeOperand>> {
        // substitute type parameters directly to operands
        if let TypeTerm::Parameter(parameter) = term {
            if let Some(argument) = self.substitution_type_operand(substitution, *parameter) {
                return Ok(Some(argument));
            }
            if let Some(argument) = self.substitution_static_operand(substitution, *parameter) {
                let term = self
                    .inference
                    .push_term(TypeTerm::StaticValue { value: argument });

                return Ok(Some(term.into()));
            }
        }

        // substitute receiver placeholders directly to operands
        if matches!(term, TypeTerm::This)
            && let Some(receiver) = self.substitution_receiver_operand(substitution)
        {
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
        let operand = match operand {
            TypeOperand::Variable(variable) => {
                self.substitute_type_variable_operand(module, substitution, variable)?
            }
            TypeOperand::Term(term) => {
                let term = self.inference.term(term).clone();
                if let Some(operand) = self.direct_type_term_substitution(substitution, &term)? {
                    return Ok(operand);
                }

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
    pub(in crate::check) fn substitute_static_operand(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        operand: StaticOperand,
    ) -> CompilerResult<StaticOperand> {
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
        origin: Origin,
        module: ModuleId,
        template_symbol: dir::GlobalSymbolId,
        pattern: &TypeTerm,
        actual: &TypeTerm,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<bool> {
        if let Some(parameter) = self.type_pattern_generic(template_symbol, pattern)? {
            let actual = self.type_pattern_term_operand(actual);

            return self.match_type_generic(origin, parameter, actual, substitution);
        }

        let pattern = self.normalize_type_pattern_term(pattern)?;
        let actual = self.normalize_type_pattern_term(actual)?;
        if let Some(parameter) = self.type_pattern_generic(template_symbol, &pattern)? {
            let actual = self.type_pattern_term_operand(&actual);

            return self.match_type_generic(origin, parameter, actual, substitution);
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
                            origin,
                            module,
                            template_symbol,
                            left,
                            right,
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
        self.inference.push_term(term.clone()).into()
    }

    /// Match one generic argument pattern.
    pub(in crate::check) fn match_argument_pattern(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template_symbol: dir::GlobalSymbolId,
        pattern: &GenericArgument,
        actual: &GenericArgument,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<bool> {
        let is_match = match (pattern, actual) {
            (GenericArgument::Type(pattern), GenericArgument::Type(actual))
            | (GenericArgument::SpreadType(pattern), GenericArgument::SpreadType(actual)) => {
                if let Some(parameter) =
                    self.type_operand_pattern_generic(template_symbol, *pattern)?
                {
                    self.match_type_generic(origin, parameter, *actual, substitution)?
                } else {
                    let Some(pattern) = self.type_pattern_operand_term(*pattern)? else {
                        return Ok(false);
                    };
                    let Some(actual) = self.type_pattern_operand_term(*actual)? else {
                        return Ok(false);
                    };

                    self.match_type_pattern(
                        origin,
                        module,
                        template_symbol,
                        &pattern,
                        &actual,
                        substitution,
                    )?
                }
            }
            (GenericArgument::Static(pattern), GenericArgument::Static(actual))
            | (GenericArgument::SpreadStatic(pattern), GenericArgument::SpreadStatic(actual)) => {
                if let Some(parameter) = self.static_pattern_generic(template_symbol, *pattern)? {
                    self.match_static_generic(parameter, *actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, *pattern, *actual)?
                        == Decision::Yes
                }
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.type_operand(), actual.type_operand()) =>
            {
                if let Some(parameter) =
                    self.type_operand_pattern_generic(template_symbol, pattern)?
                {
                    self.match_type_generic(origin, parameter, actual, substitution)?
                } else {
                    let Some(pattern) = self.type_pattern_operand_term(pattern)? else {
                        return Ok(false);
                    };
                    let Some(actual) = self.type_pattern_operand_term(actual)? else {
                        return Ok(false);
                    };

                    self.match_type_pattern(
                        origin,
                        module,
                        template_symbol,
                        &pattern,
                        &actual,
                        substitution,
                    )?
                }
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.static_operand(), actual.static_operand()) =>
            {
                if let Some(parameter) = self.static_pattern_generic(template_symbol, pattern)? {
                    self.match_static_generic(parameter, actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, pattern, actual)?
                        == Decision::Yes
                }
            }
            _ => false,
        };

        Ok(is_match)
    }

    /// Return a generic type parameter represented by a type pattern operand.
    fn type_operand_pattern_generic(
        &self,
        template_symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(pattern) = self.type_pattern_operand_term(operand)? else {
            return Ok(None);
        };

        self.type_pattern_generic(template_symbol, &pattern)
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

    /// Match one generic type parameter against an actual type.
    fn match_type_generic(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
        actual: TypeOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.substitution_type_operand(&*substitution, parameter) {
            if self.decide_type_relation(TypeRelation::Equal, existing, actual)? == Decision::No {
                return Ok(false);
            }

            self.reduce_type_equality(origin, existing, actual)?;

            return Ok(true);
        }

        substitution.generic(parameter, GenericArgument::Type(actual));

        Ok(true)
    }

    /// Match one generic static parameter against an actual static value.
    fn match_static_generic(
        &mut self,
        parameter: GenericParameterId,
        actual: StaticOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<bool> {
        if let Some(existing) = self.substitution_static_operand(&*substitution, parameter) {
            if self.decide_static_relation(StaticRelation::Equal, existing, actual)? == Decision::No
            {
                return Ok(false);
            }

            self.reduce_static_equality(existing, actual)?;

            return Ok(true);
        }

        substitution.generic(parameter, GenericArgument::Static(actual));

        Ok(true)
    }

    /// Return a generic type parameter represented by a pattern term.
    fn type_pattern_generic(
        &self,
        template_symbol: dir::GlobalSymbolId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let parameter = match term {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_type_generic(template_symbol, *symbol)?,
            TypeTerm::Parameter(parameter_id) => {
                self.parameter_type_generic(template_symbol, *parameter_id)?
            }
            _ => None,
        };

        Ok(parameter)
    }

    /// Return a generic static parameter represented by a static pattern.
    fn static_pattern_generic(
        &self,
        template_symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let term = match operand {
            StaticOperand::Variable(variable) => self.static_substitution_source(variable)?,
            StaticOperand::Term(term) => Some(self.inference.term(term).clone()),
            StaticOperand::Static(value) => match self.r#static(value) {
                dir::StaticTerm::Parameter(parameter) => Some(StaticTerm::Parameter(*parameter)),
                term => Some(StaticTerm::Literal(term.clone())),
            },
        };
        let Some(StaticTerm::Parameter(parameter)) = term else {
            return Ok(None);
        };
        let generic = self.inference.require_generic_parameter(parameter);
        let is_match = self.inference.symbol_generic_template(template_symbol)
            == Some(generic.parameter().template)
            && generic.is_static()
            && self
                .inference
                .generic_template_parameters(generic.parameter().template)
                .any(|(candidate, _)| candidate == parameter);

        Ok(is_match.then_some(parameter))
    }

    /// Return a generic type parameter represented by a symbol.
    fn symbol_type_generic(
        &self,
        template_symbol: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(template) = self.inference.symbol_generic_template(template_symbol) else {
            return Ok(None);
        };
        let parameter = self
            .inference
            .generic_template_parameters(template)
            .find_map(|(parameter, generic)| {
                if generic.parameter().key == dir::GenericParameterKey::Symbol(symbol)
                    && generic.is_type()
                {
                    Some(parameter)
                } else {
                    None
                }
            });

        Ok(parameter)
    }

    /// Return a generic type parameter represented by a parameter id.
    fn parameter_type_generic(
        &self,
        template_symbol: dir::GlobalSymbolId,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let Some(template) = self.inference.symbol_generic_template(template_symbol) else {
            return Ok(None);
        };
        let parameter = self
            .inference
            .generic_template_parameters(template)
            .find_map(|(parameter, generic)| {
                if generic.parameter().id() == parameter_id && generic.is_type() {
                    Some(parameter)
                } else {
                    None
                }
            });

        Ok(parameter)
    }
}
