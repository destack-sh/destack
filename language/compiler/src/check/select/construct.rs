use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, ConstructResult, Decision, GenericArgumentMode, GenericTemplateId, Origin,
    Relation, TypeRewrite, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the construction meaning of one new expression.
    pub(in crate::check) fn select_construct(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        result: ConstructResult,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        // collect argument types from walked inputs
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in argument_nodes {
            let ty = answer!(self.argument_type(module, *argument)?);
            arguments.push(ty);
        }

        // close the constructed annotation
        let annotation = ty.into_global_any(module);
        let target = answer!(self.node_type_answer(annotation)?);
        let target = answer!(self.reduce_type_root(origin, target)?);
        let instance = match self.ty(target)? {
            dir::Type::Instance(instance) => instance.clone(),
            _ => return self.reject_not_constructible(node, origin, target, ""),
        };

        // only classes construct through new
        let constructors = match self.definition(instance.symbol) {
            Some(dir::Definition::Struct(_)) => {
                return self.reject_not_constructible(
                    node,
                    origin,
                    target,
                    "; construct value types with 'T { … }'",
                );
            }
            Some(dir::Definition::Newtype(_)) => {
                return self.reject_not_constructible(
                    node,
                    origin,
                    target,
                    "; construct newtypes with 'T(…)'",
                );
            }
            Some(dir::Definition::Class(definition)) => {
                if definition.is_abstract {
                    self.report_cannot_construct_abstract_type(origin, target)?;
                    self.record_decision(node, Decision::Rejected)?;

                    return Ok(Answer::Ready(()));
                }

                let mut active = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
                answer!(self.class_construct_candidates(
                    origin,
                    target,
                    &instance,
                    definition.constructors.clone(),
                    definition.extends.clone(),
                    &mut active,
                )?)
            }
            _ => return self.reject_not_constructible(node, origin, target, ""),
        };
        if constructors.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("class {:?} has no construct candidates", instance.symbol),
            });
        }

        // try constructors in declaration order
        for constructor in constructors {
            let attempt = self.attempt_construct(
                origin,
                module,
                &instance,
                target,
                constructor.ty,
                &arguments,
            )?;

            if let Some((parameters, return_type)) = answer!(attempt) {
                return self.record_construct(
                    node,
                    module,
                    argument_nodes,
                    &instance,
                    constructor.constructor,
                    parameters,
                    return_type,
                    result,
                );
            }
        }

        self.reject_construct(node, origin, &arguments)
    }

    /// Return construct candidates for one class instance.
    fn class_construct_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        constructors: Vec<dir::ClassConstructorDefinition>,
        extends: Option<dir::NominalHeritage>,
        active: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<Answer<Vec<dir::ClassConstructorDefinition>>> {
        if !constructors.is_empty() {
            return Ok(Answer::Ready(constructors));
        }

        let Some(extends) = extends else {
            return Err(CompilerError::Internal {
                message: format!("class {:?} has no construct candidates", instance.symbol),
            });
        };

        self.forwarded_class_construct_candidates(origin, receiver, &extends, active)
    }

    /// Return constructors forwarded from one base class.
    fn forwarded_class_construct_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        extends: &dir::NominalHeritage,
        active: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<Answer<Vec<dir::ClassConstructorDefinition>>> {
        if active.contains(&extends.symbol) {
            return Ok(Answer::Ready(Vec::new()));
        }
        active.push(extends.symbol);

        let base = match self.definition(extends.symbol) {
            Some(dir::Definition::Class(base)) => base.clone(),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("base class {:?} has no checked definition", extends.symbol),
                });
            }
        };
        let source = self.origin_source_node(origin)?;
        let module = origin.module();
        let instance = dir::GenericInstance {
            symbol: extends.symbol,
            arguments: extends.arguments.clone(),
        };
        let base_receiver =
            self.push_type(module, dir::Type::Instance(instance.clone()), source)?;
        let base_constructors = answer!(self.class_construct_candidates(
            origin,
            base_receiver,
            &instance,
            base.constructors,
            base.extends,
            active,
        )?);
        active.pop();

        let substitution = self
            .instance_substitution(&instance)?
            .with_receiver(receiver);
        let mut constructors = Vec::with_capacity(base_constructors.len());
        for base_constructor in base_constructors {
            let constructor = base_constructor.constructor.forwarded(extends.symbol);
            let ty = if substitution.is_empty() {
                base_constructor.ty
            } else {
                self.fold_type(module, source, base_constructor.ty, substitution.rewrite())?
            };
            let ty = self.class_constructor_returning(origin, ty, receiver)?;

            constructors.push(dir::ClassConstructorDefinition { constructor, ty });
        }

        Ok(Answer::Ready(constructors))
    }

    /// Return one constructor signature with a replaced return type.
    fn class_constructor_returning(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let dir::Type::FunctionSignature(mut function) = self.ty(ty)?.clone() else {
            return Err(CompilerError::Internal {
                message: format!("class constructor type {ty:?} is not a function signature"),
            });
        };
        function.return_type = Some(receiver);

        self.push_type_at_origin(origin, dir::Type::FunctionSignature(function))
    }

    /// Attempt one constructor candidate against collected arguments.
    fn attempt_construct(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
        function_type: dir::GlobalTypeId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<(Vec<dir::FunctionParameterType>, dir::GlobalTypeId)>>> {
        let source = self.origin_source_node(origin)?;

        // close the constructor shape first
        let function_type = answer!(self.reduce_type_root(origin, function_type)?);
        let function = match self.ty(function_type)? {
            dir::Type::FunctionSignature(function) => function.clone(),
            _ => return Ok(Answer::Ready(None)),
        };
        let return_type = function.return_type;

        // infer omitted class arguments while testing this constructor
        let template = self.symbol_template(instance.symbol);
        if instance.arguments.is_empty()
            && let Some(template) = template
        {
            let probe = self.begin_probe();
            let parameters = self.generic_template_parameters(template);
            let attempt = self.attempt_signature(
                origin,
                module,
                source,
                &parameters,
                &[],
                &function,
                return_type,
                arguments,
            )?;

            return match attempt {
                Answer::Ready(Some(accepted)) => {
                    self.commit_probe(probe);

                    Ok(Answer::Ready(Some((
                        accepted.parameters.to_vec(),
                        accepted.return_type,
                    ))))
                }
                Answer::Ready(None) => {
                    self.reject_probe(probe);

                    Ok(Answer::Ready(None))
                }
                Answer::Pending(blockers) => {
                    self.reject_probe(probe);

                    // blockers that died with the probe cannot wake this candidate
                    let blockers = self.live_blockers(blockers);
                    Ok(Answer::ready_unless_blocked(None, blockers))
                }
            };
        }

        // applied classes substitute their written arguments
        let substitution = self.instance_substitution(instance)?.with_receiver(target);

        // reject arities the constructor cannot accept
        let required = function
            .parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = function
            .parameters
            .iter()
            .any(|parameter| parameter.is_rest);
        if arguments.len() < required || (!has_rest && arguments.len() > function.parameters.len())
        {
            return Ok(Answer::Ready(None));
        }

        // constrain each argument into its substituted parameter
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = function
                .parameters
                .get(index)
                .or_else(|| function.parameters.last());
            let Some(parameter) = parameter else {
                return Ok(Answer::Ready(None));
            };
            let parameter_type = if substitution.is_empty() {
                parameter.ty
            } else {
                self.fold_type(module, source, parameter.ty, substitution.rewrite())?
            };

            if !answer!(self.constrain(origin, Relation::Assignable, *argument, parameter_type)?) {
                return Ok(Answer::Ready(None));
            }
        }

        // the construction produces the substituted receiver
        let produced = match return_type {
            Some(return_type) if !substitution.is_empty() => {
                self.fold_type(module, source, return_type, substitution.rewrite())?
            }
            Some(return_type) => return_type,
            None => target,
        };

        let parameters = function
            .parameters
            .iter()
            .map(|parameter| {
                let ty = if substitution.is_empty() {
                    parameter.ty
                } else {
                    self.fold_type(module, source, parameter.ty, substitution.rewrite())?
                };
                let ty = self.fold_type(module, source, ty, TypeRewrite::Resolve)?;

                Ok(dir::FunctionParameterType {
                    ty,
                    static_parameter: None,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        Ok(Answer::Ready(Some((parameters, produced))))
    }

    /// Select one newtype construction through call syntax.
    ///
    /// Example:
    /// ```ds
    /// UserId(1)
    /// Point(1, 2)
    /// ```
    pub(in crate::check) fn select_newtype_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // read the wrapped backing type
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol) else {
            return self.reject_construct(node, origin, arguments);
        };
        let backing = definition.value;

        // instantiate the newtype's generic parameters
        let template = self.symbol_template(symbol);
        let probe = self.begin_probe();
        let matched =
            self.match_newtype_construct(origin, module, source, template, backing, arguments);

        match matched? {
            Answer::Ready(Some((applied, parameters))) => {
                self.commit_probe(probe);

                // the construction produces the applied newtype
                let produced = self.push_type(
                    module,
                    dir::Type::Instance(dir::GenericInstance {
                        symbol,
                        arguments: applied.clone(),
                    }),
                    source,
                )?;
                let arguments = self.symbol_generic_argument_bindings(symbol, &applied)?;
                let target = dir::ConstructTarget::Newtype(dir::NewtypeConstructCandidate {
                    symbol,
                    generic_arguments: arguments,
                });
                let parameter_slots = Self::plain_parameter_slots(&parameters);
                let resolution = dir::ConstructResolution::new(
                    target,
                    parameters,
                    self.argument_bindings(module, argument_nodes, &parameter_slots),
                    produced,
                );
                answer!(self.push_argument_constraints(
                    module,
                    argument_nodes,
                    &resolution.arguments
                )?);
                self.record_decision(node, Decision::Construct(resolution))?;
                self.bind_node_type(node, produced)?;

                Ok(Answer::Ready(()))
            }
            Answer::Ready(None) => {
                self.reject_probe(probe);

                self.reject_construct(node, origin, arguments)
            }
            Answer::Pending(blockers) => {
                self.reject_probe(probe);

                // blockers that died with the probe cannot wake this construction
                let blockers = self.live_blockers(blockers);
                if blockers.is_empty() {
                    self.reject_construct(node, origin, arguments)
                } else {
                    Ok(Answer::Pending(blockers))
                }
            }
        }
    }

    /// Match newtype construction arguments under an active probe.
    /// Returns the resolved generic arguments on a match.
    fn match_newtype_construct(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        template: Option<GenericTemplateId>,
        backing: dir::GlobalTypeId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<(Vec<dir::GlobalTypeId>, Vec<dir::GlobalTypeId>)>>> {
        // instantiate the declared parameters
        let substitution = match template {
            Some(template) => {
                match self.instantiate_template(
                    origin,
                    template,
                    &[],
                    GenericArgumentMode::Infer,
                )? {
                    Some(substitution) => substitution,
                    None => return Ok(Answer::Ready(None)),
                }
            }
            None => Default::default(),
        };
        let backing = if substitution.is_empty() {
            backing
        } else {
            self.fold_type(module, source, backing, substitution.rewrite())?
        };

        // tuple backings take their elements positionally
        let backing = answer!(self.reduce_type_root(origin, backing)?);
        let elements = match self.ty(backing)? {
            dir::Type::Tuple(tuple) => Some(
                tuple
                    .elements
                    .iter()
                    .map(|element| element.ty)
                    .collect::<SmallVec<[_; 4]>>(),
            ),
            _ => None,
        };
        let parameters = match elements {
            Some(elements) => {
                // every tuple element takes one positional argument
                if arguments.len() != elements.len() {
                    return Ok(Answer::Ready(None));
                }
                for (argument, element) in arguments.iter().zip(elements.iter()) {
                    if !answer!(self.constrain(
                        origin,
                        Relation::Assignable,
                        *argument,
                        *element
                    )?) {
                        return Ok(Answer::Ready(None));
                    }
                }

                elements.to_vec()
            }
            None => {
                // every other backing takes exactly one argument
                let [argument] = arguments else {
                    return Ok(Answer::Ready(None));
                };
                if !answer!(self.constrain(origin, Relation::Assignable, *argument, backing)?) {
                    return Ok(Answer::Ready(None));
                }

                vec![backing]
            }
        };

        // solve and resolve the probe variables
        let variables = self.substitution_variables(&substitution)?;
        if !answer!(self.solve_probe_variables(variables)?) {
            return Ok(Answer::Ready(None));
        }
        let mut applied = Vec::with_capacity(substitution.arguments.len());
        for argument in substitution.arguments.iter().copied() {
            applied.push(self.fold_type(module, source, argument, TypeRewrite::Resolve)?);
        }

        let raw_parameters = parameters;
        let mut parameters = Vec::with_capacity(raw_parameters.len());
        for parameter in raw_parameters {
            parameters.push(self.fold_type(module, source, parameter, TypeRewrite::Resolve)?);
        }

        Ok(Answer::Ready(Some((applied, parameters))))
    }

    /// Record one selected construction and bound the node variable.
    fn record_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        module: destack_source::ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        instance: &dir::GenericInstance,
        constructor: dir::ClassConstructor,
        parameters: Vec<dir::FunctionParameterType>,
        return_type: dir::GlobalTypeId,
        result: ConstructResult,
    ) -> CompilerResult<Answer<()>> {
        let arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;
        let target = dir::ConstructTarget::Class(dir::ClassConstructCandidate {
            symbol: instance.symbol,
            constructor,
            generic_arguments: arguments,
        });
        let produced = match result {
            ConstructResult::Direct => return_type,
            ConstructResult::Fallible => self.fallible_construct_type(node, return_type)?,
        };
        let resolution = dir::ConstructResolution::new(
            target,
            Self::parameter_types(&parameters),
            self.argument_bindings(module, argument_nodes, &parameters),
            produced,
        );
        answer!(self.push_argument_constraints(module, argument_nodes, &resolution.arguments)?);
        self.record_decision(node, Decision::Construct(resolution))?;

        // flow the constructed type into the node variable
        self.bind_node_type(node, produced)?;

        Ok(Answer::Ready(()))
    }

    /// Return the result carrier for one fallible construction.
    fn fallible_construct_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let allocation_error = self.push_language_type(
            node.module_id,
            node.local_id,
            dir::LanguageItem::AllocationError,
            Vec::new(),
        )?;
        let carrier = self.push_language_type(
            node.module_id,
            node.local_id,
            dir::LanguageItem::Result,
            vec![value, allocation_error],
        )?;

        Ok(carrier)
    }

    /// Return simple positional parameter slots from selected parameter types.
    fn plain_parameter_slots(parameters: &[dir::GlobalTypeId]) -> Vec<dir::FunctionParameterType> {
        parameters
            .iter()
            .copied()
            .map(|ty| dir::FunctionParameterType {
                ty,
                static_parameter: None,
                is_optional: false,
                is_rest: false,
            })
            .collect()
    }

    /// Reject one construction whose arguments fit no constructor.
    fn reject_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        self.report_no_matching_call(origin, arguments, None)?;
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }

    /// Reject one construction whose target cannot use new.
    fn reject_not_constructible(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        target: dir::GlobalTypeId,
        hint: &str,
    ) -> CompilerResult<Answer<()>> {
        self.report_not_constructible(origin, target, hint)?;
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}
