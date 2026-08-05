use std::slice;
use std::sync::Arc;

use destack_artifact::DiagnosticLike;
use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::r#type::LoweredSignature;
use crate::lower::{
    CallableImplementation, FunctionDefinition, LifetimeParameters, LowerModuleState,
    ModuleLowerer, ReceiverBinding, TypeSubstitution,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// The declaration outcome behind one callable instance key.
pub(in crate::lower) enum FunctionDeclaration {
    /// The declared MIR function.
    Declared(mir::FunctionId),
    /// The declaration failed with a reported diagnostic.
    Failed,
}

/// One concrete declaration instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::lower) struct GenericInstanceKey {
    /// The instantiated declaration.
    pub(in crate::lower) symbol: dir::GlobalSymbolId,
    /// The concrete generic arguments.
    pub(in crate::lower) arguments: Vec<mir::StaticId>,
}

impl GenericInstanceKey {
    /// Create the instance key of one non-generic declaration.
    pub(in crate::lower) fn non_generic(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            symbol,
            arguments: Vec::new(),
        }
    }
}

/// Everything the bodies reference beyond the module's own declarations.
#[derive(Default)]
pub(in crate::lower) struct References {
    /// Generic instantiations to declare as concrete instances, in demand order.
    pub(in crate::lower) instances: Vec<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>,
    /// Foreign callables to declare as imports.
    pub(in crate::lower) imports: FxIndexSet<dir::GlobalSymbolId>,
    /// Sealed bindings to declare as dotted host externs.
    pub(in crate::lower) bindings: FxIndexSet<dir::GlobalSymbolId>,
    /// Foreign module constants to declare as imported globals.
    pub(in crate::lower) constants: FxIndexSet<dir::GlobalSymbolId>,
    /// Erased concrete and constraint pairs to declare as dispatch implementers.
    pub(in crate::lower) erasures: FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
}

/// Visitor collecting every node in one body subtree.
struct NodeCollector {
    /// The visitor options.
    options: dir::NodeVisitorOptions,
    /// Every collected node, of any kind.
    nodes: Vec<dir::LocalNodeIdAny>,
}

impl dir::NodeVisitor for NodeCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_any(&mut self, _tree: &dir::Tree, ty: dir::NodeType, id: u32) {
        self.nodes.push(dir::LocalNodeIdAny::new(id, ty));
    }
}

impl ModuleLowerer<'_> {
    /// Declare every concrete generic instance reachable from the bodies.
    ///
    /// A body whose collection or instance declaration fails keeps its diagnostic and the
    /// remaining bodies keep their collected references.
    pub(in crate::lower) fn declare_reachable_instances(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bodies: &[FunctionDefinition],
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<(Vec<FunctionDefinition>, References)> {
        // collect calls from the concrete bodies queued for lowering
        let mut references = References::default();
        for body in bodies {
            match self.collect_body_calls(
                body.source,
                body.expression,
                &body.type_substitution,
                &mut references,
            ) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
            self.declare_body_types(builder, body)?;
        }

        // collect calls from the module initializer expressions
        let substitution = TypeSubstitution::default();
        let lifetime_parameters = LifetimeParameters::default();
        let expressions: Vec<_> = self
            .initializers
            .iter()
            .map(|(_, expression)| *expression)
            .collect();
        for expression in expressions {
            match self.collect_body_calls(self.module, expression, &substitution, &mut references) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
            self.declare_expression_types(
                builder,
                self.module,
                expression,
                &substitution,
                &lifetime_parameters,
            )?;
        }

        // collect calls recursively from each declared instance body
        let mut instances = Vec::new();
        let mut index = 0;
        while index < references.instances.len() {
            let (symbol, bindings) = references.instances[index].clone();
            index += 1;
            let body = match self.declare_instance(builder, symbol, &bindings) {
                Ok(Some(body)) => body,
                Ok(None) => continue,
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bank_failed_callable(Some(symbol), diagnostic, errors);

                    continue;
                }
                Err(error) => return Err(error),
            };
            match self.collect_body_calls(
                symbol.module_id,
                body.expression,
                &body.type_substitution,
                &mut references,
            ) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
            self.declare_body_types(builder, &body)?;
            instances.push(body);
        }

        Ok((instances, references))
    }

    /// Declare the type representations one queued body reads.
    fn declare_body_types(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        body: &FunctionDefinition,
    ) -> CompilerResult<()> {
        // record the body's own parameter types beside its expression types
        let state = self.state(body.source)?;
        let mut types = FxIndexSet::default();
        for parameter in &body.parameters {
            let symbol = parameter.into_global(body.source);
            types.extend(state.types.get_symbol_type_id(symbol));
        }
        self.declare_recorded_types(
            builder,
            body.source,
            types,
            FxIndexSet::default(),
            &body.type_substitution,
            &body.lifetime_parameters,
        )?;

        self.declare_expression_types(
            builder,
            body.source,
            body.expression,
            &body.type_substitution,
            &body.lifetime_parameters,
        )
    }

    /// Declare the type representations one body subtree reads.
    ///
    /// Unrepresentable types keep quiet here: the body derives and reports them itself.
    fn declare_expression_types(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<()> {
        // record the checked, expected, and resolution types behind every node
        let state = self.state(module)?;
        let mut collector = NodeCollector {
            options: dir::NodeVisitorOptions::default(),
            nodes: Vec::new(),
        };
        dir::NodeVisitor::visit_expression(
            &mut collector,
            state.tree(),
            expression,
            state.tree().get(expression),
        );

        // index the module's symbols by their declaring nodes once
        let mut declared = FxIndexMap::default();
        for id in state.bindings.symbol_ids() {
            let symbol = state.bindings.get_symbol(id);
            if let Some(node) = symbol.declaration {
                declared.insert(node, id.into_global(module));
            }
        }

        // record the checked, expected, declared, and resolution types per node
        let mut types = FxIndexSet::default();
        let mut constraints = FxIndexSet::default();
        for id in collector.nodes {
            let node = dir::GlobalNodeIdAny {
                module_id: module,
                local_id: id,
            };
            types.extend(state.types.get_node_type_id(node));
            types.extend(state.types.get_expected_type_id(node));
            if let Some(symbol) = declared.get(&node) {
                types.extend(state.types.get_symbol_type_id(*symbol));
            }
            self.record_resolution_types(state, node, &mut types, &mut constraints);
        }

        self.declare_recorded_types(
            builder,
            module,
            types,
            constraints,
            substitution,
            lifetime_parameters,
        )
    }

    /// Declare the representations behind one recorded type set.
    fn declare_recorded_types(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        module: ModuleId,
        types: FxIndexSet<dir::GlobalTypeId>,
        constraints: FxIndexSet<dir::GlobalTypeId>,
        substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<()> {
        // lower each type into the shared representation table, keeping alias
        //  identity by lowering the unreduced spelling
        let pointer_bytes = builder.pointer_bytes();
        for ty in types {
            // materialize best effort: bodies rederive and report their own failures
            let Ok(ty) = substitution.resolve(self, ty) else {
                continue;
            };
            // reduce through the reading module: reductions key per consumer
            let Ok(reduced) = self
                .types(module)
                .map(|types| types.get_reduced_type_id(ty))
            else {
                continue;
            };
            if self.lowered_types.contains_key(&ty) {
                continue;
            }

            let mut lowerer = self.type_lowerer(
                builder.tree_mut(),
                pointer_bytes,
                substitution,
                lifetime_parameters,
            );
            match lowerer.lower(ty) {
                Ok(node) => {
                    self.lowered_types.insert(ty, Ok(node));
                }
                // keep the diagnostic for the first body that reads the type
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.lowered_types.insert(ty, Err(Arc::from(diagnostic)));
                }
                Err(_) => {}
            }

            // lower the reduced spelling separately for reduced body reads
            if reduced != ty && !self.lowered_types.contains_key(&reduced) {
                let mut lowerer = self.type_lowerer(
                    builder.tree_mut(),
                    pointer_bytes,
                    substitution,
                    lifetime_parameters,
                );
                match lowerer.lower(reduced) {
                    Ok(node) => {
                        self.lowered_types.insert(reduced, Ok(node));
                    }
                    Err(CompilerError::Diagnostic(diagnostic)) => {
                        self.lowered_types
                            .insert(reduced, Err(Arc::from(diagnostic)));
                    }
                    Err(_) => {}
                }
            }
            self.declare_stored_nominal(builder, ty, substitution, lifetime_parameters)?;
        }

        // lower each dispatch constraint into its registered shape row
        for constraint in constraints {
            let Ok(constraint) = substitution.resolve(self, constraint) else {
                continue;
            };
            let Ok(reduced) = self
                .types(module)
                .map(|types| types.get_reduced_type_id(constraint))
            else {
                continue;
            };
            if self.lowered_constraints.contains_key(&reduced) {
                continue;
            }

            let mut lowerer = self.type_lowerer(
                builder.tree_mut(),
                pointer_bytes,
                substitution,
                lifetime_parameters,
            );
            match lowerer.lower_dynamic_constraint(reduced) {
                Ok(row) => {
                    self.lowered_constraints.insert(reduced, Ok(row));
                }
                // keep the diagnostic for the first body that reads the constraint
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.lowered_constraints
                        .insert(reduced, Err(Arc::from(diagnostic)));
                }
                Err(_) => {}
            }
        }

        Ok(())
    }

    /// Declare the nominal instance behind one stored application type.
    ///
    /// Unrepresentable nominals keep quiet here: the body derives and reports them itself.
    fn declare_stored_nominal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        ty: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<()> {
        // peel value indirection down to one stored application, skipping
        //  shapes outside the value form algebra
        let stored = match self.peel_indirection(ty, substitution) {
            Ok(Some(reference)) => reference.stored,
            Ok(None) => match self.peel_owned(ty, substitution) {
                Ok(stored) => stored,
                Err(CompilerError::Diagnostic(_)) => return Ok(()),
                Err(error) => return Err(error),
            },
            Err(CompilerError::Diagnostic(_)) => return Ok(()),
            Err(error) => return Err(error),
        };
        let dir::Type::Application(instance) = self.ty(stored)? else {
            return Ok(());
        };
        if self.lowered_nominals.contains_key(&stored) {
            return Ok(());
        }

        // lower the instance once for every body that reads it
        let arguments = self
            .types(stored.module_id)?
            .type_ids(instance.arguments)
            .to_vec();
        let pointer_bytes = builder.pointer_bytes();
        let mut lowerer = self.type_lowerer(
            builder.tree_mut(),
            pointer_bytes,
            substitution,
            lifetime_parameters,
        );
        match lowerer.lower_nominal(instance.symbol, &arguments) {
            Ok(nominal) => {
                self.lowered_nominals.insert(stored, Ok(nominal));
            }
            // keep the diagnostic for the first body that reads the nominal
            Err(CompilerError::Diagnostic(diagnostic)) => {
                self.lowered_nominals
                    .insert(stored, Err(Arc::from(diagnostic)));
            }
            Err(_) => {}
        }

        Ok(())
    }

    /// Record the artifact types one node's resolutions carry.
    fn record_resolution_types(
        &self,
        state: &LowerModuleState,
        node: dir::GlobalNodeIdAny,
        types: &mut FxIndexSet<dir::GlobalTypeId>,
        constraints: &mut FxIndexSet<dir::GlobalTypeId>,
    ) {
        // record every type id each resolution carries
        let mut record = |ty: dir::GlobalTypeId| {
            types.insert(ty);
            ty
        };
        if let Some(resolution) = state.resolutions.call_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.member_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.subscript_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.assignment_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.construct_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.operator_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.tree_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(coercion) = state.coercions.coercion(node) {
            coercion.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.place_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.receiver_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.guard_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.pattern_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }
        if let Some(resolution) = state.resolutions.instantiation_resolution(node) {
            resolution.clone().map_type_ids(&mut record);
        }

        // record the dispatch constraints behind dynamic call targets
        if let Some(resolution) = state.resolutions.call_resolution(node) {
            let calls: &[dir::Call] = match resolution {
                dir::OperationResolution::One(call) => slice::from_ref(call),
                dir::OperationResolution::Union { arms, .. } => arms,
            };
            for call in calls {
                if let dir::CallTarget::Dynamic { dispatch, .. } = &call.target {
                    constraints.insert(dispatch.constraint);
                }
            }
        }
    }

    /// Declare one concrete instance unless its representation is already declared.
    fn declare_instance(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        bindings: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Option<FunctionDefinition>> {
        // key the instance by its runtime representation
        let pointer_bytes = builder.pointer_bytes();
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let type_substitution = TypeSubstitution::default();
        let lifetime_parameters = LifetimeParameters::default();
        let key = self
            .type_lowerer(
                builder.tree_mut(),
                pointer_bytes,
                &type_substitution,
                &lifetime_parameters,
            )
            .generic_instance_key(symbol, &arguments)?;
        if self.functions.contains_key(&key) {
            return Ok(None);
        }

        // declare under the instance's concrete types and polymorphic lifetimes
        let lifetime_parameters = self.lifetime_parameters(self.symbol_type(symbol)?)?;
        let type_substitution = TypeSubstitution::from_bindings(bindings);
        let declared =
            self.declare_instance_header(builder, &key, &type_substitution, &lifetime_parameters);

        declared.map(Some)
    }

    /// Collect one body's calls under one substitution.
    fn collect_body_calls(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        // walk the body subtree collecting every node
        let state = self.state(module)?;
        let mut collector = NodeCollector {
            options: dir::NodeVisitorOptions::default(),
            nodes: Vec::new(),
        };
        dir::NodeVisitor::visit_expression(
            &mut collector,
            state.tree(),
            expression,
            state.tree().get(expression),
        );

        for id in collector.nodes {
            let node = dir::GlobalNodeIdAny {
                module_id: module,
                local_id: id,
            };

            // import foreign declared constructors
            if let Some(resolution) = state.resolutions.construct_resolution(node)
                && let dir::ConstructTarget::Class(candidate) = &resolution.target
                && let dir::ClassConstructor::Declared { symbol } = &candidate.constructor
                && symbol.module_id != self.module
            {
                references.imports.insert(*symbol);
            }

            // collect the calls selected inside a tree resolution
            if let Some(resolution) = state.resolutions.tree_resolution(node) {
                match &resolution.target {
                    dir::TreeTarget::Element { call, .. } | dir::TreeTarget::Fragment { call } => {
                        self.collect_call_resolution(call, substitution, references)?;
                    }
                    dir::TreeTarget::Component { invocation, .. } => match invocation {
                        dir::TreeInvocation::Call(call) => {
                            self.collect_call_resolution(call, substitution, references)?;
                        }
                        dir::TreeInvocation::Construct(construct) => {
                            if let dir::ConstructTarget::Class(candidate) = &construct.target
                                && let dir::ClassConstructor::Declared { symbol } =
                                    &candidate.constructor
                                && symbol.module_id != self.module
                            {
                                references.imports.insert(*symbol);
                            }
                        }
                        dir::TreeInvocation::Struct { .. } => {}
                    },
                }
            }

            // collect the implementing methods required by erasing coercions
            if let Some(coercion) = state.coercions.coercion(node) {
                self.collect_coercion(coercion, substitution, references)?;
            }

            // demand instances behind value-position callable references, leaving
            //  unmatchable shapes to their own body diagnostics
            match self.collect_function_reference(module, node, substitution, references) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(_)) => {}
                Err(error) => return Err(error),
            }

            // collect the accessor and protocol calls selected behind places
            if let Some(resolution) = state.resolutions.member_resolution(node) {
                self.collect_member_resolution(resolution, substitution, references)?;
            }
            if let Some(resolution) = state.resolutions.subscript_resolution(node) {
                self.collect_subscript_resolution(resolution, substitution, references)?;
            }
            if let Some(resolution) = state.resolutions.assignment_resolution(node) {
                match &resolution.read {
                    Some(dir::ReadResolution::Member(member)) => {
                        self.collect_member_resolution(member, substitution, references)?;
                    }
                    Some(dir::ReadResolution::Subscript(subscript)) => {
                        self.collect_subscript_resolution(subscript, substitution, references)?;
                    }
                    _ => {}
                }
                match &resolution.write {
                    dir::WriteResolution::Member(member) => {
                        self.collect_member_resolution(member, substitution, references)?;
                    }
                    dir::WriteResolution::Subscript(subscript) => {
                        self.collect_subscript_resolution(subscript, substitution, references)?;
                    }
                    _ => {}
                }
            }

            let Some(resolution) = state.resolutions.call_resolution(node) else {
                continue;
            };
            self.collect_call_resolution(resolution, substitution, references)?;
        }

        Ok(())
    }

    /// Collect the accessor calls selected by one member resolution.
    fn collect_member_resolution(
        &self,
        resolution: &dir::MemberResolution,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        match resolution {
            dir::OperationResolution::One(access) => {
                self.collect_member_access(access, substitution, references)
            }
            dir::OperationResolution::Union { arms, .. } => {
                for access in arms {
                    self.collect_member_access(access, substitution, references)?;
                }

                Ok(())
            }
        }
    }

    /// Collect the accessor call selected by one member access.
    fn collect_member_access(
        &self,
        access: &dir::MemberAccess,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        if let dir::MemberTarget::Call(call) = &access.target {
            self.collect_call(call, substitution, references)?;
        }

        Ok(())
    }

    /// Collect the protocol calls selected by one subscript resolution.
    fn collect_subscript_resolution(
        &self,
        resolution: &dir::SubscriptResolution,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        // flatten singular and union selections into one arm list
        let arms: &[dir::Subscript] = match resolution {
            dir::OperationResolution::One(subscript) => slice::from_ref(subscript),
            dir::OperationResolution::Union { arms, .. } => arms,
        };

        // collect the call selected behind each arm's target
        for subscript in arms {
            match &subscript.target {
                dir::SubscriptTarget::Member(access) => {
                    self.collect_member_access(access, substitution, references)?;
                }
                dir::SubscriptTarget::Call(call) => {
                    self.collect_call(call, substitution, references)?;
                }
                dir::SubscriptTarget::Index(read) => {
                    self.collect_call(&read.call, substitution, references)?;
                    if let dir::DereferenceTarget::Call(call) = &read.dereference.target {
                        self.collect_call(call, substitution, references)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Collect one value-position callable reference.
    fn collect_function_reference(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        // resolve the referenced symbol and require a callable declaration
        let state = self.state(module)?;
        let Some(symbol) = state
            .resolutions
            .name_resolution(node)
            .and_then(|resolution| resolution.symbols().first().copied())
        else {
            return Ok(());
        };

        // import foreign module constants before any callable routing
        if symbol.module_id != self.module && self.module_constant(symbol)?.is_some() {
            references.constants.insert(symbol);

            return Ok(());
        }
        let Some(ty) = self
            .types(symbol.module_id)?
            .get_reduced_symbol_type_id(symbol)
        else {
            return Ok(());
        };
        if !matches!(
            self.ty(ty)?,
            dir::Type::Function(_)
                | dir::Type::FunctionSignature(_)
                | dir::Type::FunctionPointer(_)
        ) {
            return Ok(());
        }

        // route intrinsic and binding callables without an instance
        match self.callable_implementation(symbol)? {
            Some(CallableImplementation::Binding { .. }) => {
                references.bindings.insert(symbol);

                return Ok(());
            }
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(()),
            None => {}
        }

        // select an instance when the reference binds type parameters
        if let Some(instantiation) = state.resolutions.instantiation_resolution(node) {
            let bindings =
                self.instance_bindings(&instantiation.generic_arguments, substitution)?;
            if !bindings.is_empty() {
                return self.push_instance(symbol, bindings, references);
            }
        }

        // recover inferred instantiations from the converted concrete expectation
        //  (call callees carry no expectation and resolve through call collection)
        let parameters = self.signature_template_parameters(ty)?;
        if !parameters.is_empty() {
            let Some(expected) = state.types.get_expected_type_id(node) else {
                return Ok(());
            };
            let mut bindings = Vec::new();
            self.match_template_arguments(&parameters, ty, expected, &mut bindings)?;
            let bindings = self.instance_bindings(&bindings, substitution)?;

            return self.push_instance(symbol, bindings, references);
        }

        // import plain references into other modules
        if symbol.module_id != self.module {
            references.imports.insert(symbol);
        }

        Ok(())
    }

    /// Return the non-lifetime template parameters behind one callable type.
    pub(in crate::lower) fn signature_template_parameters(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<dir::GlobalGenericParameterId>> {
        // peel the callable down to its signature template
        let (signature, owner) = self.signature(ty)?;
        let Some(template) = self.types(owner)?.signature(signature).template else {
            return Ok(Vec::new());
        };

        // keep the parameters an instance key selects
        let template_module = template.module_id;
        let generics = &self.state(template_module)?.generics;
        let template = generics.get_template(template.local_id);
        let mut parameters = Vec::new();
        for parameter in &template.parameters {
            let binding = generics.get_parameter(*parameter);
            if binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime) {
                continue;
            }
            parameters.push(parameter.into_global(template_module));
        }

        Ok(parameters)
    }

    /// Bind template parameters by matching one declared type against its instantiated form.
    pub(in crate::lower) fn match_template_arguments(
        &self,
        parameters: &[dir::GlobalGenericParameterId],
        declared: dir::GlobalTypeId,
        concrete: dir::GlobalTypeId,
        bindings: &mut Vec<dir::GenericArgumentBinding>,
    ) -> CompilerResult<()> {
        // skip declared positions without open parameters
        let declared = self.reduced_type(declared)?;
        let flags = self
            .types(declared.module_id)?
            .get_type_flags(declared.local_id);
        if !flags.has_parameter() {
            return Ok(());
        }
        let concrete = self.reduced_type(concrete)?;

        match (self.ty(declared)?, self.ty(concrete)?) {
            // bind one open parameter at its first concrete position
            (dir::Type::Parameter(parameter), _) if parameters.contains(&parameter) => {
                if !bindings
                    .iter()
                    .any(|binding| binding.parameter == parameter)
                {
                    bindings.push(dir::GenericArgumentBinding {
                        parameter,
                        argument: concrete,
                    });
                }

                Ok(())
            }

            // walk callable carriers down to their signatures
            (dir::Type::Function(declared), dir::Type::Function(concrete)) => self
                .match_template_arguments(
                    parameters,
                    declared.signature,
                    concrete.signature,
                    bindings,
                ),
            (dir::Type::FunctionSignature(_), dir::Type::Function(concrete)) => {
                self.match_template_arguments(parameters, declared, concrete.signature, bindings)
            }
            (dir::Type::Function(declared), dir::Type::FunctionSignature(_)) => {
                self.match_template_arguments(parameters, declared.signature, concrete, bindings)
            }
            (dir::Type::FunctionPointer(declared), dir::Type::FunctionPointer(concrete)) => self
                .match_template_arguments(
                    parameters,
                    declared.signature,
                    concrete.signature,
                    bindings,
                ),

            // signatures match positionally over parameters and returns
            (
                dir::Type::FunctionSignature(declared_signature),
                dir::Type::FunctionSignature(concrete_signature),
            ) => {
                let declared_signature = *self
                    .types(declared.module_id)?
                    .signature(declared_signature);
                let concrete_signature = *self
                    .types(concrete.module_id)?
                    .signature(concrete_signature);
                let declared_parameters = self
                    .types(declared.module_id)?
                    .parameters(declared_signature.parameters)
                    .to_vec();
                let concrete_parameters = self
                    .types(concrete.module_id)?
                    .parameters(concrete_signature.parameters)
                    .to_vec();
                for (declared, concrete) in declared_parameters.iter().zip(&concrete_parameters) {
                    self.match_template_arguments(parameters, declared.ty, concrete.ty, bindings)?;
                }
                if let (Some(declared), Some(concrete)) = (
                    declared_signature.return_type,
                    concrete_signature.return_type,
                ) {
                    self.match_template_arguments(parameters, declared, concrete, bindings)?;
                }

                Ok(())
            }

            // applications match pairwise over their arguments
            (
                dir::Type::Application(declared_application),
                dir::Type::Application(concrete_application),
            ) if declared_application.symbol == concrete_application.symbol => {
                let declared_arguments = self
                    .types(declared.module_id)?
                    .type_ids(declared_application.arguments)
                    .to_vec();
                let concrete_arguments = self
                    .types(concrete.module_id)?
                    .type_ids(concrete_application.arguments)
                    .to_vec();
                for (declared, concrete) in declared_arguments.iter().zip(&concrete_arguments) {
                    self.match_template_arguments(parameters, *declared, *concrete, bindings)?;
                }

                Ok(())
            }

            _ => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a generic reference outside matchable signature positions".to_string(),
            }
            .into()),
        }
    }

    /// Queue one concrete instance for declaration.
    fn push_instance(
        &self,
        symbol: dir::GlobalSymbolId,
        bindings: Vec<dir::GenericArgumentBinding>,
        references: &mut References,
    ) -> CompilerResult<()> {
        // require every generic argument to be concrete under this body instance
        for binding in &bindings {
            if matches!(self.ty(binding.argument)?, dir::Type::Parameter(_)) {
                return Err(CompilerError::Internal {
                    message: "instantiation collection left a generic argument unsubstituted"
                        .to_string(),
                });
            }
        }
        let instance = (symbol, bindings);
        if !references.instances.contains(&instance) {
            references.instances.push(instance);
        }

        Ok(())
    }

    /// Collect the implementing methods one erasing coercion requires.
    fn collect_coercion(
        &self,
        coercion: &dir::Coercion,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        let mut source = coercion.source;
        for adjustment in &coercion.adjustments {
            if let dir::CoercionAdjustment::Existential { target } = adjustment {
                self.collect_existential(source, *target, substitution, references)?;
            }
            source = adjustment.target();
        }

        Ok(())
    }

    /// Collect the methods one concrete source supplies for a constraint.
    fn collect_existential(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        // record the resolved erasure for its declared dispatch entries
        let source = substitution.resolve(self, source)?;
        let source = self.reduced_type(source)?;
        let resolved_target = substitution.resolve(self, target)?;
        let resolved_target = self.reduced_type(resolved_target)?;
        references.erasures.insert((source, resolved_target));

        let dir::Type::Application(class) = self.ty(source)? else {
            return Ok(());
        };

        // read the constraint interface from the erased target
        let target = self.reduced_type(target)?;
        let target = match self.ty(target)? {
            dir::Type::Dynamic(dynamic) => self.reduced_type(dynamic.constraint)?,
            _ => target,
        };
        let dir::Type::Application(interface) = self.ty(target)? else {
            return Ok(());
        };
        let Some(definition) = self.definition(interface.symbol)? else {
            return Ok(());
        };

        // require one declared instance per dispatched method
        for member in definition.members() {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            let Some(name) = self.symbol_name(method.symbol)? else {
                continue;
            };
            let implementing = self.implementing_method(class.symbol, name)?;
            let instance = (implementing, Vec::new());
            if !references.instances.contains(&instance) {
                references.instances.push(instance);
            }
        }

        Ok(())
    }

    /// Collect every declaration selected by one call resolution.
    fn collect_call_resolution(
        &self,
        resolution: &dir::CallResolution,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        match resolution {
            dir::OperationResolution::One(call) => {
                self.collect_call(call, substitution, references)
            }
            dir::OperationResolution::Union { arms, .. } => {
                for call in arms {
                    self.collect_call(call, substitution, references)?;
                }

                Ok(())
            }
        }
    }

    /// Collect one singular call target.
    fn collect_call(
        &self,
        call: &dir::Call,
        substitution: &TypeSubstitution,
        references: &mut References,
    ) -> CompilerResult<()> {
        let function = match &call.target {
            dir::CallTarget::Expression { .. } | dir::CallTarget::Dynamic { .. } => return Ok(()),
            dir::CallTarget::Symbol { function, .. } => function,
        };

        // route intrinsic and binding callables without an instance
        match self.callable_implementation(function.symbol)? {
            // declare dotted host externs for bindings
            Some(CallableImplementation::Binding { .. }) => {
                references.bindings.insert(function.symbol);

                return Ok(());
            }
            // emit intrinsics without declarations
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(()),
            None => {}
        }

        // select an instance only when the call binds type parameters
        let bindings = self.instance_bindings(&function.generic_arguments, substitution)?;
        if bindings.is_empty() {
            // import plain calls into other modules
            if function.symbol.module_id != self.module {
                references.imports.insert(function.symbol);
            }

            return Ok(());
        }

        self.push_instance(function.symbol, bindings, references)
    }

    /// Return the substituted type bindings one candidate selects beyond lifetimes.
    pub(in crate::lower) fn instance_bindings(
        &self,
        generic_arguments: &[dir::GenericArgumentBinding],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let mut bindings = Vec::new();
        for binding in generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.state(parameter.module_id)?.generics;
            let parameter = generics.get_parameter(parameter.local_id);
            match parameter.kind {
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Lifetime) => continue,
                dir::GenericParameterKind::Type => {}
                dir::GenericParameterKind::Value | dir::GenericParameterKind::Memory(_) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "a value-parameterized callable instance".to_string(),
                    }
                    .into());
                }
            }

            // substitute arguments through the enclosing instance
            let argument = substitution.resolve(self, binding.argument)?;
            bindings.push(dir::GenericArgumentBinding {
                parameter: binding.parameter,
                argument,
            });
        }

        Ok(bindings)
    }

    /// Declare the header of one generic instance and queue its body.
    fn declare_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        type_substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<FunctionDefinition> {
        let symbol = key.symbol;

        // find the declaration's source function in its defining module's tree
        let Some(declaration) = self
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id)
            .declaration
        else {
            return Err(CompilerError::Internal {
                message: "an instantiated callable without a declaration".to_string(),
            });
        };

        // declare member callables through their owner's receiver
        if let Ok(member) = declaration.local_id.try_into_typed::<dir::Member>() {
            return self.declare_member_instance_header(
                builder,
                key,
                type_substitution,
                lifetime_parameters,
                member,
            );
        }
        let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Declaration>() else {
            return Err(CompilerError::Internal {
                message: "an instantiated non-declaration callable".to_string(),
            });
        };

        // resolve the source body and parameter symbols
        let dir::Declaration::Function(function) =
            self.state(symbol.module_id)?.tree().get(declaration)
        else {
            return Err(CompilerError::Internal {
                message: "an instantiated non-function declaration".to_string(),
            });
        };
        let Some(expression) = function.body else {
            return Err(CompilerError::Internal {
                message: "an instantiated bodiless function".to_string(),
            });
        };
        let parameter_nodes = function.signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }
        let declared = self.symbol_type(symbol)?;
        let signature =
            self.lower_signature(builder, declared, type_substitution, lifetime_parameters)?;
        if signature.parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        self.declare_instance_function(
            builder,
            key,
            signature,
            symbols,
            type_substitution.clone(),
            lifetime_parameters,
            expression,
        )
    }

    /// Declare the header of one static member instance and queue its body.
    fn declare_member_instance_header(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        type_substitution: &TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
        member: dir::LocalNodeId<dir::Member>,
    ) -> CompilerResult<FunctionDefinition> {
        let symbol = key.symbol;
        let member_node = member.into_global_any(symbol.module_id);
        let state = self.state(symbol.module_id)?;

        // find the owner declaring this member
        let owner = state
            .definitions
            .iter_definitions()
            .find(|(_, definition)| definition.method_declared_at(member_node) == Some(symbol))
            .map(|(owner, _)| owner)
            .ok_or_else(|| CompilerError::Internal {
                message: "an instantiated member without an owner".to_string(),
            })?;

        // require the member to be static
        let is_static = match self.definition(owner)? {
            Some(definition) => definition.members().iter().any(|candidate| {
                matches!(
                    candidate,
                    dir::DefinitionMember::Method(method)
                        if method.symbol == symbol && method.space == dir::MemberSpace::Static
                )
            }),
            None => false,
        };
        if !is_static {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a generic instance method instance".to_string(),
            }
            .into());
        }

        // receive extension members at their target
        let receiver = match self.definition(owner)? {
            Some(dir::Definition::Extension(extension)) => {
                ReceiverBinding::Type(extension.target.r#type())
            }
            _ => ReceiverBinding::Application(dir::GenericApplication {
                symbol: owner,
                arguments: dir::TypeListId::EMPTY,
            }),
        };
        let type_substitution = type_substitution.clone().with_receiver(receiver);

        // resolve the member body and parameter symbols
        let state = self.state(symbol.module_id)?;
        let dir::Member::Method {
            signature, body, ..
        } = state.tree().get(member)
        else {
            return Err(CompilerError::Internal {
                message: "an instantiated non-method member".to_string(),
            });
        };
        let Some(expression) = *body else {
            return Err(CompilerError::Internal {
                message: "an instantiated bodiless member".to_string(),
            });
        };
        let parameter_nodes = signature.parameters.to_vec();
        let mut symbols = Vec::with_capacity(parameter_nodes.len());
        for parameter in parameter_nodes {
            let node = parameter.into_global_any(symbol.module_id);
            let Some(parameter_symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one parameter".to_string(),
                });
            };
            symbols.push(parameter_symbol.local_id);
        }
        let declared = self.symbol_type(symbol)?;
        let signature =
            self.lower_signature(builder, declared, &type_substitution, lifetime_parameters)?;
        if signature.parameters.len() != symbols.len() {
            return Err(CompilerError::Internal {
                message: "instance parameters disagree with the declared signature".to_string(),
            });
        }

        self.declare_instance_function(
            builder,
            key,
            signature,
            symbols,
            type_substitution,
            lifetime_parameters,
            expression,
        )
    }

    /// Declare one instance header under its canonical name and queue its body.
    fn declare_instance_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        key: &GenericInstanceKey,
        signature: LoweredSignature,
        symbols: Vec<dir::LocalSymbolId>,
        type_substitution: TypeSubstitution,
        lifetime_parameters: &LifetimeParameters,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<FunctionDefinition> {
        let symbol = key.symbol;
        let name = self.symbol_path(symbol)?;
        let base = mir::Symbol::named(builder.intern(&name));
        let instance = base.instantiate(&key.arguments, builder.tree());
        let header = builder
            .function_header(&name)
            .arguments(key.arguments.iter().cloned())
            .symbol(instance);
        let header = lifetime_parameters.declare(header);
        let header = header
            .parameters(signature.parameters)
            .result(signature.result);
        let function = builder.declare_function(header);
        self.functions
            .insert(key.clone(), FunctionDeclaration::Declared(function));

        Ok(FunctionDefinition {
            function,
            has_this: false,
            parameters: symbols,
            type_substitution,
            lifetime_parameters: lifetime_parameters.clone(),
            source: symbol.module_id,
            expression,
        })
    }
}
