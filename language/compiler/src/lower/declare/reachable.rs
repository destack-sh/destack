use std::slice;

use destack_core::{FxIndexSet, StringId};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::lower::{CallableImplementation, LowerModuleState, ModuleLowerer, TypeSubstitution};
use crate::{CompilerError, CompilerResult, LowerError};

/// Everything reachable from the bodies that lowering must declare.
#[derive(Default)]
pub(in crate::lower) struct Reachable {
    /// Generic instantiations to declare, with their collecting substitution, in reference order.
    pub(in crate::lower) instances: Vec<(
        dir::GlobalSymbolId,
        Vec<dir::GenericArgumentBinding>,
        TypeSubstitution,
    )>,
    /// Foreign callables to declare as imports.
    pub(in crate::lower) imports: FxIndexSet<dir::GlobalSymbolId>,
    /// Sealed bindings to declare as dotted host externs.
    pub(in crate::lower) bindings: FxIndexSet<dir::GlobalSymbolId>,
    /// Foreign module constants to declare as imported globals.
    pub(in crate::lower) constants: FxIndexSet<dir::GlobalSymbolId>,
    /// Concrete and constraint pairs whose implementing methods to declare.
    pub(in crate::lower) implementers: FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    /// String literal contents to declare as immortal String objects.
    pub(in crate::lower) strings: FxIndexSet<StringId>,
    /// Bigint literal values to declare as immortal BigInt objects.
    pub(in crate::lower) bigints: FxIndexSet<i64>,
    /// Closure declarations to declare as module callables, in discovery order.
    pub(in crate::lower) closures: Vec<(
        dir::LocalNodeId<dir::Declaration>,
        dir::LocalNodeId<dir::Expression>,
    )>,
}

/// Visitor collecting every node in one body subtree.
struct NodeCollector {
    /// The visitor options.
    options: dir::NodeVisitorOptions,
    /// Every collected node, of any kind.
    nodes: Vec<dir::LocalNodeIdAny>,
}

impl NodeCollector {
    /// Collect every node in one body subtree.
    fn collect(
        state: &LowerModuleState,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Vec<dir::LocalNodeIdAny> {
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

        collector.nodes
    }
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
    /// Collect everything one body reaches under one substitution.
    pub(super) fn collect_body(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        substitution: &TypeSubstitution,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // walk the body subtree collecting every node
        let state = self.state(module)?;
        let nodes = NodeCollector::collect(state, expression);

        // collect what each node in the body reaches
        for id in nodes {
            let node = dir::GlobalNodeIdAny {
                module_id: module,
                local_id: id,
            };

            // queue closures declared inside this module's bodies
            if module == self.module
                && let Ok(expression) = id.try_into_typed::<dir::Expression>()
                && let dir::Expression::Declaration(declaration) = state.tree().get(expression)
                && let dir::Declaration::Function(function) = state.tree().get(*declaration)
                && let Some(body) = function.body
            {
                let closure = (*declaration, body);
                if !reachable.closures.contains(&closure) {
                    reachable.closures.push(closure);
                }
            }

            // collect literals committed at their runtime types, skipping comptime singletons
            if let Ok(expression) = id.try_into_typed::<dir::Expression>()
                && let dir::Expression::ScalarLiteral(literal) = state.tree().get(expression)
            {
                let is_runtime = state
                    .types
                    .get_node_type_id(node)
                    .is_some_and(|ty| !matches!(self.ty(ty), Ok(dir::Type::Literal(_))));
                match literal {
                    _ if !is_runtime => {}
                    dir::ScalarLiteral::String(string) => {
                        reachable.strings.insert(*string);
                    }
                    dir::ScalarLiteral::Bigint(bigint) => {
                        reachable.bigints.insert(*bigint);
                    }
                    _ => {}
                }
            }

            // declare constructor instances and import foreign declared constructors
            if let Some(resolution) = state.decisions.construct_decision(node)
                && let dir::ConstructTarget::Class(candidate) = &resolution.target
                && let dir::ClassConstructor::Declared { symbol } = &candidate.constructor
            {
                // instantiate generic-owner constructors at their class arguments
                if !candidate.generic_arguments.is_empty() {
                    let bindings =
                        self.instance_bindings(&candidate.generic_arguments, substitution)?;
                    self.push_instance(*symbol, bindings, substitution, reachable)?;
                }
                // import plain foreign constructors directly
                else if symbol.module_id != self.module {
                    reachable.imports.insert(*symbol);
                }
            }

            // collect the calls selected inside a tree resolution
            if let Some(resolution) = state.decisions.tree_decision(node) {
                match &resolution.target {
                    dir::TreeTarget::Element { call, .. } | dir::TreeTarget::Fragment { call } => {
                        self.collect_call_decision(call, substitution, reachable)?;
                    }
                    dir::TreeTarget::Component { invocation, .. } => match invocation {
                        dir::TreeInvocation::Call(call) => {
                            self.collect_call_decision(call, substitution, reachable)?;
                        }
                        dir::TreeInvocation::Construct(construct) => {
                            if let dir::ConstructTarget::Class(candidate) = &construct.target
                                && let dir::ClassConstructor::Declared { symbol } =
                                    &candidate.constructor
                                && symbol.module_id != self.module
                            {
                                reachable.imports.insert(*symbol);
                            }
                        }
                        dir::TreeInvocation::Struct { .. } => {}
                    },
                }
            }

            // collect the implementing methods required by erasing coercions
            if let Some(coercion) = state.coercions.coercion(node) {
                self.collect_coercion(coercion, substitution, reachable)?;
            }

            // collect instances behind value-position callable references
            match self.collect_function_reference(module, node, substitution, reachable) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(_)) => {}
                Err(error) => return Err(error),
            }

            // collect the accessor and protocol calls selected behind places
            if let Some(resolution) = state.decisions.member_decision(node) {
                self.collect_member_decision(resolution, substitution, reachable)?;
            }
            if let Some(resolution) = state.decisions.subscript_decision(node) {
                self.collect_subscript_decision(resolution, substitution, reachable)?;
            }
            if let Some(resolution) = state.decisions.assignment_decision(node) {
                match &resolution.read {
                    Some(dir::ReadResolution::Member(member)) => {
                        self.collect_member_decision(member, substitution, reachable)?;
                    }
                    Some(dir::ReadResolution::Subscript(subscript)) => {
                        self.collect_subscript_decision(subscript, substitution, reachable)?;
                    }
                    _ => {}
                }
                match &resolution.write {
                    dir::WriteResolution::Member(member) => {
                        self.collect_member_decision(member, substitution, reachable)?;
                    }
                    dir::WriteResolution::Subscript(subscript) => {
                        self.collect_subscript_decision(subscript, substitution, reachable)?;
                    }
                    _ => {}
                }
            }

            // collect the instance behind a resolved call
            let Some(resolution) = state.decisions.call_decision(node) else {
                continue;
            };
            self.collect_call_decision(resolution, substitution, reachable)?;
        }

        Ok(())
    }

    /// Collect the accessor calls selected by one member resolution.
    fn collect_member_decision(
        &self,
        resolution: &dir::MemberDecision,
        substitution: &TypeSubstitution,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        match resolution {
            dir::OperationResolution::One(access) => {
                self.collect_member_access(access, substitution, reachable)
            }
            dir::OperationResolution::Union { arms, .. } => {
                for access in arms {
                    self.collect_member_access(access, substitution, reachable)?;
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
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        if let dir::MemberTarget::Call(call) = &access.target {
            self.collect_call(call, substitution, reachable)?;
        }

        Ok(())
    }

    /// Collect the protocol calls selected by one subscript resolution.
    fn collect_subscript_decision(
        &self,
        resolution: &dir::SubscriptDecision,
        substitution: &TypeSubstitution,
        reachable: &mut Reachable,
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
                    self.collect_member_access(access, substitution, reachable)?;
                }
                dir::SubscriptTarget::Call(call) => {
                    self.collect_call(call, substitution, reachable)?;
                }
                dir::SubscriptTarget::Index(read) => {
                    self.collect_call(&read.call, substitution, reachable)?;
                    if let dir::DereferenceTarget::Call(call) = &read.dereference.target {
                        self.collect_call(call, substitution, reachable)?;
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
        reachable: &mut Reachable,
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
        if symbol.module_id != self.module && self.is_module_binding(symbol)? {
            reachable.constants.insert(symbol);

            return Ok(());
        }

        // value bindings call through their bound values, never as declarations
        let kind = self
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id)
            .kind;
        if kind.is_binding() {
            return Ok(());
        }
        let Some(ty) = self.types(symbol.module_id)?.get_symbol_type_id(symbol) else {
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
                reachable.bindings.insert(symbol);

                return Ok(());
            }
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(()),
            None => {}
        }

        // select the instance the recorded instantiation binds
        if let Some(instantiation) = state.decisions.instantiation_decision(node) {
            let bindings =
                self.instance_bindings(&instantiation.generic_arguments, substitution)?;
            if !bindings.is_empty() {
                return self.push_instance(symbol, bindings, substitution, reachable);
            }
        }

        // select the instance the reference's instantiating conversion binds
        if let Some(coercion) = state.coercions.coercion(node) {
            for adjustment in &coercion.adjustments {
                if let dir::CoercionAdjustment::Instantiate { arguments, .. } = adjustment {
                    let bindings = self.instance_bindings(arguments, substitution)?;

                    return self.push_instance(symbol, bindings, substitution, reachable);
                }
            }
        }

        // leave unconverted generic references to their call resolutions
        if !self.signature_template_parameters(ty)?.is_empty() {
            return Ok(());
        }

        // import plain references into other modules
        if symbol.module_id != self.module {
            reachable.imports.insert(symbol);
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

    /// Queue one concrete instance for declaration.
    fn push_instance(
        &self,
        symbol: dir::GlobalSymbolId,
        bindings: Vec<dir::GenericArgumentBinding>,
        substitution: &TypeSubstitution,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // require every generic argument to close under the collecting substitution
        for binding in &bindings {
            let mut queue = vec![binding.argument];
            let mut visited = FxIndexSet::default();
            while let Some(id) = queue.pop() {
                if !visited.insert(id) {
                    continue;
                }
                let ty = self.ty(id)?;
                if let dir::Type::Parameter(parameter) = ty {
                    let Some(bound) = substitution.binding(parameter) else {
                        return Err(LowerError::Unsupported {
                            anchor: self.module.into(),
                            construct: "a generically scoped callable instance".to_string(),
                        }
                        .into());
                    };
                    queue.push(bound);

                    continue;
                }
                self.types(id.module_id)?
                    .for_each_child(&ty, |child| queue.push(child));
            }
        }

        // queue the instance, letting the declared representation key collapse duplicates
        reachable
            .instances
            .push((symbol, bindings, substitution.clone()));

        Ok(())
    }

    /// Collect the implementing methods one erasing coercion requires.
    fn collect_coercion(
        &self,
        coercion: &dir::Coercion,
        substitution: &TypeSubstitution,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // materialize widened comptime literals as immortal objects
        if let Ok(source) = substitution.resolve(self, coercion.source) {
            match self.ty(source)? {
                dir::Type::Literal(dir::ScalarLiteral::String(string)) => {
                    reachable.strings.insert(string);
                }
                dir::Type::Literal(dir::ScalarLiteral::Bigint(bigint)) => {
                    reachable.bigints.insert(bigint);
                }
                _ => {}
            }
        }

        // walk the adjustment chain, collecting each erasing step
        let mut source = coercion.source;
        for adjustment in &coercion.adjustments {
            if let dir::CoercionAdjustment::Existential { target } = adjustment {
                self.collect_existential(source, *target, substitution, reachable)?;
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
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // record the resolved pair for its declared dispatch entries
        let source = substitution.resolve(self, source)?;
        let resolved_target = substitution.resolve(self, target)?;
        reachable.implementers.insert((source, resolved_target));

        // stop at structural sources, which supply properties
        let dir::Type::Application(class) = self.ty(source)? else {
            return Ok(());
        };

        // read the constraint interface from the erased target
        let target = match self.ty(target)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
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
            reachable
                .instances
                .push((implementing, Vec::new(), TypeSubstitution::default()));
        }

        Ok(())
    }

    /// Collect every declaration selected by one call resolution.
    fn collect_call_decision(
        &self,
        resolution: &dir::CallDecision,
        substitution: &TypeSubstitution,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        match resolution {
            dir::OperationResolution::One(call) => self.collect_call(call, substitution, reachable),
            dir::OperationResolution::Union { arms, .. } => {
                for call in arms {
                    self.collect_call(call, substitution, reachable)?;
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
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        let function = match &call.target {
            dir::CallTarget::Expression { .. } | dir::CallTarget::Dynamic { .. } => return Ok(()),
            dir::CallTarget::Symbol { function, .. } => function,
        };

        // route intrinsic and binding callables without an instance
        match self.callable_implementation(function.symbol)? {
            // declare dotted host externs for bindings
            Some(CallableImplementation::Binding { .. }) => {
                reachable.bindings.insert(function.symbol);

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
                reachable.imports.insert(function.symbol);
            }

            return Ok(());
        }

        self.push_instance(function.symbol, bindings, substitution, reachable)
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
}
