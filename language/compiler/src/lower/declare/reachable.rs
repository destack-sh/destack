use std::slice;

use destack_core::{FxIndexSet, StringId};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::lower::{CallableImplementation, LowerModuleState, ModuleLowerer, NominalField};
use crate::{CompilerError, CompilerResult};

/// Everything reachable from the bodies that lowering must declare.
#[derive(Default)]
pub(in crate::lower) struct Reachable {
    /// Generic instances to declare, in reference order.
    pub(in crate::lower) instances: Vec<dir::InstanceKey>,
    /// Foreign callables to declare as imports.
    pub(in crate::lower) imports: FxIndexSet<dir::GlobalSymbolId>,
    /// Sealed bindings to declare as dotted host externs.
    pub(in crate::lower) bindings: FxIndexSet<dir::GlobalSymbolId>,
    /// Foreign module constants to declare as imported globals.
    pub(in crate::lower) constants: FxIndexSet<dir::GlobalSymbolId>,
    /// Concrete and constraint pairs whose implementing methods to declare.
    pub(in crate::lower) implementers: FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    /// String literal contents to declare as constant String objects.
    pub(in crate::lower) strings: FxIndexSet<StringId>,
    /// Bigint literal values to declare as constant BigInt objects.
    pub(in crate::lower) bigints: FxIndexSet<i64>,
    /// Closure declarations to declare as module callables, in discovery order.
    pub(in crate::lower) closures: Vec<(
        dir::LocalNodeId<dir::Declaration>,
        dir::LocalNodeId<dir::Expression>,
    )>,
    /// Field initializer bodies already collected, closing recursive constructions.
    pub(in crate::lower) initializers: FxIndexSet<(
        dir::GlobalNodeIdAny,
        Option<(ModuleId, dir::LocalInstanceId)>,
    )>,
    /// Classes with field initializers constructed without a declared constructor.
    pub(in crate::lower) default_constructors:
        FxIndexSet<(dir::GlobalSymbolId, Vec<dir::GenericArgumentBinding>)>,
}

/// Visitor collecting every node in one body subtree.
struct NodeCollector {
    /// Every collected node, of any kind.
    nodes: Vec<dir::LocalNodeIdAny>,
}

impl NodeCollector {
    /// Collect every node in one body subtree.
    fn collect(
        state: &LowerModuleState,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Vec<dir::LocalNodeIdAny> {
        // walk the subtree, recording every node the visitor reaches
        let mut collector = NodeCollector { nodes: Vec::new() };
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
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
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

            // collect literals committed at their runtime types, skipping const singletons
            if let Ok(expression) = id.try_into_typed::<dir::Expression>()
                && let dir::Expression::Literal(literal) = state.tree().get(expression)
            {
                let ty =
                    state
                        .types
                        .get_node_type_id(node)
                        .ok_or_else(|| CompilerError::Internal {
                            message: "a scalar literal has no checked type".to_string(),
                        })?;
                let is_runtime = !matches!(self.ty(ty)?, dir::Type::Literal(_));
                match literal {
                    _ if !is_runtime => {}
                    dir::Literal::String(string) => {
                        reachable.strings.insert(*string);
                    }
                    dir::Literal::Bigint(bigint) => {
                        reachable.bigints.insert(*bigint);
                    }
                    _ => {}
                }
            }

            // collect the omitted-field initializers evaluated by struct constructions
            if let Ok(expression) = id.try_into_typed::<dir::Expression>()
                && let dir::Expression::StructExpression { properties, .. } =
                    state.tree().get(expression)
            {
                let properties = properties.clone();
                self.collect_construction_initializers(
                    module,
                    node,
                    instance,
                    &properties,
                    reachable,
                )?;
            }

            // queue a synthesized constructor for default constructions with field initializers
            if let Some(resolution) = state.decisions.construct_decision(node)
                && let dir::ConstructTarget::Class {
                    key,
                    constructor: dir::ClassConstructor::Default,
                } = &resolution.target
                && self.class_has_field_initializers(key.symbol)?
            {
                let bindings = self.instance_bindings(&key.arguments, instance)?;
                if reachable
                    .default_constructors
                    .insert((key.symbol, bindings.clone()))
                {
                    let arguments: Vec<_> =
                        bindings.iter().map(|binding| binding.argument).collect();
                    let specialization = self.specialization_of(key.symbol, None, &arguments)?;
                    self.collect_constructor_initializers(key.symbol, specialization, reachable)?;
                }
            }

            // import plain foreign declared constructors; generic ones declare from their instances
            if let Some(resolution) = state.decisions.construct_decision(node)
                && let dir::ConstructTarget::Class { key, constructor } = &resolution.target
                && let dir::ClassConstructor::Declared { symbol } = constructor
                && key.arguments.is_empty()
                && symbol.module_id != self.module
            {
                reachable.imports.insert(*symbol);
            }

            // collect the calls selected inside a tree resolution
            if let Some(resolution) = state.decisions.tree_decision(node) {
                match &resolution.target {
                    dir::TreeTarget::Element { call, .. } | dir::TreeTarget::Fragment { call } => {
                        self.collect_call_decision(call, reachable)?;
                    }
                    dir::TreeTarget::Component { invocation, .. } => match invocation {
                        dir::TreeInvocation::Call(call) => {
                            self.collect_call_decision(call, reachable)?;
                        }
                        dir::TreeInvocation::Construct(construct) => {
                            if let dir::ConstructTarget::Class { constructor, .. } =
                                &construct.target
                                && let dir::ClassConstructor::Declared { symbol } = constructor
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
                self.collect_coercion(coercion, instance, reachable)?;
            }

            // collect instances behind value-position callable references
            match self.collect_function_reference(module, node, reachable) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(_)) => {}
                Err(error) => return Err(error),
            }

            // collect the accessor calls selected behind member reads
            if let Some(resolution) = state.decisions.member_decision(node) {
                self.collect_member_decision(resolution, reachable)?;
            }

            // collect the protocol calls selected behind subscript reads
            if let Some(resolution) = state.decisions.subscript_decision(node) {
                self.collect_subscript_decision(resolution, reachable)?;
            }

            // collect the same calls behind each assignment's read and write places
            if let Some(resolution) = state.decisions.assignment_decision(node) {
                match &resolution.read {
                    Some(dir::ReadResolution::Member(member)) => {
                        self.collect_member_decision(member, reachable)?;
                    }
                    Some(dir::ReadResolution::Subscript(subscript)) => {
                        self.collect_subscript_decision(subscript, reachable)?;
                    }
                    _ => {}
                }

                match &resolution.write {
                    dir::WriteResolution::Member(member) => {
                        self.collect_member_decision(member, reachable)?;
                    }
                    dir::WriteResolution::Subscript(subscript) => {
                        self.collect_subscript_decision(subscript, reachable)?;
                    }
                    _ => {}
                }
            }

            // collect the instance behind a resolved call
            let Some(resolution) = state.decisions.call_decision(node) else {
                continue;
            };

            self.collect_call_decision(resolution, reachable)?;
        }

        Ok(())
    }

    /// Return whether one class declares instance fields with initializers.
    pub(in crate::lower) fn class_has_field_initializers(
        &self,
        class: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(definition) = self.definition(class)? else {
            return Ok(false);
        };

        Ok(self
            .instance_fields(definition.members())
            .iter()
            .any(|field| field.initializer.is_some()))
    }

    /// Collect every field initializer body one constructor evaluates.
    pub(super) fn collect_constructor_initializers(
        &self,
        owner: dir::GlobalSymbolId,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        let Some(definition) = self.definition(owner)? else {
            return Ok(());
        };

        for field in self.instance_fields(definition.members()) {
            self.collect_field_initializer(&field, instance, reachable)?;
        }

        Ok(())
    }

    /// Collect one field's initializer body under its constructed instance.
    fn collect_field_initializer(
        &self,
        field: &NominalField,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        let Some(initializer) = field.initializer else {
            return Ok(());
        };

        // collect each initializer body once per constructed instance
        if !reachable.initializers.insert((initializer, instance)) {
            return Ok(());
        }

        let expression = initializer
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(|message| CompilerError::Internal { message })?;

        self.collect_body(initializer.module_id, expression, instance, reachable)
    }

    /// Collect the omitted-field initializers one struct construction evaluates.
    fn collect_construction_initializers(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        properties: &[dir::LocalNodeId<dir::Property>],
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // read the constructed nominal beneath its owner form
        let state = self.state(module)?;
        let Some(ty) = state.types.get_node_type_id(node) else {
            return Ok(());
        };
        let ty = self.instance_type(instance, ty)?;
        let ty = self.peel_owned(ty)?;
        let dir::Type::Application(instance) = self.ty(ty)? else {
            return Ok(());
        };
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(());
        };
        let fields = self.instance_fields(definition.members());

        // gather the field keys the construction writes
        let mut written = Vec::with_capacity(properties.len());
        for property in properties {
            if let dir::Property::Field { name, .. } = state.tree().get(*property) {
                written.push(dir::StaticKey::from(*name));
            }
        }

        // collect each omitted initializer body under the constructed instance
        let arguments = self
            .types(ty.module_id)?
            .type_ids(instance.arguments)
            .to_vec();
        let specialization = self.specialization_of(instance.symbol, None, &arguments)?;
        for field in fields {
            if written.contains(&field.key) {
                continue;
            }
            self.collect_field_initializer(&field, specialization, reachable)?;
        }

        Ok(())
    }

    /// Collect the accessor calls selected by one member resolution.
    fn collect_member_decision(
        &self,
        resolution: &dir::MemberDecision,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        match resolution {
            dir::OperationResolution::One(access) => self.collect_member_access(access, reachable),
            dir::OperationResolution::Union { arms, .. } => {
                for access in arms {
                    self.collect_member_access(access, reachable)?;
                }

                Ok(())
            }
        }
    }

    /// Collect the accessor call selected by one member access.
    fn collect_member_access(
        &self,
        access: &dir::MemberAccess,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        if let dir::MemberTarget::Call(call) = &access.target {
            self.collect_call(call, reachable)?;
        }

        Ok(())
    }

    /// Collect the protocol calls selected by one subscript resolution.
    fn collect_subscript_decision(
        &self,
        resolution: &dir::SubscriptDecision,
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
                    self.collect_member_access(access, reachable)?;
                }
                dir::SubscriptTarget::Call(call) => {
                    self.collect_call(call, reachable)?;
                }
                dir::SubscriptTarget::Index(read) => {
                    self.collect_call(&read.call, reachable)?;
                    if let dir::DereferenceTarget::Call(call) = &read.dereference.target {
                        self.collect_call(call, reachable)?;
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
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // resolve the referenced symbol and require a callable declaration
        let state = self.state(module)?;
        let Some(symbol) = state.decisions.function_symbol(node).or_else(|| {
            state
                .resolutions
                .name_resolution(node)
                .and_then(|resolution| resolution.single_symbol())
        }) else {
            return Ok(());
        };

        // import foreign module constants before any callable routing
        if symbol.module_id != self.module && self.is_module_binding(symbol)? {
            reachable.constants.insert(symbol);

            return Ok(());
        }

        // value bindings call through their bound values
        let kind = self
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id)
            .kind;
        if kind.is_binding() {
            return Ok(());
        }

        // require a declared callable type behind the reference
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

        // instantiated references declare from their instances
        if state
            .decisions
            .function_decision(node)
            .is_some_and(|decision| {
                decision
                    .arms()
                    .iter()
                    .any(|value| value.key().is_some_and(|key| !key.arguments.is_empty()))
            })
        {
            return Ok(());
        }

        // instantiating conversions declare from their instances too
        if let Some(coercion) = state.coercions.coercion(node)
            && coercion
                .adjustments
                .iter()
                .any(|adjustment| matches!(adjustment, dir::CoercionAdjustment::Instantiate { .. }))
        {
            return Ok(());
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
            if binding.memory_parameter() == Some(dir::MemoryParameter::Region) {
                continue;
            }
            parameters.push(parameter.into_global(template_module));
        }

        Ok(parameters)
    }

    /// Collect the implementing methods one erasing coercion requires.
    fn collect_coercion(
        &self,
        coercion: &dir::Coercion,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // materialize widened const literals as constant objects
        let resolved_source = self.instance_type(instance, coercion.source)?;
        self.collect_literals(resolved_source, reachable)?;

        // walk the adjustment chain, collecting each erasing step
        let mut source = coercion.source;
        for adjustment in &coercion.adjustments {
            if let dir::CoercionAdjustment::Erase { target } = adjustment {
                self.collect_erasure(source, *target, instance, reachable)?;
            }
            source = adjustment.target();
        }

        Ok(())
    }

    /// Collect string and bigint values a coercion may materialize at runtime.
    fn collect_literals(
        &self,
        source: dir::GlobalTypeId,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        match self.ty(source)? {
            dir::Type::Literal(dir::Literal::String(string)) => {
                reachable.strings.insert(string);
            }
            dir::Type::Literal(dir::Literal::Bigint(bigint)) => {
                reachable.bigints.insert(bigint);
            }
            dir::Type::Union(union) => {
                let members = self.types(source.module_id)?.type_ids(union.elements);
                for member in members {
                    self.collect_literals(*member, reachable)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Collect the methods one concrete source supplies for a constraint.
    fn collect_erasure(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        // record the resolved pair for its declared dispatch entries
        let source = self.instance_type(instance, source)?;
        let resolved_target = self.instance_type(instance, target)?;
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

        // require one declared instance per dispatched method, inherited included
        let mut pending = vec![definition.clone()];
        let mut visited = FxIndexSet::default();
        while let Some(definition) = pending.pop() {
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
                    .push(dir::InstanceKey::new(implementing, Vec::new()));
            }

            // queue the inherited interfaces the constraint extends
            let dir::Definition::Interface(interface) = &definition else {
                continue;
            };
            for heritage in &interface.extends {
                let dir::Type::Application(base) = self.ty(heritage.ty)? else {
                    continue;
                };
                if !visited.insert(base.symbol) {
                    continue;
                }
                if let Some(base) = self.definition(base.symbol)? {
                    pending.push(base.clone());
                }
            }
        }

        Ok(())
    }

    /// Collect every declaration selected by one call resolution.
    fn collect_call_decision(
        &self,
        resolution: &dir::CallDecision,
        reachable: &mut Reachable,
    ) -> CompilerResult<()> {
        match resolution {
            dir::OperationResolution::One(call) => self.collect_call(call, reachable),
            dir::OperationResolution::Union { arms, .. } => {
                for call in arms {
                    self.collect_call(call, reachable)?;
                }

                Ok(())
            }
        }
    }

    /// Collect one singular call target.
    fn collect_call(&self, call: &dir::Call, reachable: &mut Reachable) -> CompilerResult<()> {
        // only symbol targets name a declaration to collect
        let function = match &call.target {
            dir::CallableTarget::Expression { .. } | dir::CallableTarget::Dynamic { .. } => {
                return Ok(());
            }
            dir::CallableTarget::Symbol { function, .. } => function,
        };

        // route intrinsic and binding callables without an instance
        match self.callable_implementation(function.key.symbol)? {
            // declare dotted host externs for bindings
            Some(CallableImplementation::Binding { .. }) => {
                reachable.bindings.insert(function.key.symbol);

                return Ok(());
            }
            // emit intrinsics without declarations
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(()),
            None => {}
        }

        // parameter-binding calls declare from their instances; plain foreign calls import
        if function.key.symbol.module_id != self.module
            && !self.selects_parameters(&function.key.arguments)?
        {
            reachable.imports.insert(function.key.symbol);
        }

        Ok(())
    }

    /// Return whether one selection binds parameters beyond its importable ambient form.
    fn selects_parameters(
        &self,
        generic_arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<bool> {
        for binding in generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.state(parameter.module_id)?.generics;
            let parameter = generics.get_parameter(parameter.local_id);
            match parameter.kind {
                // regions erase from every declaration
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Region) => {}
                // ambient memory bindings import plainly, every other space instantiates
                dir::GenericParameterKind::Memory(_) => {
                    if self.place_space(binding.argument)? != Some(dir::Space::Local) {
                        return Ok(true);
                    }
                }
                dir::GenericParameterKind::Type => return Ok(true),
            }
        }

        Ok(false)
    }

    /// Return the substituted type bindings one candidate selects beyond lifetimes.
    pub(in crate::lower) fn instance_bindings(
        &self,
        generic_arguments: &[dir::GenericArgumentBinding],
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
    ) -> CompilerResult<Vec<dir::GenericArgumentBinding>> {
        let mut bindings = Vec::new();
        for binding in generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.state(parameter.module_id)?.generics;
            let parameter = generics.get_parameter(parameter.local_id);

            // regions select nothing, every other binding selects the specialization
            match parameter.kind {
                dir::GenericParameterKind::Memory(dir::MemoryParameter::Region) => continue,
                dir::GenericParameterKind::Type | dir::GenericParameterKind::Memory(_) => {}
            }

            // substitute arguments through the enclosing instance
            let argument = self.instance_type(instance, binding.argument)?;
            bindings.push(dir::GenericArgumentBinding {
                parameter: binding.parameter,
                argument,
            });
        }

        Ok(bindings)
    }
}
