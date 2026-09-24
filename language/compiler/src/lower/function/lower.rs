use std::sync::Arc;

use destack_core::{FxIndexMap, FxIndexSet, StringId};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::function::disposal::Disposal;
use crate::lower::function::place::Place;
use crate::lower::{
    DirModule, GenericInstanceKey, GenericScope, Memo, ModuleInitializer, ModuleLowerer,
    NominalInstance,
};
use crate::{CompilerError, CompilerResult};

/// Lowering state for one function body.
pub(in crate::lower) struct FunctionLowerer<'lower, 'builder, 'module> {
    // context
    /// The module lowering state.
    pub(in crate::lower) lower: &'lower mut ModuleLowerer<'module>,
    /// The function builder.
    pub(in crate::lower) builder: mir::FunctionBuilder<'builder>,
    /// The module declaring this function.
    pub(in crate::lower) source: ModuleId,
    /// The polymorphic lifetime parameters of this function.
    pub(in crate::lower) scope: GenericScope,
    /// The class whose constructor this body runs, when it is one.
    pub(in crate::lower) constructs: Option<dir::GlobalSymbolId>,
    /// The fields this constructor body assigns through its receiver.
    pub(in crate::lower) assigned_fields: FxIndexSet<dir::GlobalSymbolId>,

    // body state
    /// The lowered binding for each symbol.
    pub(in crate::lower) values: FxIndexMap<dir::LocalSymbolId, Binding>,
    /// The allocated capture frame for each lifted scope.
    pub(in crate::lower) frames: FxIndexMap<dir::GlobalScopeId, mir::Value>,
    /// The receiver binding of the enclosing method, when one exists.
    pub(in crate::lower) this: Option<Binding>,
    /// The producer binding of the enclosing generator body, when one exists.
    pub(in crate::lower) producer: Option<Binding>,
    /// The capture environment a coroutine entry forwards to its body.
    pub(in crate::lower) captures: Option<mir::Value>,
    /// The profile sites this body instruments.
    pub(in crate::lower) profile: mir::FunctionProfileTable,
    /// The enclosing control statements, innermost last.
    pub(in crate::lower) controls: Vec<ControlFrame>,
    /// The using resources awaiting disposal, outermost first.
    pub(in crate::lower) disposals: Vec<Disposal>,
    /// The enclosing optional chains, innermost last.
    pub(in crate::lower) chains: Vec<ChainFrame>,
    /// The enclosing try expressions catching residuals, innermost last.
    pub(in crate::lower) tries: Vec<TryFrame>,
}

/// One active try expression catching the residuals its body propagates.
pub(in crate::lower) struct TryFrame {
    /// The try expression.
    pub(in crate::lower) node: dir::LocalNodeId<dir::Expression>,
    /// The local the caught residual lands in, with the type the catch declares it at.
    pub(in crate::lower) residual: Option<(mir::LocalNodeId<mir::Local>, dir::GlobalTypeId)>,
    /// The block the catch runs in.
    pub(in crate::lower) catch: mir::LocalNodeId<mir::Block>,
    /// The disposal depth the try body opened at.
    pub(in crate::lower) disposals: usize,
}

/// One active optional chain.
pub(in crate::lower) struct ChainFrame {
    /// The place the chain value and its short circuit write.
    pub(in crate::lower) destination: Place,
    /// The block resuming after the chain.
    pub(in crate::lower) exit: mir::LocalNodeId<mir::Block>,
}

/// The home of one symbol.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Binding {
    /// A frame local.
    Local(mir::LocalNodeId<mir::Local>),
    /// A field of a managed capture frame.
    Captured {
        /// The frame reference holding the binding.
        frame: mir::Value,
        /// The field index within the frame.
        field: u32,
        /// The stored field type.
        ty: mir::TypeId,
    },
}

/// One enclosing control statement's break and continue targets.
pub(in crate::lower) struct ControlFrame {
    /// The label naming this statement, when one does.
    pub(in crate::lower) label: Option<StringId>,
    /// The block `break` enters.
    pub(in crate::lower) break_target: mir::LocalNodeId<mir::Block>,
    /// The disposal depth the statement opened at.
    pub(in crate::lower) disposals: usize,
    /// The block `continue` enters when the statement is a loop.
    pub(in crate::lower) continue_target: Option<mir::LocalNodeId<mir::Block>>,
    /// The place a valued break writes before leaving.
    pub(in crate::lower) destination: Option<Place>,
}

/// One concrete function definition awaiting lowering.
pub(in crate::lower) struct FunctionDefinition {
    /// The declared function.
    pub(in crate::lower) function: mir::FunctionId,
    /// The symbol declaring this function.
    pub(in crate::lower) symbol: dir::GlobalSymbolId,
    /// Whether the function receives this as its leading parameter.
    pub(in crate::lower) has_this: bool,
    /// The parameter symbols in order.
    pub(in crate::lower) parameters: Vec<dir::LocalSymbolId>,
    /// The polymorphic lifetime parameters of this definition.
    pub(in crate::lower) scope: GenericScope,
    /// The module declaring this body.
    pub(in crate::lower) source: destack_source::ModuleId,
    /// The class this constructor body initializes, when one exists.
    pub(in crate::lower) constructs: Option<dir::GlobalSymbolId>,
    /// The declared default expression of each parameter, in header order.
    pub(in crate::lower) defaults: Vec<Option<dir::LocalNodeId<dir::Expression>>>,
    /// The body this definition lowers.
    pub(in crate::lower) body: Body,
}

/// The body one queued definition lowers.
#[derive(Clone)]
pub(in crate::lower) enum Body {
    /// The declared expression, lowered as the whole function.
    Plain(dir::LocalNodeId<dir::Expression>),
    /// The coroutine entry synthesized around its extracted body function.
    CoroutineEntry {
        /// The declared expression the extracted body lowers.
        expression: dir::LocalNodeId<dir::Expression>,
        /// The extracted body function receiving the environment.
        body: mir::FunctionId,
    },
    /// The extracted coroutine body, its parameters rebound from the environment.
    Coroutine(dir::LocalNodeId<dir::Expression>),
    /// The synthesized default constructor storing a class's field initializers.
    DefaultConstructor,
    /// The allocating entry of a first-class constructor.
    Constructor(Box<dir::ConstructDecision>),
}

impl<'lower, 'builder, 'module> FunctionLowerer<'lower, 'builder, 'module> {
    /// Open the lowering of one body over its builder, the entry block current.
    fn new(
        lower: &'lower mut ModuleLowerer<'module>,
        mut builder: mir::FunctionBuilder<'builder>,
        source: ModuleId,
        scope: GenericScope,
        constructs: Option<dir::GlobalSymbolId>,
    ) -> Self {
        let entry = builder.block();
        builder.switch_to_block(entry);

        Self {
            lower,
            builder,
            source,
            scope,
            constructs,
            assigned_fields: FxIndexSet::default(),
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            producer: None,
            captures: None,
            profile: mir::FunctionProfileTable::new(mir::FunctionHash::default()),
            controls: Vec::new(),
            disposals: Vec::new(),
            chains: Vec::new(),
            tries: Vec::new(),
        }
    }

    /// Lower one declared function body.
    pub(in crate::lower) fn lower(
        lower: &mut ModuleLowerer<'_>,
        builder: &mut mir::ModuleBuilder,
        definition: FunctionDefinition,
    ) -> CompilerResult<()> {
        // unpack the queued definition
        let FunctionDefinition {
            function,
            symbol,
            has_this,
            parameters,
            scope,
            source,
            constructs,
            defaults,
            body,
        } = definition;

        // open the declared body and stand up the lowering state around it
        lower.state(source)?;
        let function_id = function;
        let body_builder =
            builder
                .function_body(function)
                .map_err(|error| CompilerError::Internal {
                    message: format!("an unopened function body: {error}"),
                })?;
        let mut function = FunctionLowerer::new(lower, body_builder, source, scope, constructs);

        // home the parameters in header order past any receiver, an extracted body's from its environment
        let shift = has_this as usize;
        match body {
            Body::Coroutine(_) => {
                function.bind_coroutine_parameters(symbol, &parameters)?;

                // home the producer a generator body receives as its leading parameter
                if !function.function_parameters(function_id).is_empty() {
                    let producer = function.builder.function_parameter(0);
                    function.producer = Some(function.bind_receiver(producer)?);
                }
            }
            _ => {
                for (index, symbol) in parameters.iter().enumerate() {
                    let value = function.builder.function_parameter(index + shift);
                    let local = function.home(value);
                    function.values.insert(*symbol, Binding::Local(local));
                }
            }
        }

        // anchor the body's synthesized nodes at the declaration
        if let Some(anchor) = function.lower.declaration_anchor(symbol)? {
            function.builder.replace_source(Some(anchor));
        }

        // home the receiver ahead of anything reading it
        if has_this {
            let value = function.builder.function_parameter(0);
            function.this = Some(function.bind_receiver(value)?);
        }

        // lower the selected body through its declared or generated operations
        match body {
            Body::Constructor(construction) => {
                function.lower_constructor_body(&construction)?;
            }
            Body::DefaultConstructor => {
                function.lower_field_initializers(symbol)?;
                function.return_value(None)?;
            }
            Body::Plain(expression)
            | Body::CoroutineEntry { expression, .. }
            | Body::Coroutine(expression) => {
                // bind captures before evaluating parameter defaults
                let is_entry = matches!(body, Body::CoroutineEntry { .. });
                function.bind_captures(symbol, is_entry)?;
                function.lower_parameter_defaults(&parameters, &defaults)?;

                // lift the parameters captured by nested functions
                for symbol in &parameters {
                    let global = symbol.into_global(source);
                    if let Some(Binding::Local(local)) = function.values.get(symbol).copied()
                        && function.is_lifted(global)
                    {
                        let value = function.builder.local_get(local);
                        function.bind_lifted(global, value)?;
                    }
                }

                // initialize fields from the assignments recorded for a class constructor
                if let Some(class) = constructs {
                    let Some(assigned) = function.source().decisions.constructor_assignments(class)
                    else {
                        return Err(CompilerError::Internal {
                            message: "a constructor without its recorded field assignments"
                                .to_string(),
                        });
                    };
                    function.assigned_fields = assigned.iter().copied().collect();
                    if !function.lower.class_extends_base(class)? {
                        function.lower_field_initializers(class)?;
                    }
                }

                // emit the coroutine entry or declared expression
                match body {
                    Body::CoroutineEntry { body, .. } => {
                        function.lower_coroutine_entry(symbol, &parameters, body)?;
                    }
                    _ => function.lower_body(expression)?,
                }
            }
        }

        // register the profile sites the body instruments
        let profile = function.finish()?;
        if !profile.counters.is_empty() || !profile.samplers.is_empty() {
            builder.profile_mut().insert_function(function_id, profile);
        }

        Ok(())
    }

    /// Seal the lowered body, answering the profile sites it instruments.
    fn finish(self) -> CompilerResult<mir::FunctionProfileTable> {
        let mut body = self.builder;
        body.seal_all_blocks();
        body.finish().map_err(|error| CompilerError::Internal {
            message: format!("an unfinished function body: {error}"),
        })?;

        Ok(self.profile)
    }

    /// Lower the module initializer storing each runtime binding.
    pub(in crate::lower) fn lower_initializer(
        lower: &mut ModuleLowerer<'_>,
        builder: &mut mir::ModuleBuilder,
        function: mir::FunctionId,
        initializers: Vec<ModuleInitializer>,
    ) -> CompilerResult<()> {
        // open the initializer body and stand up the lowering state around it
        let source = lower.module;
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("an unopened function body: {error}"),
            })?;
        let mut function =
            FunctionLowerer::new(lower, builder, source, GenericScope::default(), None);

        // run each step in source order
        for initializer in initializers {
            match initializer {
                ModuleInitializer::Binding { global, value } => {
                    let value = function.lower_value(value)?;
                    function.builder.store_global(global, value);
                }
                ModuleInitializer::Statement(statement) => {
                    // end the initializer at a terminated statement
                    if function.lower_statement(statement)? {
                        function.finish()?;

                        return Ok(());
                    }
                }
            }
        }

        // close the initializer with a void return
        function.return_value(None)?;
        function.finish()?;

        Ok(())
    }

    /// Lower one expression from another module's tree.
    pub(in crate::lower) fn lower_foreign_expression(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // swap into the declaring module with no local bindings in scope
        self.lower.state(module)?;
        let source = std::mem::replace(&mut self.source, module);
        let values = std::mem::take(&mut self.values);
        let frames = std::mem::take(&mut self.frames);
        let this = self.this.take();

        // lower the expression against the declaring module
        let value = self.lower_value(expression);

        // swap the enclosing body's state back in, keeping the outcome
        self.source = source;
        self.values = values;
        self.frames = frames;
        self.this = this;

        value
    }

    /// Lower one node through a step with its emitted MIR anchored at the node's source extent.
    pub(in crate::lower) fn lower_anchored<R>(
        &mut self,
        node: dir::LocalNodeId<dir::Expression>,
        lower: impl FnOnce(&mut Self) -> CompilerResult<R>,
    ) -> CompilerResult<R> {
        // keep the enclosing anchor over an extent-less synthesized node
        let Some(span) = self.source().tree().get_source_extent_by_id(node.id) else {
            return lower(self);
        };

        // anchor the emitted MIR at the node's extent
        let previous = self.builder.replace_source(Some((node.id, span)));
        let result = lower(self);
        self.builder.replace_source(previous);

        result
    }

    /// Return the state of the module declaring this function.
    pub(in crate::lower) fn source(&self) -> &DirModule {
        match self.lower.modules.get(&self.source) {
            Some(state) => state,
            None => unreachable!("the function source module is always loaded"),
        }
    }

    /// Return the representation type of one lowered value.
    pub(in crate::lower) fn value_representation(
        &self,
        value: mir::Value,
    ) -> CompilerResult<mir::TypeId> {
        self.builder
            .value_type(value)
            .ok_or_else(|| CompilerError::Internal {
                message: "a lowered value without a representation".to_string(),
            })
    }

    /// Return from the function, a value adapted to the declared result representation.
    pub(in crate::lower) fn return_value(
        &mut self,
        value: Option<mir::Value>,
    ) -> CompilerResult<()> {
        let value = match value {
            Some(value) => {
                // leave a diverged value, its path ending before the return
                let representation = self.value_representation(value)?;
                if matches!(
                    self.builder.tree().type_definition(representation),
                    mir::Type::Never
                ) {
                    self.builder.unreachable();

                    return Ok(());
                }

                Some(value)
            }
            None => None,
        };
        self.builder.return_(value);

        Ok(())
    }

    /// Lower one generic argument under the enclosing template.
    pub(in crate::lower) fn lower_generic_argument(
        &mut self,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<mir::GenericArgument> {
        self.lower
            .type_lowerer(self.builder.tree_mut(), &self.scope)
            .lower_generic_argument(argument)
    }

    /// Return the heap of one allocation: a managed result's space, else the receiver's space.
    pub(in crate::lower) fn allocation_space(
        &mut self,
        result: mir::TypeId,
    ) -> CompilerResult<mir::Space> {
        let definition = self.builder.tree().type_definition(result);
        if let Some(mir::Storage::Heap(space)) = definition.reference_storage() {
            return Ok(space);
        }

        self.owned_space()
    }

    /// Return the declared space of the receiver's type, local outside placed type members.
    fn owned_space(&mut self) -> CompilerResult<mir::Space> {
        let Some(mut ty) = self.scope.this_parameter else {
            return Ok(mir::Space::Local);
        };

        // peel the receiver's forms down to its nominal declaration
        while let dir::Type::Form(form) = self.lower.ty(ty)? {
            ty = form.value;
        }
        let Some(symbol) = self.lower.nominal_symbol(ty)? else {
            return Ok(mir::Space::Local);
        };
        let space = self.lower.nominal_space(symbol)?;

        Ok(space.map_or(mir::Space::Local, ModuleLowerer::mir_space))
    }

    /// Return the representation of one type, lowering it at first read.
    pub(in crate::lower) fn lower_type(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        self.memoized(
            id,
            |lower| &mut lower.representations,
            |function| {
                function
                    .lower
                    .type_lowerer(function.builder.tree_mut(), &function.scope)
                    .lower(id)
            },
        )
    }

    /// Return the dispatch shape of one constraint, lowering it at first read.
    pub(in crate::lower) fn lower_constraint(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::TypeId> {
        self.memoized(
            id,
            |lower| &mut lower.constraints,
            |function| {
                function
                    .lower
                    .type_lowerer(function.builder.tree_mut(), &function.scope)
                    .lower_dynamic_constraint(id)
            },
        )
    }

    /// Return the declared instance key of one instantiated callable.
    pub(in crate::lower) fn generic_instance_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<GenericInstanceKey> {
        self.lower
            .type_lowerer(self.builder.tree_mut(), &self.scope)
            .generic_instance_key(symbol, receiver, types)
    }

    /// Return the nominal instance beneath one value type, lowering it at first read.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<NominalInstance> {
        // peel the value type down to the nominal it stores
        let stored = match self.lower.indirection(id, &self.scope)? {
            Some(reference) => reference.stored,
            None => self.lower.stored(id)?,
        };

        let dir::Type::Application(_) = self.lower.ty(stored)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a nominal read of a '{}' type",
                    self.lower.ty(stored)?.variant_name()
                ),
            });
        };

        self.memoized(
            stored,
            |lower| &mut lower.stored_nominals,
            |function| {
                function
                    .lower
                    .type_lowerer(function.builder.tree_mut(), &function.scope)
                    .lower_nominal(stored)
            },
        )
    }

    /// Lower one key once for every body that reads it.
    fn memoized<T: Clone>(
        &mut self,
        key: dir::GlobalTypeId,
        memo: impl for<'m> Fn(&'m mut ModuleLowerer<'module>) -> &'m mut Memo<dir::GlobalTypeId, T>,
        lower: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        if !memo(self.lower).contains_key(&key) {
            let outcome = match lower(self) {
                Ok(value) => Ok(value),
                Err(CompilerError::Diagnostic(diagnostic)) => Err(Arc::from(diagnostic)),
                Err(error) => return Err(error),
            };
            memo(self.lower).insert(key, outcome);
        }

        match &memo(self.lower)[&key] {
            Ok(value) => Ok(value.clone()),
            Err(diagnostic) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
        }
    }

    /// Intern one reference type over a lowered pointee at one lifetime.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::Reference,
        lifetime: mir::Lifetime,
        access: mir::Access,
        pointee: mir::TypeId,
    ) -> mir::TypeId {
        self.builder.tree_mut().intern_type(mir::Type::Reference {
            kind,
            lifetime,
            access,
            pointee,
        })
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one expression to the value it produces.
    pub(in crate::lower) fn lower_expression_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // run a diverging jump for its control flow, then yield its dead value
        if matches!(
            self.source().tree().get(expression),
            dir::Expression::Return { .. }
                | dir::Expression::Break { .. }
                | dir::Expression::Continue { .. }
        ) {
            self.lower_statement(expression)?;

            return self.dead_value(expression);
        }

        // lower by the expression's own syntax
        match self.source().tree().get(expression).clone() {
            // read the value behind a name
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;

                self.read_symbol(expression, symbol)
            }

            // materialize a scalar literal
            dir::Expression::Literal(literal) => self.lower_scalar_literal(expression, literal),

            // render and join an interpolated template
            dir::Expression::TemplateExpression {
                value: dir::TemplateLiteral::InterpolatedString { chunks, arguments },
            } => self.lower_template_expression(expression, &chunks, &arguments),

            // bind a closure declaration as a function value
            dir::Expression::Declaration(declaration) => {
                let node = declaration.into_global_any(self.source);
                let Some(symbol) = self.lower.symbol_declared_at(node)? else {
                    return Err(CompilerError::Internal {
                        message: "a declaration expression without a symbol".to_string(),
                    });
                };

                // bind the declared closure over its captured environment
                let environment = self.capture_environment(symbol)?;
                match self.lower_function_value(expression, symbol, environment)? {
                    Some(value) => Ok(value),
                    None => Err(self.internal("a non-callable declaration expression")),
                }
            }

            // apply a binary operator
            dir::Expression::Binary {
                left,
                operator: _,
                right,
            } => {
                let resolution = self.operator_decision(expression)?;
                let dir::OperationResolution::One(application) = resolution else {
                    return Err(self.unsupported("a binary operator on union operands"));
                };
                match application {
                    dir::OperatorApplication::Binary {
                        operator,
                        target: dir::OperatorTarget::Builtin(operands),
                        ..
                    } => match operator {
                        dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                            self.lower_strict_equality(left, operator, right, &operands)
                        }
                        dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                            self.lower_logical(expression, left, operator, right)
                        }
                        dir::BinaryOperator::Coalesce => {
                            self.lower_coalesce(expression, left, right)
                        }
                        operator => self.lower_binary(left, operator, right, &operands),
                    },
                    // dispatch protocol operators as left.method(right)
                    dir::OperatorApplication::Binary {
                        operator,
                        target: dir::OperatorTarget::Call(call),
                        ..
                    } => match &*call {
                        dir::Call {
                            target:
                                dir::CallableTarget::Symbol {
                                    function,
                                    dispatch: dir::FunctionDispatch::Direct,
                                },
                            ..
                        } => {
                            let value = self.lower_operator_method(left, &call, function)?;

                            self.lower_comparison_result(operator, value, call.return_type)
                        }
                        _ => Err(CompilerError::Internal {
                            message: "a non-callable binary operator".to_string(),
                        }),
                    },
                    dir::OperatorApplication::Unary { .. } => Err(CompilerError::Internal {
                        message: "a unary resolution for a binary expression".to_string(),
                    }),
                }
            }

            // apply a unary operator
            dir::Expression::Unary { operator: _, right } => {
                let resolution = self.operator_decision(expression)?;
                let dir::OperationResolution::One(application) = resolution else {
                    return Err(self.unsupported("a unary operator on a union operand"));
                };
                match application {
                    // read the value one reference addresses
                    dir::OperatorApplication::Unary {
                        operator: dir::UnaryOperator::Dereference,
                        target: dir::OperatorTarget::Builtin(_),
                        ..
                    } => {
                        let reference = self.lower_value(right)?;
                        let ty = self.node_type_id(expression)?;
                        let pointee = self.lower_type(ty)?;

                        Ok(self.load_place(
                            mir::Place::value(reference).with_projection(mir::Projection::Deref),
                            pointee,
                        ))
                    }
                    dir::OperatorApplication::Unary {
                        operator,
                        target: dir::OperatorTarget::Builtin(operand),
                        ..
                    } => self.lower_unary(operator, right, &operand),
                    // dispatch protocol operators as right.method()
                    dir::OperatorApplication::Unary {
                        target: dir::OperatorTarget::Call(call),
                        ..
                    } => match &*call {
                        dir::Call {
                            target:
                                dir::CallableTarget::Symbol {
                                    function,
                                    dispatch: dir::FunctionDispatch::Direct,
                                },
                            ..
                        } => self.lower_operator_method(right, &call, function),
                        _ => Err(CompilerError::Internal {
                            message: "a non-callable unary operator".to_string(),
                        }),
                    },
                    dir::OperatorApplication::Binary { .. } => Err(CompilerError::Internal {
                        message: "a binary resolution for a unary expression".to_string(),
                    }),
                }
            }

            // join the arms of a match, a block, or a loop through a destination
            dir::Expression::Match { .. }
            | dir::Expression::Block(_)
            | dir::Expression::Loop { .. }
            | dir::Expression::Try { .. } => self.lower_joined(expression),

            // take the value an assignment writes
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => match self.lower_assign(expression, left, operator, right)? {
                Some(value) => Ok(value),
                None => Err(self.internal("a member write in value position")),
            },

            // read a satisfies expression as its own value
            dir::Expression::Satisfies {
                expression: inner, ..
            } => self.lower_value(inner),

            // build the range family its written bounds name
            dir::Expression::RangeExpression { start, end, .. } => {
                self.lower_range_expression(expression, start, end)
            }

            // repeat one value across its fixed array storage
            dir::Expression::FixedArrayExpression { value, .. } => {
                self.lower_fixed_array_expression(expression, value)
            }

            // join the arm values of a value if or ternary
            dir::Expression::If { .. } => self.lower_joined(expression),

            // build an anonymous object
            dir::Expression::ObjectExpression { properties } => {
                let properties = properties.into_iter().collect::<Vec<_>>();

                self.lower_object_expression(expression, &properties)
            }

            // build a named struct
            dir::Expression::StructExpression { properties, .. } => {
                self.lower_struct_expression(expression, &properties)
            }

            // build a tuple
            dir::Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression, &elements)
            }

            // read the receiver binding, projecting a flow-narrowed read onto its narrowing
            dir::Expression::This => {
                let Some(binding) = self.this else {
                    return Err(CompilerError::Internal {
                        message: "a this outside a method body".to_string(),
                    });
                };
                let value = self.read_binding(binding)?;
                let value = self.constructed_this(expression, value)?;

                self.lower_narrowing(expression, value)
            }

            // read the receiver at the base its heritage prefixes
            dir::Expression::Super => {
                let Some(binding) = self.this else {
                    return Err(CompilerError::Internal {
                        message: "a super outside a method body".to_string(),
                    });
                };
                let value = self.read_binding(binding)?;
                let base = self.lower_type(self.node_type_id(expression)?)?;

                self.adopt(value, base)
            }

            // read the callable one instantiation selects
            dir::Expression::Instantiation { left, .. } => {
                let symbol = self
                    .lower
                    .resolved_symbol(left.into_global_any(self.source))?;

                self.read_symbol(expression, symbol)
            }

            // borrow the operand's place
            dir::Expression::BorrowOf { right, .. } => {
                let target = self.lower_type(self.node_type_id(expression)?)?;

                self.lower_borrowed_place(right, target)
            }

            // read a member
            dir::Expression::Member { left, .. } => self.lower_member(expression, left),

            // join an optional chain
            dir::Expression::Chain { expression: inner } => self.lower_chain(expression, inner),

            // read a subscript
            dir::Expression::Index { left, index, .. } => {
                self.lower_subscript(expression, left, index)
            }

            // cast to the written type
            dir::Expression::As {
                expression: value, ..
            } => self.lower_as(expression, value),

            // initialize a nominal value through its selected constructor
            dir::Expression::Call { .. } | dir::Expression::New { .. }
                if let Some(resolution) = self.construct_decision(expression) =>
            {
                self.lower_construct(expression, &resolution)
            }

            // build an array
            dir::Expression::ArrayExpression { elements } => {
                self.lower_array_expression(expression, &elements)
            }

            // build a tree literal
            dir::Expression::TreeExpression { .. } => {
                let resolution = self.tree_decision(expression)?;

                self.lower_tree(&resolution)
            }

            // assert the operand onto the cases it holds
            dir::Expression::Must { left, .. } => self.lower_must(expression, left),

            // continue with the output a propagating try projection produces
            dir::Expression::Maybe { left, .. } => self.lower_maybe(expression, left),

            // test the operand against the recorded runtime predicate
            dir::Expression::Is { value, .. } | dir::Expression::InstanceOf { value, .. } => {
                self.lower_guard(expression, value)
            }

            // resume with the value the generator's consumer sends
            dir::Expression::Yield { .. } => {
                let value = self.lower_yield(expression)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "a void yield in value position".to_string(),
                })
            }

            // call and take the value it produces, awaits running their recorded park call
            dir::Expression::Call { .. }
            | dir::Expression::New { .. }
            | dir::Expression::Await { .. } => {
                let value = self.lower_call(expression)?;

                // return the value the call produced
                if let Some(value) = value {
                    return Ok(value);
                }

                // yield one dead uninit value behind a diverging call's sealed block
                let ty = self.node_type_id(expression)?;
                if matches!(self.lower.ty(ty)?, dir::Type::Never) {
                    let never = self.lower_type(ty)?;

                    return Ok(self.builder.constant(mir::Constant::Uninit, never));
                }

                // yield the unique inhabitant of a zero sized result
                let result_type = self.lower_type(ty)?;
                if matches!(self.builder.tree().get(result_type), mir::Type::Void) {
                    return Ok(self.builder.constant(mir::Constant::Zeroed, result_type));
                }

                // reject a call that produced no value in value position
                Err(CompilerError::Internal {
                    message: "a void call in value position".to_string(),
                })
            }

            // reject every other expression
            other => Err(self.unsupported(format!("'{}' expressions", other.variant_name()))),
        }
    }
}
