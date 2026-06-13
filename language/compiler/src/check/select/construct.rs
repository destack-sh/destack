use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Decision, GenericTemplateId, Origin, Relation, Task};
use crate::{CheckError, CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the construction meaning of one new expression.
    pub(in crate::check) fn select_construct(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        // collect argument types from walked inputs
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in argument_nodes {
            let argument = argument.into_global_any(module);
            let Some(ty) = self.inputs.node_type(argument) else {
                return Err(CompilerError::Internal {
                    message: format!("construct argument {argument:?} has no input type"),
                });
            };
            arguments.push(ty);
        }

        // close the constructed annotation
        let annotation = ty.into_global_any(module);
        let Some(target) = self.inputs.node_type(annotation) else {
            return Err(CompilerError::Internal {
                message: format!("construct annotation {annotation:?} has no input type"),
            });
        };
        let target = match self.evaluate_root(origin, target)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let instance = match self.ty(target)? {
            dir::Type::Reference(instance) => instance.clone(),
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
                // abstract classes never construct directly
                if definition.is_abstract {
                    let ty = self.format_type(target);
                    let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                    let error = CheckError::CannotConstructAbstractType { anchor, module, ty };
                    self.module_mut(module).diagnostics.push(
                        destack_artifact::DiagnosticBuilder::new(error)
                            .help("construct a concrete subclass instead"),
                    );
                    self.record_decision(node, Decision::Rejected)?;

                    return Ok(Answer::Ready(()));
                }

                definition
                    .members
                    .iter()
                    .filter_map(|member| match member {
                        dir::DefinitionMember::Method(method)
                            if method.slot == dir::MemberSlot::Constructor =>
                        {
                            Some((method.symbol, method.ty))
                        }
                        _ => None,
                    })
                    .collect::<SmallVec<[_; 2]>>()
            }
            _ => return self.reject_not_constructible(node, origin, target, ""),
        };

        // default construction takes no arguments
        if constructors.is_empty() {
            if !arguments.is_empty() {
                return self.reject_construct(node, origin, &arguments);
            }

            return self.record_construct(node, &instance, None, Vec::new(), target);
        }

        // try constructors in declaration order
        for (constructor, function_type) in constructors {
            let attempt = self.attempt_construct(
                origin,
                module,
                &instance,
                target,
                function_type,
                &arguments,
            )?;

            match attempt {
                Answer::Ready(Some((parameters, return_type))) => {
                    return self.record_construct(
                        node,
                        &instance,
                        constructor,
                        parameters,
                        return_type,
                    );
                }
                Answer::Ready(None) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        self.reject_construct(node, origin, &arguments)
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
    ) -> CompilerResult<Answer<Option<(Vec<dir::GlobalTypeId>, dir::GlobalTypeId)>>> {
        let source = self.origin_source_node(origin)?;

        // close the constructor shape first
        let function_type = match self.evaluate_root(origin, function_type)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let (parameters, return_type) = match self.ty(function_type)? {
            dir::Type::Function(function) => (
                function
                    .parameters
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>(),
                function.return_type,
            ),
            _ => return Ok(Answer::Ready(None)),
        };

        // unapplied generic classes infer their arguments under a probe
        let template = self.generics.template_by_symbol(instance.symbol);
        if instance.arguments.is_empty() && template.is_some() {
            let probe = self.begin_probe();
            let attempt = self.attempt_signature(
                origin,
                module,
                source,
                template,
                &parameters,
                return_type,
                arguments,
            )?;

            return match attempt {
                Answer::Ready(Some((parameters, return_type))) => {
                    self.keep_probe(probe)?;

                    Ok(Answer::Ready(Some((parameters.to_vec(), return_type))))
                }
                Answer::Ready(None) => {
                    self.unwind_probe(probe)?;

                    Ok(Answer::Ready(None))
                }
                Answer::Pending(blockers) => {
                    self.unwind_probe(probe)?;

                    // blockers that died with the probe cannot wake this candidate
                    let blockers = self.surviving_blockers(blockers);
                    if blockers.is_empty() {
                        Ok(Answer::Ready(None))
                    } else {
                        Ok(Answer::Pending(blockers))
                    }
                }
            };
        }

        // applied classes substitute their written arguments
        let substitution = self.parameter_substitution(instance)?.with_receiver(target);

        // reject arities the constructor cannot accept
        let required = parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = parameters.iter().any(|parameter| parameter.is_rest);
        if arguments.len() < required || (!has_rest && arguments.len() > parameters.len()) {
            return Ok(Answer::Ready(None));
        }

        // constrain each argument into its substituted parameter
        let mut substituted = Vec::with_capacity(arguments.len());
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = parameters.get(index).or_else(|| parameters.last());
            let Some(parameter) = parameter else {
                return Ok(Answer::Ready(None));
            };
            let parameter_type = if substitution.is_empty() {
                parameter.ty
            } else {
                self.fold_type(module, source, parameter.ty, substitution.rewrite())?
            };

            match self.constrain(origin, Relation::Assignable, *argument, parameter_type)? {
                Answer::Ready(true) => substituted.push(parameter_type),
                Answer::Ready(false) => return Ok(Answer::Ready(None)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
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

        Ok(Answer::Ready(Some((substituted, produced))))
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
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // read the wrapped backing type
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol) else {
            return self.reject_construct(node, origin, arguments);
        };
        let backing = definition.value;

        // hypothesize the newtype's generic parameters
        let template = self.generics.template_by_symbol(symbol);
        let probe = self.begin_probe();
        let matched =
            self.match_newtype_construct(origin, module, source, template, backing, arguments);

        match matched? {
            Answer::Ready(Some(applied)) => {
                self.keep_probe(probe)?;

                // the construction produces the applied newtype
                let produced = self.push_type(
                    module,
                    dir::Type::Reference(dir::GenericInstance {
                        symbol,
                        arguments: applied.clone(),
                    }),
                    source,
                )?;
                let target = dir::ConstructTarget::Newtype(dir::NewtypeConstructCandidate {
                    symbol,
                    arguments: applied,
                });
                let resolution =
                    dir::ConstructResolution::new(target, arguments.to_vec(), produced);
                self.record_decision(node, Decision::Construct(resolution))?;
                if let Some(variable) = self.node_variable(node)? {
                    self.push_lower_bound(variable, produced)?;
                }

                Ok(Answer::Ready(()))
            }
            Answer::Ready(None) => {
                self.unwind_probe(probe)?;

                self.reject_construct(node, origin, arguments)
            }
            Answer::Pending(blockers) => {
                self.unwind_probe(probe)?;

                // blockers that died with the probe cannot wake this construction
                let blockers = self.surviving_blockers(blockers);
                if blockers.is_empty() {
                    self.reject_construct(node, origin, arguments)
                } else {
                    Ok(Answer::Pending(blockers))
                }
            }
        }
    }

    /// Match newtype construction arguments under an active probe.
    /// Returns the harvested generic arguments on a match.
    fn match_newtype_construct(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        template: Option<GenericTemplateId>,
        backing: dir::GlobalTypeId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<Vec<dir::GlobalTypeId>>>> {
        // hypothesize the declared parameters
        let substitution = match template {
            Some(template) => self.instantiate_template(origin, template)?,
            None => Default::default(),
        };
        let backing = if substitution.is_empty() {
            backing
        } else {
            self.fold_type(module, source, backing, substitution.rewrite())?
        };

        // tuple backings take their elements positionally
        let backing = match self.evaluate_root(origin, backing)? {
            Answer::Ready(backing) => backing,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
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
        match elements {
            Some(elements) => {
                // every tuple element takes one positional argument
                if arguments.len() != elements.len() {
                    return Ok(Answer::Ready(None));
                }
                for (argument, element) in arguments.iter().zip(elements.iter()) {
                    match self.constrain(origin, Relation::Assignable, *argument, *element)? {
                        Answer::Ready(true) => {}
                        Answer::Ready(false) => return Ok(Answer::Ready(None)),
                        Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                    }
                }
            }
            None => {
                // every other backing takes exactly one argument
                let [argument] = arguments else {
                    return Ok(Answer::Ready(None));
                };
                match self.constrain(origin, Relation::Assignable, *argument, backing)? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => return Ok(Answer::Ready(None)),
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                }
            }
        }

        // solve and harvest the hypothesized parameters
        let floor = self.queue.solve_count();
        for variable_type in substitution.arguments.iter().copied() {
            if let Some(variable) = self.root_variable(variable_type)? {
                self.queue_task(Task::Solve(variable));
            }
        }
        if !self.drain_probe_tasks(floor)? {
            return Ok(Answer::Ready(None));
        }
        self.close_hypotheses(module, source, &substitution)?;
        let mut applied = Vec::with_capacity(substitution.arguments.len());
        for argument in substitution.arguments.iter().copied() {
            applied.push(self.harvest_type(module, source, argument)?);
        }

        Ok(Answer::Ready(Some(applied)))
    }

    /// Record one selected construction and bound the node variable.
    fn record_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        instance: &dir::GenericInstance,
        constructor: Option<dir::GlobalSymbolId>,
        parameters: Vec<dir::GlobalTypeId>,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let target = dir::ConstructTarget::Class(dir::ClassConstructCandidate {
            symbol: instance.symbol,
            constructor,
            arguments: instance.arguments.clone(),
        });
        let resolution = dir::ConstructResolution::new(target, parameters, return_type);
        self.record_decision(node, Decision::Construct(resolution))?;

        // flow the constructed type into the node variable
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, return_type)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Reject one construction whose arguments fit no constructor.
    fn reject_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let arguments = self.format_types(arguments);
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingCall {
            anchor,
            module,
            arguments,
        };
        self.module_mut(module).diagnostics.push(error.into());
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
        let ty = self.format_type(target);
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NotConstructible {
            anchor,
            module,
            ty,
            hint: hint.to_string(),
        };
        self.module_mut(module).diagnostics.push(error.into());
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }
}
