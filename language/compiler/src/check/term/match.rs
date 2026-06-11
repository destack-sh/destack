use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Condition, Dependency, FormTerm, FunctionTerm, GenericArgument,
    GenericParameterId, GenericTemplateId, Origin, StaticOperand, StaticRelation, StaticTerm,
    SubstitutionSet, TermId, TypeOperand, TypeRelation, TypeTerm, VariableId,
};

impl CheckState<'_> {
    /// Match a type pattern and collect generic substitutions.
    pub(in crate::check) fn match_type_pattern(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template_symbol: dir::GlobalSymbolId,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let Some(template) = self.inference.generic_template_by_symbol(template_symbol) else {
            return Ok(Answer::Ready(false));
        };

        self.match_type_pattern_in_template(origin, module, template, pattern, actual, substitution)
    }

    /// Match a type pattern in one generic template.
    pub(in crate::check) fn match_type_pattern_in_template(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let mut candidate = substitution.clone();
        let is_match = self.match_type_pattern_in_template_candidate(
            origin,
            module,
            template,
            pattern,
            actual,
            &mut candidate,
        )?;

        // commit substitutions only after the whole pattern matches
        if is_match == Answer::Ready(true) {
            *substitution = candidate;
        }

        Ok(is_match)
    }

    /// Match a type pattern against a candidate substitution.
    fn match_type_pattern_in_template_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        // match contextual generic parameters
        if let Some(parameter) = self.type_pattern_generic(template, pattern)? {
            return self.match_type_generic(origin, parameter, actual.into(), substitution);
        }
        let pattern = self.contextual_type_operand(origin, pattern.into())?;
        let actual = self.contextual_type_operand(origin, actual.into())?;
        let Some(pattern) = self.type_operand_term_id(pattern)? else {
            return Ok(Answer::pending(pattern.dependencies(self)));
        };
        let Some(actual) = self.type_operand_term_id(actual)? else {
            return Ok(Answer::pending(actual.dependencies(self)));
        };

        if let Some(parameter) = self.type_pattern_generic(template, pattern)? {
            return self.match_type_generic(origin, parameter, actual.into(), substitution);
        }

        let is_match = match (self.inference.term(pattern), self.inference.term(actual)) {
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
                    Answer::Ready(false)
                } else {
                    self.match_reference_argument_patterns_in_template(
                        origin,
                        module,
                        template,
                        pattern,
                        actual,
                        substitution,
                    )?
                }
            }
            (TypeTerm::Union { elements: left }, TypeTerm::Union { elements: right })
            | (
                TypeTerm::Intersection { elements: left },
                TypeTerm::Intersection { elements: right },
            ) => {
                if left.len() != right.len() {
                    Answer::Ready(false)
                } else {
                    self.match_element_patterns_in_template(
                        origin,
                        module,
                        template,
                        pattern,
                        actual,
                        substitution,
                    )?
                }
            }
            (TypeTerm::Union { elements: _ }, _actual) => self.match_union_pattern_in_template(
                origin,
                module,
                template,
                pattern,
                actual,
                substitution,
            )?,
            (TypeTerm::Function(left), TypeTerm::Function(right)) => self
                .match_function_pattern_in_template(
                    origin,
                    module,
                    template,
                    *left,
                    *right,
                    substitution,
                )?,
            (
                TypeTerm::Form {
                    form: left_form,
                    payload: left_payload,
                },
                TypeTerm::Form {
                    form: right_form,
                    payload: right_payload,
                },
            ) => {
                let left_form = *left_form;
                let right_form = *right_form;
                let left_payload = *left_payload;
                let right_payload = *right_payload;
                let left_form = *self.inference.term(left_form);
                let right_form = *self.inference.term(right_form);
                let form = self.match_form_pattern_in_template(
                    origin,
                    template,
                    left_form,
                    right_form,
                    substitution,
                )?;
                if form != Answer::Ready(true) {
                    form
                } else {
                    self.match_type_operand_pattern_in_template(
                        origin,
                        module,
                        template,
                        left_payload,
                        right_payload,
                        substitution,
                    )?
                }
            }
            (left, right) => Answer::Ready(left == right),
        };

        Ok(is_match)
    }

    /// Match one form pattern against an actual form.
    fn match_form_pattern_in_template(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        pattern: FormTerm,
        actual: FormTerm,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let is_match = match (pattern, actual) {
            (
                FormTerm::Borrowed {
                    lifetime: pattern_lifetime,
                    access: pattern_access,
                },
                FormTerm::Borrowed {
                    lifetime: actual_lifetime,
                    access: actual_access,
                },
            ) => {
                let lifetime = self.match_static_pattern_in_template(
                    origin,
                    template,
                    pattern_lifetime,
                    actual_lifetime,
                    substitution,
                )?;
                if lifetime != Answer::Ready(true) {
                    return Ok(lifetime);
                }

                self.match_static_pattern_in_template(
                    origin,
                    template,
                    pattern_access,
                    actual_access,
                    substitution,
                )?
            }
            (FormTerm::Placed { place: pattern }, FormTerm::Placed { place: actual }) => self
                .match_static_pattern_in_template(
                    origin,
                    template,
                    pattern,
                    actual,
                    substitution,
                )?,
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Answer::Ready(true),
            _ => Answer::Ready(false),
        };

        Ok(is_match)
    }

    /// Match one static pattern against an actual static value.
    fn match_static_pattern_in_template(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        pattern: StaticOperand,
        actual: StaticOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        match self.static_pattern_parameter(pattern)? {
            Answer::Ready(Some(parameter)) => {
                if let Some(parameter) = self.parameter_static_generic(template, parameter)? {
                    self.match_static_generic(origin, parameter, actual, substitution)
                } else {
                    self.match_static_operand_pattern(pattern, actual)
                }
            }
            Answer::Ready(None) => self.match_static_operand_pattern(pattern, actual),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Match one generic argument pattern in one generic template.
    fn match_argument_pattern_in_template(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: &GenericArgument,
        actual: &GenericArgument,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let is_match = match (pattern, actual) {
            (GenericArgument::Type(pattern), GenericArgument::Type(actual))
            | (GenericArgument::SpreadType(pattern), GenericArgument::SpreadType(actual)) => self
                .match_type_argument_pattern(
                origin,
                module,
                template,
                *pattern,
                *actual,
                substitution,
            )?,
            (GenericArgument::Static(pattern), GenericArgument::Static(actual))
            | (GenericArgument::SpreadStatic(pattern), GenericArgument::SpreadStatic(actual)) => {
                self.match_static_argument_pattern(
                    origin,
                    template,
                    *pattern,
                    *actual,
                    substitution,
                )?
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.type_operand(), actual.type_operand()) =>
            {
                self.match_type_argument_pattern(
                    origin,
                    module,
                    template,
                    pattern,
                    actual,
                    substitution,
                )?
            }
            (pattern, actual)
                if let (Some(pattern), Some(actual)) =
                    (pattern.static_operand(), actual.static_operand()) =>
            {
                self.match_static_argument_pattern(origin, template, pattern, actual, substitution)?
            }
            _ => Answer::Ready(false),
        };

        Ok(is_match)
    }

    /// Match one type generic argument pattern.
    fn match_type_argument_pattern(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TypeOperand,
        actual: TypeOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        match self.type_operand_pattern_generic(template, pattern)? {
            Answer::Ready(Some(parameter)) => {
                return self.match_type_generic(origin, parameter, actual, substitution);
            }
            Answer::Ready(None) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        // infer covariantly from values already produced by an open actual
        if let TypeOperand::Variable(variable) = actual {
            return self.match_type_pattern_against_variable_bounds(
                origin,
                module,
                template,
                pattern,
                variable,
                substitution,
            );
        }

        let Some(pattern) = self.type_operand_term_id(pattern)? else {
            return Ok(Answer::pending(pattern.dependencies(self)));
        };
        let Some(actual) = self.type_operand_term_id(actual)? else {
            return Ok(Answer::pending(actual.dependencies(self)));
        };

        self.match_type_pattern_in_template(origin, module, template, pattern, actual, substitution)
    }

    /// Match one static generic argument pattern.
    fn match_static_argument_pattern(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        pattern: StaticOperand,
        actual: StaticOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        match self.static_pattern_generic(template, pattern)? {
            Answer::Ready(Some(parameter)) => {
                self.match_static_generic(origin, parameter, actual, substitution)
            }
            Answer::Ready(None) => self.match_static_operand_pattern(pattern, actual),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Match one concrete static pattern against one actual static value.
    fn match_static_operand_pattern(
        &mut self,
        pattern: StaticOperand,
        actual: StaticOperand,
    ) -> CompilerResult<Answer<bool>> {
        self.decide_static_relation(StaticRelation::Equal, pattern, actual)
    }

    /// Match reference argument patterns from stored terms.
    fn match_reference_argument_patterns_in_template(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let TypeTerm::Reference {
            arguments: patterns,
            ..
        } = self.inference.term(pattern)
        else {
            return Ok(Answer::Ready(false));
        };
        let len = patterns.len();
        let mut answer = Answer::Ready(true);

        // match arguments without owning either argument list
        for index in 0..len {
            let Some((pattern, actual)) =
                self.match_reference_argument_pair(pattern, actual, index)
            else {
                return Ok(Answer::Ready(false));
            };
            let is_match = self.match_argument_pattern_in_template(
                origin,
                module,
                template,
                &pattern,
                &actual,
                substitution,
            )?;

            if is_match == Answer::Ready(false) {
                return Ok(Answer::Ready(false));
            }
            answer = answer.and(is_match);
        }

        Ok(answer)
    }

    /// Return one pair of reference generic arguments.
    fn match_reference_argument_pair(
        &self,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        index: usize,
    ) -> Option<(GenericArgument, GenericArgument)> {
        let TypeTerm::Reference {
            arguments: patterns,
            ..
        } = self.inference.term(pattern)
        else {
            return None;
        };
        let TypeTerm::Reference {
            arguments: actuals, ..
        } = self.inference.term(actual)
        else {
            return None;
        };

        Some((patterns.get(index).copied()?, actuals.get(index).copied()?))
    }

    /// Match union or intersection element patterns from stored terms.
    fn match_element_patterns_in_template(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let Some(len) = self.type_element_len(pattern) else {
            return Ok(Answer::Ready(false));
        };
        let mut answer = Answer::Ready(true);

        // match elements without owning either element list
        for index in 0..len {
            let Some((pattern, actual)) = self.type_element_pair(pattern, actual, index) else {
                return Ok(Answer::Ready(false));
            };
            let is_match = self.match_type_operand_pattern_in_template(
                origin,
                module,
                template,
                pattern,
                actual,
                substitution,
            )?;

            if is_match == Answer::Ready(false) {
                return Ok(Answer::Ready(false));
            }
            answer = answer.and(is_match);
        }

        Ok(answer)
    }

    /// Return the element count for a union or intersection term.
    fn type_element_len(&self, term: TermId<TypeTerm>) -> Option<usize> {
        match self.inference.term(term) {
            TypeTerm::Union { elements } | TypeTerm::Intersection { elements } => {
                Some(elements.len())
            }
            _ => None,
        }
    }

    /// Return one pair of union or intersection elements.
    fn type_element_pair(
        &self,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        index: usize,
    ) -> Option<(TypeOperand, TypeOperand)> {
        let pattern = match self.inference.term(pattern) {
            TypeTerm::Union { elements } | TypeTerm::Intersection { elements } => {
                elements.get(index).copied()?
            }
            _ => return None,
        };
        let actual = match self.inference.term(actual) {
            TypeTerm::Union { elements } | TypeTerm::Intersection { elements } => {
                elements.get(index).copied()?
            }
            _ => return None,
        };

        Some((pattern, actual))
    }

    /// Match one union pattern against one actual type.
    fn match_union_pattern_in_template(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TermId<TypeTerm>,
        actual: TermId<TypeTerm>,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let Some(len) = self.type_element_len(pattern) else {
            return Ok(Answer::Ready(false));
        };
        let actual: TypeOperand = actual.into();
        let mut selected = None;
        let mut pending = Answer::Ready(false);

        // choose the single pattern branch that accepts the actual type
        for index in 0..len {
            let Some(pattern) = self.type_element(pattern, index) else {
                return Ok(Answer::Ready(false));
            };
            let mut candidate = substitution.clone();
            let is_match = self.match_type_operand_pattern_in_template(
                origin,
                module,
                template,
                pattern,
                actual,
                &mut candidate,
            )?;

            // keep unresolved branch ambiguity open
            if let Answer::Pending(blockers) = is_match {
                pending = pending.or(Answer::Pending(blockers));
            }
            // skip rejected branches
            else if is_match == Answer::Ready(false) {
                continue;
            }
            // reject ambiguous successful matches
            else if selected.is_some() {
                return Ok(Answer::Ready(false));
            } else {
                selected = Some(candidate);
            }
        }

        if pending.is_pending() {
            return Ok(pending);
        }
        if let Some(selected) = selected {
            *substitution = selected;

            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::Ready(false))
        }
    }

    /// Return one union or intersection element.
    fn type_element(&self, term: TermId<TypeTerm>, index: usize) -> Option<TypeOperand> {
        match self.inference.term(term) {
            TypeTerm::Union { elements } | TypeTerm::Intersection { elements } => {
                elements.get(index).copied()
            }
            _ => None,
        }
    }

    /// Match one function pattern against one actual function type.
    fn match_function_pattern_in_template(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TermId<FunctionTerm>,
        actual: TermId<FunctionTerm>,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let pattern_term = self.inference.term(pattern);
        let pattern_asynchrony = pattern_term.asynchrony;
        let pattern_is_generator = pattern_term.is_generator;
        let pattern_this = pattern_term.this_parameter;
        let pattern_return = pattern_term.return_type;
        let pattern_len = pattern_term.parameters.len();

        let actual_term = self.inference.term(actual);
        let actual_asynchrony = actual_term.asynchrony;
        let actual_is_generator = actual_term.is_generator;
        let actual_this = actual_term.this_parameter;
        let actual_return = actual_term.return_type;
        let actual_len = actual_term.parameters.len();

        if pattern_asynchrony != actual_asynchrony || pattern_is_generator != actual_is_generator {
            return Ok(Answer::Ready(false));
        }

        // require matching receiver shape without contravariant inference
        if pattern_this.is_some() != actual_this.is_some() {
            return Ok(Answer::Ready(false));
        }

        // require assignable parameter arity without contravariant inference
        if !self.function_accepts_arities(actual, pattern) {
            return Ok(Answer::Ready(false));
        }
        for index in 0..pattern_len.min(actual_len) {
            let pattern = self.inference.term(pattern).parameters[index];
            let actual = self.inference.term(actual).parameters[index];

            if pattern.is_rest != actual.is_rest {
                return Ok(Answer::Ready(false));
            }
        }

        // infer from return types covariantly
        if let (Some(pattern), Some(actual)) = (pattern_return, actual_return) {
            self.match_type_operand_pattern_in_template(
                origin,
                module,
                template,
                pattern,
                actual,
                substitution,
            )
        } else {
            Ok(Answer::Ready(pattern_return == actual_return))
        }
    }

    /// Return whether one function term accepts every target arity.
    fn function_accepts_arities(
        &self,
        source: TermId<FunctionTerm>,
        target: TermId<FunctionTerm>,
    ) -> bool {
        let source = self.function_required_parameter_count(source);
        let target = self.function_required_parameter_count(target);

        source <= target
    }

    /// Return the required runtime parameter count for one function term.
    fn function_required_parameter_count(&self, function: TermId<FunctionTerm>) -> usize {
        let function = self.inference.term(function);

        function
            .parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count()
    }

    /// Match one type operand pattern in one generic template.
    fn match_type_operand_pattern_in_template(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TypeOperand,
        actual: TypeOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        match self.type_operand_pattern_generic(template, pattern)? {
            Answer::Ready(Some(parameter)) => {
                return self.match_type_generic(origin, parameter, actual, substitution);
            }
            Answer::Ready(None) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        let Some(pattern) = self.type_operand_term_id(pattern)? else {
            return Ok(Answer::pending(pattern.dependencies(self)));
        };
        let Some(actual) = self.type_operand_term_id(actual)? else {
            return Ok(Answer::pending(actual.dependencies(self)));
        };

        self.match_type_pattern_in_template(origin, module, template, pattern, actual, substitution)
    }

    /// Match one type pattern against the produced bounds of one actual variable.
    fn match_type_pattern_against_variable_bounds(
        &mut self,
        origin: Origin,
        module: ModuleId,
        template: GenericTemplateId,
        pattern: TypeOperand,
        variable: VariableId,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let bounds = self.inference.lower_type_bounds(variable);
        let mut answer = Answer::Ready(true);
        let mut has_bound = false;

        // match each produced lower bound as covariant evidence
        for bound in bounds {
            if matches!(bound, TypeOperand::Variable(bound) if bound == variable) {
                continue;
            }

            has_bound = true;
            let bound = match self.reduce_type_bound(origin, variable, bound)? {
                Answer::Ready(bound) => bound,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let is_match = self.match_type_operand_pattern_in_template(
                origin,
                module,
                template,
                pattern,
                bound,
                substitution,
            )?;

            if is_match == Answer::Ready(false) {
                return Ok(Answer::Ready(false));
            }

            answer = answer.and(is_match);
        }

        // wait until the actual has produced at least one bound
        if !has_bound {
            return Ok(Answer::pending([Dependency::Variable(variable)]));
        }

        Ok(answer)
    }

    /// Return a generic type parameter represented by a type pattern operand.
    fn type_operand_pattern_generic(
        &mut self,
        template: GenericTemplateId,
        operand: TypeOperand,
    ) -> CompilerResult<Answer<Option<GenericParameterId>>> {
        let Some(pattern) = self.type_operand_term_id(operand)? else {
            return Ok(Answer::pending(operand.dependencies(self)));
        };

        let parameter = self.type_pattern_generic(template, pattern)?;

        Ok(Answer::Ready(parameter))
    }

    /// Match one generic type parameter against an actual type.
    fn match_type_generic(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
        actual: TypeOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        if let Some(existing) = self.substitution_type_operand(&*substitution, parameter) {
            let is_equal = self.decide_type_relation(TypeRelation::Equal, existing, actual)?;

            if is_equal == Answer::Ready(false) {
                return Ok(Answer::Ready(false));
            }

            self.constrain_type(
                origin,
                TypeRelation::Equal,
                existing,
                actual,
                Condition::Always,
            );
            return match is_equal {
                Answer::Ready(true) => Ok(Answer::Ready(true)),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                Answer::Ready(false) => Ok(Answer::Ready(false)),
            };
        }

        substitution.generic(parameter, GenericArgument::Type(actual));

        Ok(Answer::Ready(true))
    }

    /// Match one generic static parameter against an actual static value.
    fn match_static_generic(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
        actual: StaticOperand,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        if let Some(existing) = self.substitution_static_operand(&*substitution, parameter) {
            let is_equal = self.decide_static_relation(StaticRelation::Equal, existing, actual)?;

            if is_equal == Answer::Ready(false) {
                return Ok(Answer::Ready(false));
            }

            self.constrain_static(
                origin,
                StaticRelation::Equal,
                existing,
                actual,
                Condition::Always,
            );
            return match is_equal {
                Answer::Ready(true) => Ok(Answer::Ready(true)),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                Answer::Ready(false) => Ok(Answer::Ready(false)),
            };
        }

        substitution.generic(parameter, GenericArgument::Static(actual));

        Ok(Answer::Ready(true))
    }

    /// Return a generic type parameter represented by a pattern term.
    fn type_pattern_generic(
        &self,
        template: GenericTemplateId,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let parameter = match self.inference.term(term) {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_type_generic(template, *symbol)?,
            TypeTerm::Parameter(parameter_id) => {
                self.parameter_type_generic(template, *parameter_id)?
            }
            _ => None,
        };

        Ok(parameter)
    }

    /// Return a generic static parameter represented by a static pattern.
    fn static_pattern_generic(
        &mut self,
        template: GenericTemplateId,
        operand: StaticOperand,
    ) -> CompilerResult<Answer<Option<GenericParameterId>>> {
        let parameter = match self.static_pattern_parameter(operand)? {
            Answer::Ready(Some(parameter)) => parameter,
            Answer::Ready(None) => return Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let parameter = self.parameter_static_generic(template, parameter)?;

        Ok(Answer::Ready(parameter))
    }

    /// Return the generic static parameter represented by a static pattern.
    fn static_pattern_parameter(
        &mut self,
        operand: StaticOperand,
    ) -> CompilerResult<Answer<Option<GenericParameterId>>> {
        let parameter = match operand {
            StaticOperand::Variable(variable) => {
                match self.static_substitution_parameter(variable)? {
                    Answer::Ready(parameter) => parameter,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                }
            }
            StaticOperand::Term(term) => match self.inference.term(term) {
                StaticTerm::Parameter(parameter) => Some(*parameter),
                _ => None,
            },
            StaticOperand::Static(value) => match self.r#static(value) {
                dir::StaticTerm::Parameter(parameter) => Some(*parameter),
                _ => None,
            },
        };
        let Some(parameter) = parameter else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(parameter)))
    }

    /// Return the static parameter represented by a variable pattern.
    fn static_substitution_parameter(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Answer<Option<GenericParameterId>>> {
        if let Some(operand) = self.resolved_static_variable(variable) {
            let Some(term) = self.static_operand_term_id(operand) else {
                return Ok(Answer::Ready(None));
            };
            let parameter = match self.inference.term(term) {
                StaticTerm::Parameter(parameter) => Some(*parameter),
                _ => None,
            };

            return Ok(Answer::Ready(parameter));
        }
        let Origin::Node(node) = self.variable(variable).source else {
            return Ok(Answer::pending([Dependency::Variable(variable)]));
        };
        if node.local_id.ty != dir::NodeType::Expression {
            return Ok(Answer::pending([Dependency::Variable(variable)]));
        }

        Ok(Answer::Ready(None))
    }

    /// Return a generic type parameter represented by a symbol.
    fn symbol_type_generic(
        &self,
        template: GenericTemplateId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<GenericParameterId>> {
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
        template: GenericTemplateId,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let parameter = self
            .inference
            .generic_template_parameters(template)
            .find_map(|(parameter, generic)| {
                if generic.parameter().id == parameter_id && generic.is_type() {
                    Some(parameter)
                } else {
                    None
                }
            });

        Ok(parameter)
    }

    /// Return a generic static parameter represented by a parameter id.
    fn parameter_static_generic(
        &self,
        template: GenericTemplateId,
        parameter_id: GenericParameterId,
    ) -> CompilerResult<Option<GenericParameterId>> {
        let parameter = self
            .inference
            .generic_template_parameters(template)
            .find_map(|(parameter, generic)| {
                if generic.parameter().id == parameter_id && generic.is_static() {
                    Some(parameter)
                } else {
                    None
                }
            });

        Ok(parameter)
    }
}
