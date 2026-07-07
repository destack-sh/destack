use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, ConstructResult, Decision, FlowSite, Origin, SignatureRejection,
    SignatureSelection, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the construction meaning of one new expression.
    pub(in crate::check) fn select_construct(
        &mut self,
        site: FlowSite,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        result: ConstructResult,
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // infer constructor argument value types at this construct site
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in argument_nodes {
            let ty = answer!(self.argument_value_type(site, *argument)?);
            arguments.push(ty);
        }
        let argument_sources = self.argument_value_sources(module, argument_nodes);

        // reduce the constructed annotation
        let annotation = ty.into_global_any(module);
        let target = answer!(self.node_type(annotation)?);
        let target = answer!(self.reduce_type_head(origin, target)?);
        let instance = match self.ty(target)? {
            dir::Type::Instance(instance) => instance,
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
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                }

                let mut active = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
                answer!(self.collect_class_construct_candidates(
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
                target.module_id,
                &instance,
                target,
                constructor.ty,
                &arguments,
                &argument_sources,
            )?;

            if let Ok(signature) = answer!(attempt) {
                return self.commit_construct(
                    site,
                    node,
                    module,
                    target.module_id,
                    argument_nodes,
                    &instance,
                    constructor.constructor,
                    signature,
                    result,
                );
            }
        }

        self.reject_construct(node, origin, &arguments)
    }

    /// Return construct candidates for one class instance.
    fn collect_class_construct_candidates(
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

        self.collect_forwarded_class_construct_candidates(origin, receiver, &extends, active)
    }

    /// Return constructors forwarded from one base class.
    fn collect_forwarded_class_construct_candidates(
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
        let module = origin.module();
        let arguments = self.intern_type_ids(module, &extends.arguments)?;
        let instance = dir::GenericInstance {
            symbol: extends.symbol,
            arguments,
        };
        let base_receiver = self.intern_type(module, dir::Type::Instance(instance))?;
        let base_constructors = answer!(self.collect_class_construct_candidates(
            origin,
            base_receiver,
            &instance,
            base.constructors,
            base.extends,
            active,
        )?);
        active.pop();

        let substitution = self
            .instance_substitution(module, &instance)?
            .with_receiver(receiver);
        let mut constructors = Vec::with_capacity(base_constructors.len());
        for base_constructor in base_constructors {
            let constructor = base_constructor.constructor.forwarded(extends.symbol);
            let ty = self.substitute_type(origin.module(), base_constructor.ty, &substitution)?;
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
        let dir::Type::FunctionSignature(mut function) = self.ty(ty)? else {
            return Err(CompilerError::Internal {
                message: format!("class constructor type {ty:?} is not a function signature"),
            });
        };
        function.return_type = Some(receiver);

        self.intern_type(origin.module(), dir::Type::FunctionSignature(function))
    }

    /// Attempt one constructor candidate against collected arguments.
    fn attempt_construct(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        target: dir::GlobalTypeId,
        function_type: dir::GlobalTypeId,
        arguments: &[dir::GlobalTypeId],
        argument_sources: &[dir::GlobalNodeIdAny],
    ) -> CompilerResult<Answer<Result<SignatureSelection, SignatureRejection>>> {
        let source = self.origin_source_node(origin)?;

        // reduce the constructor shape before matching arguments
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let function = match self.ty(function_type)? {
            dir::Type::FunctionSignature(function) => function,
            _ => {
                return Ok(Answer::Ready(Err(SignatureRejection::Inapplicable)));
            }
        };
        let return_type = function.return_type;

        // infer omitted class arguments while testing this constructor
        let template = self.symbol_template(instance.symbol);
        if instance.arguments.is_empty()
            && let Some(template) = template
        {
            let parameters = self.generic_template_parameters(template);
            return self.attempt_signature(
                origin,
                module,
                function_type.module_id,
                source,
                &parameters,
                None,
                &[],
                &[],
                &function,
                return_type,
                None,
                arguments,
                argument_sources,
            );
        }

        // applied classes substitute their written arguments
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(target);
        let function_type = self.substitute_type(origin.module(), function_type, &substitution)?;
        let function_type = answer!(self.reduce_type_head(origin, function_type)?);
        let dir::Type::FunctionSignature(function) = self.ty(function_type)? else {
            return Ok(Answer::Ready(Err(SignatureRejection::Inapplicable)));
        };

        let return_type = function.return_type.or(Some(target));
        let carried =
            self.generic_argument_bindings(&substitution.parameters, &substitution.arguments)?;
        self.attempt_signature(
            origin,
            module,
            function_type.module_id,
            source,
            &[],
            None,
            &carried,
            &[],
            &function,
            return_type,
            None,
            arguments,
            argument_sources,
        )
    }

    /// Select one newtype construction through call expression form.
    ///
    /// Example:
    /// ```ds
    /// UserId(1)
    /// Point(1, 2)
    /// ```
    pub(in crate::check) fn select_newtype_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let argument_sources = self.argument_value_sources(module, argument_nodes);

        // read the wrapped backing type
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol) else {
            return self.reject_construct(node, origin, arguments);
        };
        let backing = definition.value;

        // model the backing as a callable signature
        let generic_parameters = self
            .symbol_template(symbol)
            .map(|template| self.generic_template_parameters(template))
            .unwrap_or_default();
        let return_arguments = generic_parameters
            .iter()
            .copied()
            .map(|parameter| self.intern_type(module, dir::Type::Parameter(parameter)))
            .collect::<CompilerResult<Vec<_>>>()?;
        let return_arguments = self.intern_type_ids(module, &return_arguments)?;
        let return_type = self.intern_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments: return_arguments,
            }),
        )?;
        let backing = answer!(self.reduce_type_head(origin, backing)?);
        let parameters = match self.ty(backing)? {
            dir::Type::Tuple(tuple) => self
                .tuple_elements(backing.module_id, tuple.elements)?
                .iter()
                .map(|element| dir::FunctionParameterType {
                    ty: element.ty,
                    is_optional: false,
                    is_rest: false,
                })
                .collect::<Vec<_>>(),
            _ => vec![dir::FunctionParameterType {
                ty: backing,
                is_optional: false,
                is_rest: false,
            }],
        };
        let parameters = self.intern_parameters(module, &parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: None,
            parameters,
            return_type: Some(return_type),
            is_generator: false,
        };
        let attempt = self.attempt_signature(
            origin,
            module,
            module,
            source,
            &generic_parameters,
            None,
            &[],
            type_arguments,
            &function,
            function.return_type,
            None,
            arguments,
            &argument_sources,
        )?;
        let signature = match answer!(attempt) {
            Ok(signature) => signature,
            Err(_) => return self.reject_construct(node, origin, arguments),
        };

        // commit the selected newtype construction
        let target = dir::ConstructTarget::Newtype(dir::NewtypeConstructCandidate {
            symbol,
            generic_arguments: signature.generic_arguments.clone(),
        });
        let resolution = dir::ConstructResolution::new(
            target,
            Self::parameter_types(&signature.parameters),
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            signature.return_type,
        );
        answer!(self.push_argument_constraints(site, argument_nodes, &resolution.arguments)?);
        self.commit_decision(node, Decision::Construct(resolution))?;
        self.commit_node_type(node, signature.return_type)?;

        Ok(Answer::Ready(()))
    }

    /// Commit one accepted construction selection.
    fn commit_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        module: ModuleId,
        instance_module: ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        instance: &dir::GenericInstance,
        constructor: dir::ClassConstructor,
        signature: SignatureSelection,
        result: ConstructResult,
    ) -> CompilerResult<Answer<()>> {
        let generic_arguments = if signature.generic_arguments.is_empty() {
            let arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();

            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?
        } else {
            signature.generic_arguments.clone()
        };
        let target = dir::ConstructTarget::Class(dir::ClassConstructCandidate {
            symbol: instance.symbol,
            constructor,
            generic_arguments,
        });
        let produced = match result {
            ConstructResult::Direct => signature.return_type,
            ConstructResult::Fallible => {
                self.fallible_construct_type(node, signature.return_type)?
            }
        };
        let resolution = dir::ConstructResolution::new(
            target,
            Self::parameter_types(&signature.parameters),
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            produced,
        );
        answer!(self.push_argument_constraints(site, argument_nodes, &resolution.arguments)?);
        self.commit_decision(node, Decision::Construct(resolution))?;

        self.commit_node_type(node, produced)?;

        Ok(Answer::Ready(()))
    }

    /// Return the result carrier for one fallible construction.
    fn fallible_construct_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let allocation_error =
            self.language_type(node.module_id, dir::LanguageItem::AllocationError, &[])?;
        let carrier = self.language_type(
            node.module_id,
            dir::LanguageItem::Result,
            &[value, allocation_error],
        )?;

        Ok(carrier)
    }

    /// Reject one construction whose arguments fit no constructor.
    pub(in crate::check) fn reject_construct(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        self.report_no_matching_construct(origin, arguments)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

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
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }
}
