use std::sync::Arc;

use destack_core::{FxIndexMap, StringId};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    GenericInstanceKey, LifetimeParameters, LowerModuleState, LowerState, Lowered, NominalInstance,
    insert_reference_type,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// Lowering state for one function body.
pub(in crate::lower) struct FunctionLowerer<'lower, 'builder, 'module> {
    // context
    /// The module lowering state.
    pub(in crate::lower) lower: &'lower mut LowerState<'module>,
    /// The function builder.
    pub(in crate::lower) builder: mir::FunctionBuilder<'builder>,
    /// The module declaring this function.
    pub(in crate::lower) source: ModuleId,
    /// The materialized instance this function specializes, when generic.
    pub(in crate::lower) instance: Option<(ModuleId, dir::LocalInstanceId)>,
    /// The polymorphic lifetime parameters of this function.
    pub(in crate::lower) lifetime_parameters: LifetimeParameters,
    /// The class whose constructor this body runs, when it is one.
    pub(in crate::lower) constructs: Option<dir::GlobalSymbolId>,

    // body state
    /// The lowered binding for each symbol.
    pub(in crate::lower) values: FxIndexMap<dir::LocalSymbolId, Binding>,
    /// The allocated capture frame for each lifted scope.
    pub(in crate::lower) frames: FxIndexMap<dir::GlobalScopeId, mir::Value>,
    /// The receiver binding of the enclosing method, when one exists.
    pub(in crate::lower) this: Option<Binding>,
    /// The enclosing control statements, innermost last.
    pub(in crate::lower) controls: Vec<ControlFrame>,
}

/// One lowered value bound to a symbol.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Binding {
    /// An immutable SSA value.
    Value(mir::Value),
    /// A mutable local.
    Local(mir::LocalNodeId<mir::Local>),
    /// A field of a managed capture frame.
    Captured {
        /// The frame reference holding the binding.
        frame: mir::Value,
        /// The field index within the frame.
        field: u32,
        /// The stored field type.
        ty: mir::LocalNodeId<mir::Type>,
    },
}

/// One enclosing control statement's break and continue targets.
pub(in crate::lower) struct ControlFrame {
    /// The label naming this statement, when one does.
    pub(in crate::lower) label: Option<StringId>,
    /// The block `break` enters.
    pub(in crate::lower) break_target: mir::LocalNodeId<mir::Block>,
    /// The block `continue` enters when the statement is a loop.
    pub(in crate::lower) continue_target: Option<mir::LocalNodeId<mir::Block>>,
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
    /// The materialized instance this definition specializes, when generic.
    pub(in crate::lower) instance: Option<(ModuleId, dir::LocalInstanceId)>,
    /// The polymorphic lifetime parameters of this definition.
    pub(in crate::lower) lifetime_parameters: LifetimeParameters,
    /// The module declaring this body.
    pub(in crate::lower) source: destack_source::ModuleId,
    /// The body expression.
    pub(in crate::lower) expression: dir::LocalNodeId<dir::Expression>,
    /// The class this constructor body initializes, when one exists.
    pub(in crate::lower) constructs: Option<dir::GlobalSymbolId>,
    /// The declared default expression of each parameter, in header order.
    pub(in crate::lower) defaults: Vec<Option<dir::LocalNodeId<dir::Expression>>>,
}

impl<'module> FunctionLowerer<'_, '_, 'module> {
    /// Lower one declared function body.
    pub(in crate::lower) fn lower(
        lower: &mut LowerState<'_>,
        builder: &mut mir::ModuleBuilder,
        definition: FunctionDefinition,
    ) -> CompilerResult<()> {
        // unpack the queued definition
        let FunctionDefinition {
            function,
            symbol,
            has_this,
            parameters,
            instance,
            lifetime_parameters,
            source,
            expression,
            constructs,
            defaults,
        } = definition;

        // open the declared body and stand up the lowering state around it
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("an unopened function body: {error}"),
            })?;
        let mut function = FunctionLowerer {
            lower,
            builder,
            source,
            instance,
            lifetime_parameters,
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            constructs,
            controls: Vec::new(),
        };

        // bind the parameters in header order past any receiver
        let shift = has_this as usize;
        for (index, symbol) in parameters.iter().enumerate() {
            let value = function.builder.function_parameter(index + shift);
            function.values.insert(*symbol, Binding::Value(value));
        }

        // open the entry block
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);

        // home the receiver ahead of anything reading it
        if has_this {
            let value = function.builder.function_parameter(0);
            function.this = Some(function.bind_receiver(value)?);
        }

        // receive the closure environment before defaults can read captures
        function.bind_captures(symbol)?;

        // resolve the defaulted parameters before anything reads them
        function.lower_parameter_defaults(&parameters, &defaults)?;

        // lift every parameter a nested scope captures into its frame
        for symbol in &parameters {
            if let Some(Binding::Value(value)) = function.values.get(symbol).copied() {
                function.bind_lifted(symbol.into_global(source), value)?;
            }
        }

        // store the declared field initializers before a base class constructor body
        if let Some(owner) = constructs
            && !function.lower.class_extends_base(owner)?
        {
            function.lower_field_initializers(owner)?;
        }

        // lower the body and finalize its blocks
        function.lower_body(expression)?;
        function.builder.seal_all_blocks();
        function
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("an unfinished function body: {error}"),
            })?;

        Ok(())
    }

    /// Lower one synthesized default constructor to its initializer prologue.
    pub(in crate::lower) fn lower_default_constructor(
        lower: &mut LowerState<'_>,
        builder: &mut mir::ModuleBuilder,
        class: dir::GlobalSymbolId,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        function: mir::FunctionId,
    ) -> CompilerResult<()> {
        // open the synthesized body and stand up the lowering state around it
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("an unopened function body: {error}"),
            })?;
        let mut function = FunctionLowerer {
            lower,
            builder,
            source: class.module_id,
            instance,
            lifetime_parameters: LifetimeParameters::default(),
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            constructs: None,
            controls: Vec::new(),
        };

        // open the entry block and home the receiver like any declared constructor
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);
        let value = function.builder.function_parameter(0);
        function.this = Some(function.bind_receiver(value)?);

        // store the declared field initializers the synthesized body stands in for
        function.lower_field_initializers(class)?;

        // close the constructor with a void return
        function.builder.return_(None);
        function.builder.seal_all_blocks();
        function
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("an unfinished function body: {error}"),
            })?;

        Ok(())
    }

    /// Lower one synthesized builtin clone to a copy of its receiver's pointee.
    pub(in crate::lower) fn lower_builtin_clone(
        lower: &mut LowerState<'_>,
        builder: &mut mir::ModuleBuilder,
        function: mir::FunctionId,
        result: mir::TypeId,
    ) -> CompilerResult<()> {
        // open the synthesized body and stand up the lowering state around it
        let module = lower.module;
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("an unopened function body: {error}"),
            })?;
        let mut lowering = FunctionLowerer {
            lower,
            builder,
            source: module,
            instance: None,
            lifetime_parameters: LifetimeParameters::default(),
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            constructs: None,
            controls: Vec::new(),
        };

        // copy the borrowed receiver's value and return it
        let entry = lowering.builder.block();
        lowering.builder.switch_to_block(entry);
        let receiver = lowering.builder.function_parameter(0);
        let value = lowering.builder.load(receiver, result);
        lowering.builder.return_(Some(value));

        // finalize the synthesized blocks
        lowering.builder.seal_all_blocks();
        lowering
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("an unfinished function body: {error}"),
            })?;

        Ok(())
    }

    /// Lower the module initializer storing each runtime binding.
    pub(in crate::lower) fn lower_initializer(
        lower: &mut LowerState<'_>,
        builder: &mut mir::ModuleBuilder,
        function: mir::FunctionId,
        initializers: Vec<(
            mir::LocalNodeId<mir::Global>,
            dir::LocalNodeId<dir::Expression>,
        )>,
    ) -> CompilerResult<()> {
        // open the initializer body and stand up the lowering state around it
        let source = lower.module;
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("an unopened function body: {error}"),
            })?;
        let mut function = FunctionLowerer {
            lower,
            builder,
            source,
            instance: None,
            lifetime_parameters: LifetimeParameters::default(),
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            constructs: None,
            controls: Vec::new(),
        };

        // open the entry block
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);

        // store each binding in declaration order
        for (global, expression) in initializers {
            let value = function.lower_expression(expression)?;
            function.builder.store_global(global, value);
        }

        // close the initializer with a void return
        function.builder.return_(None);
        function.builder.seal_all_blocks();
        function
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("an unfinished function body: {error}"),
            })?;

        Ok(())
    }

    /// Lower one expression from another module's tree.
    pub(in crate::lower) fn lower_foreign_expression(
        &mut self,
        module: ModuleId,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // swap into the declaring module with no local bindings in scope
        let source = std::mem::replace(&mut self.source, module);
        let outer_instance = std::mem::replace(&mut self.instance, instance);
        let values = std::mem::take(&mut self.values);
        let frames = std::mem::take(&mut self.frames);
        let this = self.this.take();

        // lower the expression against the declaring module
        let value = self.lower_expression(expression);

        // swap the enclosing body's state back in, keeping the outcome
        self.source = source;
        self.instance = outer_instance;
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
    pub(in crate::lower) fn source(&self) -> &LowerModuleState {
        match self.lower.modules.get(&self.source) {
            Some(state) => state,
            None => unreachable!("the function source module is always loaded"),
        }
    }

    /// Return the representation type of one lowered value.
    pub(in crate::lower) fn value_representation(
        &self,
        value: mir::Value,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        self.builder
            .value_type(value)
            .ok_or_else(|| CompilerError::Internal {
                message: "a lowered value without a representation".to_string(),
            })
    }

    /// Allocate one join local typed as one expression's runtime type.
    pub(in crate::lower) fn value_slot(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Local>> {
        let ty = self.node_type_id(expression)?;
        let ty = self.lower_type(ty)?;

        Ok(self.builder.local(ty, mir::Mutability::Mutable))
    }

    /// Return the representation of one type, lowering it at first read.
    pub(in crate::lower) fn lower_type(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the written id through its materialized types
        let id = self.lower.instance_type(self.instance, id)?;

        // lower the representation once for every body that reads it
        if !self.lower.representations.contains_key(&id) {
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lower
                .type_lowerer(
                    self.builder.tree_mut(),
                    pointer_bytes,
                    &self.lifetime_parameters,
                )
                .with_instance(self.instance)
                .lower(id);
            Self::bank(&mut self.lower.representations, id, outcome)?;
        }

        // read the banked outcome, cascading the kept failure
        match &self.lower.representations[&id] {
            Ok(node) => Ok(*node),
            Err(diagnostic) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
        }
    }

    /// Return the dispatch shape of one constraint, lowering it at first read.
    pub(in crate::lower) fn lower_constraint(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the written id through its materialized types
        let id = self.lower.instance_type(self.instance, id)?;

        // lower the dispatch shape once for every body that reads it
        if !self.lower.constraints.contains_key(&id) {
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lower
                .type_lowerer(
                    self.builder.tree_mut(),
                    pointer_bytes,
                    &self.lifetime_parameters,
                )
                .with_instance(self.instance)
                .lower_dynamic_constraint(id);
            Self::bank(&mut self.lower.constraints, id, outcome)?;
        }

        // read the banked outcome, cascading the kept failure
        match &self.lower.constraints[&id] {
            Ok(shape) => Ok(*shape),
            Err(diagnostic) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
        }
    }

    /// Return the declared instance key of one instantiated callable.
    pub(in crate::lower) fn generic_instance_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<GenericInstanceKey> {
        // lower the receiver into a static type argument
        let receiver = match receiver {
            Some(ty) => {
                let node = self.lower_type(ty)?;
                let ty = mir::TypeId::from(node);
                Some(self.builder.tree_mut().intern_static(mir::Static::Type(ty)))
            }
            None => None,
        };

        // bind each concrete argument as a static
        let mut arguments = Vec::with_capacity(types.len());
        for ty in types {
            // skip lifetime arguments
            if self.lower.type_is_lifetime(*ty)? {
                continue;
            }

            match self.lower.place_space(*ty)? {
                // local place arguments canonicalize onto the plain declaration
                Some(dir::Space::Local) => {}
                // other place arguments bind their space
                Some(space) => {
                    let space = LowerState::mir_space(space);
                    let argument = self
                        .builder
                        .tree_mut()
                        .intern_static(mir::Static::Space(space));
                    arguments.push(argument);
                }
                // every other argument binds a type
                None => {
                    let node = self.lower_type(*ty)?;
                    let argument = self
                        .builder
                        .tree_mut()
                        .intern_static(mir::Static::Type(mir::TypeId::from(node)));
                    arguments.push(argument);
                }
            }
        }

        Ok(GenericInstanceKey {
            symbol,
            receiver,
            arguments,
        })
    }

    /// Return the nominal instance beneath one value type, lowering it at first read.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<NominalInstance> {
        // peel the value type down to the nominal it stores
        let stored = match self.lower.peel_indirection(id)? {
            Some(reference) => reference.stored,
            None => self.lower.peel_owned(id)?,
        };

        // lower the nominal instance once for every body that reads it
        if !self.lower.stored_nominals.contains_key(&stored) {
            let dir::Type::Application(_) = self.lower.ty(stored)? else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "a nominal read of a '{}' type",
                        self.lower.ty(stored)?.variant_name()
                    ),
                });
            };
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lower
                .type_lowerer(
                    self.builder.tree_mut(),
                    pointer_bytes,
                    &self.lifetime_parameters,
                )
                .with_instance(self.instance)
                .lower_nominal(stored);
            Self::bank(&mut self.lower.stored_nominals, stored, outcome)?;
        }

        // read the banked outcome, cascading the kept failure
        match &self.lower.stored_nominals[&stored] {
            Ok(nominal) => Ok(nominal.clone()),
            Err(diagnostic) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
        }
    }

    /// Bank one lowering outcome under its key for every body that reads it.
    fn bank<T>(
        outcomes: &mut FxIndexMap<dir::GlobalTypeId, Lowered<T>>,
        key: dir::GlobalTypeId,
        outcome: CompilerResult<T>,
    ) -> CompilerResult<()> {
        match outcome {
            // keep the lowered value
            Ok(value) => {
                outcomes.insert(key, Ok(value));
            }
            // keep the diagnostic so every later read cascades it
            Err(CompilerError::Diagnostic(diagnostic)) => {
                outcomes.insert(key, Err(Arc::from(diagnostic)));
            }
            // raise every internal failure straight out
            Err(error) => return Err(error),
        }

        Ok(())
    }

    /// Intern one reference type over a lowered pointee in one storage.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::ReferenceKind,
        access: mir::Access,
        storage: mir::Storage,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        insert_reference_type(self.builder.tree_mut(), kind, access, storage, pointee)
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one expression through its coercion, anchoring its emitted MIR at its extent.
    pub(in crate::lower) fn lower_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        self.lower_anchored(expression, |lower| {
            // lower an uncoerced expression as its own value
            let Some(coercion) = lower.coercion(expression) else {
                return lower.lower_expression_value(expression);
            };

            // walk the coercion path from the classified source to its target
            let target = coercion.target();
            let source = lower.lower.instance_type(lower.instance, coercion.source)?;
            let value = lower.coercion_source(expression, source)?;
            let value = lower.lower_adjustments(value, source, &coercion.adjustments)?;

            lower.materialize_coercion_value(value, target)
        })
    }

    /// Lower one expression to the value it produces.
    pub(in crate::lower) fn lower_expression_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // lower by the expression's own syntax
        match self.source().tree().get(expression).clone() {
            // read the value behind a name
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.source);
                let symbol = self.lower.resolved_symbol(node)?;

                self.lower_resolved_value(expression, symbol)
            }

            // materialize a scalar literal
            dir::Expression::Literal(literal) => self.lower_scalar_literal(expression, literal),

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
                    None => Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a non-callable declaration expression".to_string(),
                    }
                    .into()),
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
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a binary operator on union operands".to_string(),
                    }
                    .into());
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
                        } => self.lower_operator_method(left, &call, function),
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
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a unary operator on a union operand".to_string(),
                    }
                    .into());
                };
                match application {
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

            // join the arm values of a match
            dir::Expression::Match { value, arms } => self.lower_match(expression, value, &arms),

            // join the arm values of a ternary
            dir::Expression::If {
                form: dir::IfForm::Ternary,
                condition,
                then_expression,
                else_expression,
            } => self.lower_ternary(expression, &condition, then_expression, else_expression),

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

            // read the receiver binding
            dir::Expression::This => {
                let Some(binding) = self.this else {
                    return Err(CompilerError::Internal {
                        message: "a this outside a method body".to_string(),
                    });
                };

                Ok(self.read_binding(binding))
            }

            // borrow the operand's place
            dir::Expression::BorrowOf { right, .. } => {
                let target = self.lower_type(self.node_type_id(expression)?)?;

                self.lower_borrowed_place(right, target)
            }

            // read a member
            dir::Expression::Member { left, .. } => self.lower_member(expression, left),

            // read a subscript
            dir::Expression::Index { left, index, .. } => {
                self.lower_subscript(expression, left, index)
            }

            // cast to the written type
            dir::Expression::As {
                expression: value, ..
            } => self.lower_as(expression, value),

            // construct a value
            dir::Expression::Call { .. }
                if let Some(resolution) = self.construct_decision(expression) =>
            {
                self.lower_construct(expression, &resolution)
            }

            // construct a class instance
            dir::Expression::New { .. } => {
                let Some(resolution) = self.construct_decision(expression) else {
                    return Err(CompilerError::Internal {
                        message: "a missing construct resolution for one new".to_string(),
                    });
                };

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

            // call and take the value it produces
            dir::Expression::Call { .. } => {
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
                    return Ok(self.builder.constant(mir::Constant::Undefined, result_type));
                }

                // reject a call that produced no value in value position
                Err(CompilerError::Internal {
                    message: "a void call in value position".to_string(),
                })
            }

            // reject every other expression
            other => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("'{}' expressions", other.variant_name()),
            }
            .into()),
        }
    }
}
