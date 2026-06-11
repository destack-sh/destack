use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CallArgument, CallFailure, CallableApplicability, CheckEvent, CheckState, Condition,
    FunctionParameter, FunctionTerm, GenericArgument, GenericInstance, GenericParameterId,
    GenericTemplateId, Origin, SubstitutionSet, TermId, TraceOperand, TupleElement,
    TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select one callable signature for actual call arguments.
    pub(in crate::check) fn select_callable_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        target_symbol: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: TermId<FunctionTerm>,
        generic_arguments: &[GenericArgument],
        arguments: &[CallArgument],
        expected: Option<VariableId>,
        receiver: Option<TypeOperand>,
    ) -> CompilerResult<CallableApplicability> {
        let signature_parameters = self.signature_generic_parameters(target_symbol, function);
        let call_site_parameters = self.call_site_generic_parameters(target_symbol, function);
        let accepts_generic_arguments = !call_site_parameters.is_empty();

        // reject impossible explicit generic arity
        if generic_arguments.len() > call_site_parameters.len()
            || (!accepts_generic_arguments && !generic_arguments.is_empty())
        {
            return Ok(CallableApplicability::Rejected(CallFailure::NoMatch));
        }

        // instantiate receiver and call generics
        let selected_instance = instance.as_ref();
        let (generic_instance, substitution) = self.instantiate_call_signature(
            module,
            source,
            target_symbol,
            receiver,
            selected_instance,
            function,
            &signature_parameters,
            generic_arguments,
            arguments,
        )?;
        let mut substitution = substitution;

        // infer omitted call generics from argument patterns
        let inference_decision = self.infer_call_arguments(
            origin,
            module,
            function,
            &signature_parameters,
            receiver,
            arguments,
            &mut substitution,
        )?;

        // apply receiver and generic arguments to the selected signature
        let mut function = self.substitute_function_term(module, function, &substitution)?;
        if let Some(instance) = selected_instance {
            let substitution = self.generic_instance_substitution(instance)?;

            function = function.substitute(module, &substitution, self)?;
        }
        function.generic_parameters.clear();

        // constrain call inputs against the selected signature
        self.constrain_call_receiver(origin, receiver, function.this_parameter)?;
        self.constrain_call_arguments(origin, arguments, &function.parameters)?;
        self.constrain_call_return(origin, &function, expected)?;

        // constrain inferred call generics
        if accepts_generic_arguments {
            self.apply_generic_argument_defaults(
                module,
                &substitution,
                &call_site_parameters,
                generic_arguments.len(),
            )?;
            self.constrain_call_generic_parameters(
                origin,
                module,
                &substitution,
                &call_site_parameters,
            )?;
        }

        // reduce final argument and result decisions
        let receiver_decision = self.decide_call_receiver(receiver, function.this_parameter)?;
        let argument_decision =
            self.decide_call_arguments(origin, source, arguments, &function.parameters)?;
        let return_decision = self.decide_call_return(&function, expected)?;
        let generic_parameters = self.decide_call_generic_parameters(
            origin,
            module,
            source,
            &substitution,
            &call_site_parameters,
        )?;
        self.record_event(CheckEvent::CallableSignature {
            origin,
            source,
            arguments: argument_decision.clone(),
            receiver: receiver_decision.clone(),
            return_type: return_decision.clone(),
            inference: inference_decision.clone(),
            generics: generic_parameters.clone(),
        });

        let inference_decision = match inference_decision {
            Answer::Ready(()) => Answer::Ready(true),
            Answer::Pending(blockers) => Answer::Pending(blockers),
        };
        let input_decision = receiver_decision
            .and(argument_decision)
            .and(generic_parameters)
            .and(inference_decision);
        match input_decision {
            Answer::Ready(true) => match return_decision {
                Answer::Ready(true) => Ok(CallableApplicability::Applicable {
                    instance: generic_instance,
                    function,
                }),
                Answer::Pending(blockers) => Ok(CallableApplicability::Pending(blockers)),
                Answer::Ready(false) => {
                    self.call_signature_rejected(arguments, &function.parameters)
                }
            },
            Answer::Pending(blockers) => Ok(CallableApplicability::Pending(blockers)),
            Answer::Ready(false) => self.call_signature_rejected(arguments, &function.parameters),
        }
    }

    /// Return every generic parameter applied by one selected signature.
    fn signature_generic_parameters(
        &self,
        target: Option<dir::GlobalSymbolId>,
        function: TermId<FunctionTerm>,
    ) -> SmallVec<[GenericParameterId; 2]> {
        let Some(target) = target else {
            let mut parameters = SmallVec::new();

            self.extend_function_generic_parameters(function, &mut parameters);

            return parameters;
        };
        let Some(template) = self.inference.generic_template_by_symbol(target) else {
            let mut parameters = SmallVec::new();

            self.extend_function_generic_parameters(function, &mut parameters);

            return parameters;
        };
        let mut parameters = SmallVec::new();

        self.extend_template_parameters(template, &mut parameters);

        parameters
    }

    /// Return generic parameters supplied by a call expression.
    fn call_site_generic_parameters(
        &self,
        target: Option<dir::GlobalSymbolId>,
        function: TermId<FunctionTerm>,
    ) -> SmallVec<[GenericParameterId; 2]> {
        let Some(target) = target else {
            let mut parameters = SmallVec::new();

            self.extend_function_generic_parameters(function, &mut parameters);

            return parameters;
        };
        let Some(template) = self.inference.generic_template_by_symbol(target) else {
            return SmallVec::new();
        };

        self.template_parameters(template)
    }

    /// Append one function's direct generic parameters.
    fn extend_function_generic_parameters(
        &self,
        function: TermId<FunctionTerm>,
        parameters: &mut SmallVec<[GenericParameterId; 2]>,
    ) {
        let function = self.inference.term(function);

        // preserve function generic parameter order
        for parameter in &function.generic_parameters {
            parameters.push(*parameter);
        }
    }

    /// Append one template's parent and local parameters.
    fn extend_template_parameters(
        &self,
        template: GenericTemplateId,
        parameters: &mut SmallVec<[GenericParameterId; 2]>,
    ) {
        let Some(template) = self.inference.generic_template(template) else {
            return;
        };

        // collect enclosing parameters before local parameters
        if let Some(parent) = template.parent {
            self.extend_template_parameters(parent, parameters);
        }

        // collect local parameters in declaration order
        for parameter in &template.parameters {
            parameters.push(*parameter);
        }
    }

    /// Return parameters declared directly by one generic template.
    fn template_parameters(
        &self,
        template: GenericTemplateId,
    ) -> SmallVec<[GenericParameterId; 2]> {
        self.inference
            .generic_template_parameters(template)
            .map(|(parameter, _)| parameter)
            .collect()
    }

    /// Substitute one stored function term.
    pub(in crate::check) fn substitute_function_term(
        &mut self,
        module: ModuleId,
        function: TermId<FunctionTerm>,
        substitution: &SubstitutionSet,
    ) -> CompilerResult<FunctionTerm> {
        let source = self.inference.term(function);
        let asynchrony = source.asynchrony;
        let this_parameter = source.this_parameter;
        let parameter_len = source.parameters.len();
        let return_type = source.return_type;
        let is_generator = source.is_generator;
        let mut generic_parameters = SmallVec::<[GenericParameterId; 2]>::new();
        let mut parameters = SmallVec::with_capacity(parameter_len);

        // retain unapplied generic parameters
        for parameter in &self.inference.term(function).generic_parameters {
            if !substitution.has_generic(*parameter) {
                generic_parameters.push(*parameter);
            }
        }

        // substitute runtime parameters in declaration order
        for index in 0..parameter_len {
            let parameter = self.inference.term(function).parameters[index];
            let parameter = parameter.substitute(module, substitution, self)?;

            parameters.push(parameter);
        }

        let function = FunctionTerm {
            asynchrony,
            generic_parameters,
            this_parameter: this_parameter
                .map(|parameter| self.substitute_type_operand(module, substitution, parameter))
                .transpose()?,
            parameters,
            return_type: return_type
                .map(|return_type| self.substitute_type_operand(module, substitution, return_type))
                .transpose()?,
            is_generator,
        };

        Ok(function)
    }

    /// Return the precise rejection for a failed callable signature.
    fn call_signature_rejected(
        &mut self,
        arguments: &[CallArgument],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<CallableApplicability> {
        if let Some((argument, parameter)) =
            self.call_argument_type_failure(arguments, parameters)?
        {
            Ok(CallableApplicability::Rejected(CallFailure::ArgumentType {
                argument,
                parameter,
            }))
        } else {
            Ok(CallableApplicability::Rejected(CallFailure::NoMatch))
        }
    }

    /// Constrain call instantiation arguments by declared generic parameters.
    fn constrain_call_generic_parameters(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameters: &[GenericParameterId],
    ) -> CompilerResult<()> {
        // constrain each substituted generic argument by its parameter declaration
        for parameter in parameters {
            let type_constraint = self
                .inference
                .generic_parameter_binding(*parameter)?
                .type_constraint();
            let static_ty = self
                .inference
                .generic_parameter_binding(*parameter)?
                .static_ty();

            self.constrain_call_generic_parameter(
                origin,
                module,
                substitution,
                *parameter,
                type_constraint,
                static_ty,
            )?;
        }

        Ok(())
    }

    /// Constrain one call instantiation argument by its generic parameter declaration.
    fn constrain_call_generic_parameter(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameter: GenericParameterId,
        type_constraint: Option<TypeOperand>,
        static_ty: Option<TypeOperand>,
    ) -> CompilerResult<()> {
        match (type_constraint, static_ty) {
            (Some(constraint), None) => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Ok(());
                };
                let constraint = self.substitute_type_operand(module, substitution, constraint)?;

                self.constrain_type(
                    origin,
                    TypeRelation::Assignable,
                    argument,
                    constraint,
                    Condition::Always,
                );
            }
            (None, Some(ty)) => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Ok(());
                };
                let ty = self.substitute_type_operand(module, substitution, ty)?;

                self.constrain_static_value_type(origin, argument, ty)?;
            }
            (None, None) => {}
            (Some(_), Some(_)) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "generic parameter {parameter:?} has both a type constraint and static value type"
                    ),
                });
            }
        }

        Ok(())
    }

    /// Decide whether call generic arguments satisfy declared generic parameters.
    fn decide_call_generic_parameters(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        substitution: &SubstitutionSet,
        parameters: &[GenericParameterId],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);

        // combine each generic parameter declaration
        for parameter in parameters {
            let generic = self.inference.generic_parameter_binding(*parameter)?;
            let is_type = generic.is_type();
            let type_constraint = generic.type_constraint();
            let static_ty = generic.static_ty();

            let parameter_decision = self.decide_call_generic_parameter(
                origin,
                module,
                substitution,
                *parameter,
                type_constraint,
                static_ty,
            )?;
            let argument = if is_type {
                self.substitution_type_operand(substitution, *parameter)
                    .map(TraceOperand::Type)
            } else {
                self.substitution_static_operand(substitution, *parameter)
                    .map(TraceOperand::Static)
            };

            self.record_event(CheckEvent::CallableGeneric {
                origin,
                source,
                parameter: *parameter,
                argument,
                decision: parameter_decision.clone(),
            });

            decision = decision.and(parameter_decision);
        }

        Ok(decision)
    }

    /// Decide whether one call generic argument satisfies its parameter declaration.
    fn decide_call_generic_parameter(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameter: GenericParameterId,
        type_constraint: Option<TypeOperand>,
        static_ty: Option<TypeOperand>,
    ) -> CompilerResult<Answer<bool>> {
        match (type_constraint, static_ty) {
            (Some(constraint), None) => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Err(CompilerError::Internal {
                        message: format!("generic parameter {parameter:?} has no type argument"),
                    });
                };
                let constraint = self.substitute_type_operand(module, substitution, constraint)?;
                let Answer::Ready(argument) = self.reduce_type_operand(origin, argument)? else {
                    return Ok(Answer::pending(argument.dependencies(self)));
                };
                let Answer::Ready(constraint) = self.reduce_type_operand(origin, constraint)?
                else {
                    return Ok(Answer::pending(constraint.dependencies(self)));
                };

                let decision =
                    self.decide_type_relation(TypeRelation::Assignable, argument, constraint)?;

                Ok(decision)
            }
            (None, Some(ty)) => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Err(CompilerError::Internal {
                        message: format!("generic parameter {parameter:?} has no static argument"),
                    });
                };
                let ty = self.substitute_type_operand(module, substitution, ty)?;

                let decision = self.decide_static_value_type(origin, argument, ty)?;

                Ok(decision)
            }
            (None, None) => Ok(Answer::Ready(true)),
            (Some(_), Some(_)) => Err(CompilerError::Internal {
                message: format!(
                    "generic parameter {parameter:?} has both a type constraint and static value type"
                ),
            }),
        }
    }

    /// Decide whether an actual receiver fits one explicit receiver parameter.
    fn decide_call_receiver(
        &mut self,
        receiver: Option<TypeOperand>,
        parameter: Option<TypeOperand>,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (receiver, parameter) {
            // no explicit receiver parameter
            (_, None) => Answer::Ready(true),
            // unbound function value cannot satisfy an explicit receiver
            (None, Some(_)) => Answer::Ready(false),
            // actual receiver must satisfy the explicit receiver parameter
            (Some(receiver), Some(parameter)) => {
                self.decide_type_relation(TypeRelation::Assignable, receiver, parameter)?
            }
        };

        Ok(decision)
    }

    /// Decide whether arguments are assignable to parameters.
    fn decide_call_arguments(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        arguments: &[CallArgument],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<Answer<bool>> {
        if !self.call_arity_accepts(arguments, parameters) {
            return Ok(Answer::Ready(false));
        }
        let mut decision = Answer::Ready(true);

        // every argument must be assignable to the corresponding parameter
        for (index, parameter_index, argument, parameter) in
            Self::call_argument_parameters(arguments, parameters)
        {
            let Answer::Ready(Some(parameter)) = self.call_argument_parameter_type(
                origin,
                index,
                parameter_index,
                argument,
                parameter,
            )?
            else {
                return Ok(Answer::pending(argument.ty.dependencies(self)));
            };
            let result =
                self.decide_type_relation(TypeRelation::Assignable, argument.ty, parameter)?;
            self.record_event(CheckEvent::CallableArgument {
                origin,
                source,
                index,
                argument: argument.ty,
                parameter,
                decision: result.clone(),
            });

            decision = decision.and(result);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the first solved argument type failure.
    fn call_argument_type_failure(
        &mut self,
        arguments: &[CallArgument],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<Option<(TypeOperand, TypeOperand)>> {
        if !self.call_arity_accepts(arguments, parameters) {
            return Ok(None);
        }

        for (index, parameter_index, argument, parameter) in
            Self::call_argument_parameters(arguments, parameters)
        {
            let origin = Origin::Node(argument.source);
            let Answer::Ready(Some(parameter)) = self.call_argument_parameter_type(
                origin,
                index,
                parameter_index,
                argument,
                parameter,
            )?
            else {
                return Ok(None);
            };

            let decision =
                self.decide_type_relation(TypeRelation::Assignable, argument.ty, parameter)?;
            if decision == Answer::Ready(false) {
                return Ok(Some((argument.ty, parameter)));
            }
        }

        Ok(None)
    }

    /// Decide whether the return type can flow into the expected result.
    fn decide_call_return(
        &mut self,
        function: &FunctionTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Answer<bool>> {
        let Some(expected) = expected else {
            return Ok(Answer::Ready(true));
        };
        let Some(expected) = self.resolved_type_variable(expected) else {
            return Ok(Answer::Ready(true));
        };
        let Some(return_type) = function.return_type else {
            let void = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Void));

            return self.decide_type_relation(TypeRelation::Assignable, void, expected);
        };

        self.decide_type_relation(TypeRelation::Assignable, return_type, expected)
    }

    /// Constrain an actual receiver by one explicit receiver parameter.
    fn constrain_call_receiver(
        &mut self,
        origin: Origin,
        receiver: Option<TypeOperand>,
        parameter: Option<TypeOperand>,
    ) -> CompilerResult<()> {
        if let (Some(receiver), Some(parameter)) = (receiver, parameter) {
            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                receiver,
                parameter,
                Condition::Always,
            );
        }

        Ok(())
    }

    /// Constrain accepted call arguments by parameter types.
    fn constrain_call_arguments(
        &mut self,
        origin: Origin,
        arguments: &[CallArgument],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<()> {
        for (index, parameter_index, argument, parameter) in
            Self::call_argument_parameters(arguments, parameters)
        {
            let Answer::Ready(Some(parameter)) = self.call_argument_parameter_type(
                origin,
                index,
                parameter_index,
                argument,
                parameter,
            )?
            else {
                continue;
            };

            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                argument.ty,
                parameter,
                Condition::Always,
            );
        }

        Ok(())
    }

    /// Return whether runtime arguments fit one function parameter list.
    fn call_arity_accepts(
        &self,
        arguments: &[CallArgument],
        parameters: &[FunctionParameter],
    ) -> bool {
        let required = parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = parameters.last().is_some_and(|parameter| parameter.is_rest);

        arguments.len() >= required && (has_rest || arguments.len() <= parameters.len())
    }

    /// Return call arguments paired with their accepted parameter.
    pub(in crate::check) fn call_argument_parameters<'a>(
        arguments: &'a [CallArgument],
        parameters: &'a [FunctionParameter],
    ) -> impl Iterator<Item = (usize, usize, CallArgument, &'a FunctionParameter)> + 'a {
        arguments
            .iter()
            .copied()
            .enumerate()
            .filter_map(
                move |(index, argument)| match Self::call_parameter(index, parameters) {
                    Some((parameter_index, parameter)) => {
                        Some((index, parameter_index, argument, parameter))
                    }
                    None => None,
                },
            )
    }

    /// Return the parameter that receives one call argument.
    fn call_parameter(
        index: usize,
        parameters: &[FunctionParameter],
    ) -> Option<(usize, &FunctionParameter)> {
        if index < parameters.len() {
            Some((index, &parameters[index]))
        } else {
            parameters
                .last()
                .filter(|parameter| parameter.is_rest)
                .map(|parameter| (parameters.len() - 1, parameter))
        }
    }

    /// Return the type expected by one argument and parameter pair.
    pub(in crate::check) fn call_argument_parameter_type(
        &mut self,
        origin: Origin,
        index: usize,
        parameter_index: usize,
        argument: CallArgument,
        parameter: &FunctionParameter,
    ) -> CompilerResult<Answer<Option<TypeOperand>>> {
        if !parameter.is_rest || argument.is_spread {
            return Ok(Answer::Ready(Some(parameter.ty)));
        }
        let rest_index = index - parameter_index;

        self.rest_parameter_argument_type(origin, parameter.ty, rest_index)
    }

    /// Return the element type accepted by one rest parameter at one offset.
    fn rest_parameter_argument_type(
        &mut self,
        origin: Origin,
        parameter: TypeOperand,
        index: usize,
    ) -> CompilerResult<Answer<Option<TypeOperand>>> {
        let Answer::Ready(parameter) = self.reduce_type_operand(origin, parameter)? else {
            return Ok(Answer::pending(parameter.dependencies(self)));
        };
        let Some(term) = self.type_operand_term_id(parameter)? else {
            return Ok(Answer::pending(parameter.dependencies(self)));
        };
        let ty = match self.inference.term(term) {
            TypeTerm::Array { element }
            | TypeTerm::Slice { element }
            | TypeTerm::FixedArray { element, length: _ } => Some(*element),
            TypeTerm::Tuple { elements, .. } => Self::rest_tuple_argument_type(&elements, index),
            TypeTerm::Union { elements: _ } => {
                let Answer::Ready(ty) = self.rest_union_argument_type(origin, term, index)? else {
                    return Ok(Answer::pending(parameter.dependencies(self)));
                };

                ty
            }
            _ => None,
        };

        Ok(Answer::Ready(ty))
    }

    /// Return the element type accepted by one rest tuple at one offset.
    fn rest_tuple_argument_type(elements: &[TupleElement], index: usize) -> Option<TypeOperand> {
        let mut position = 0;

        // find the positional element or trailing rest element
        for element in elements {
            if element.is_rest {
                return Some(element.ty);
            }
            if position == index {
                return Some(element.ty);
            }

            position += 1;
        }

        None
    }

    /// Return the element type accepted by one union rest parameter at one offset.
    fn rest_union_argument_type(
        &mut self,
        origin: Origin,
        term: TermId<TypeTerm>,
        index: usize,
    ) -> CompilerResult<Answer<Option<TypeOperand>>> {
        let Some(len) = self.rest_union_len(term) else {
            let operand = TypeOperand::Term(term);

            return Ok(Answer::pending(operand.dependencies(self)));
        };
        let mut alternatives = Vec::new();

        // collect every union branch that accepts this rest position
        for element in 0..len {
            let Some(element) = self.rest_union_element(term, element) else {
                let operand = TypeOperand::Term(term);

                return Ok(Answer::pending(operand.dependencies(self)));
            };

            match self.rest_parameter_argument_type(origin, element, index)? {
                Answer::Ready(Some(ty)) => alternatives.push(ty),
                Answer::Ready(None) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }
        let ty = match alternatives.as_slice() {
            [] => None,
            [ty] => Some(*ty),
            _ => {
                let term = TypeTerm::Union {
                    elements: alternatives,
                };

                Some(self.inference.push_term(term).into())
            }
        };

        Ok(Answer::Ready(ty))
    }

    /// Return the number of elements in one rest union term.
    fn rest_union_len(&self, term: TermId<TypeTerm>) -> Option<usize> {
        let TypeTerm::Union { elements } = self.inference.term(term) else {
            return None;
        };

        Some(elements.len())
    }

    /// Return one element from a rest union term.
    fn rest_union_element(&self, term: TermId<TypeTerm>, index: usize) -> Option<TypeOperand> {
        let TypeTerm::Union { elements } = self.inference.term(term) else {
            return None;
        };

        elements.get(index).copied()
    }

    /// Constrain one accepted call return by the expected result type.
    pub(in crate::check) fn constrain_call_return(
        &mut self,
        origin: Origin,
        function: &FunctionTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<()> {
        let Some(expected) = expected else {
            return Ok(());
        };
        let Some(return_type) = function.return_type else {
            let void = TypeTerm::Literal(TypeLiteralTerm::Void);
            let void = self.inference.push_term(void);

            self.constrain_type(
                origin,
                TypeRelation::Assignable,
                void,
                expected,
                Condition::Always,
            );

            return Ok(());
        };

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            return_type,
            expected,
            Condition::Always,
        );

        Ok(())
    }
}
