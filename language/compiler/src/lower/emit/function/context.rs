use std::collections::{HashMap, HashSet};

use destack_base::StringPool;
use destack_dir::{AnchoredGlobalNodeId, Expression, GlobalSymbolId, IfCondition, LocalNodeId};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::constructor::ConstructorState;
use super::policy::RuntimeCheckConfig;
use crate::lower::emit::{BreakContext, LoopContext, Terminates};
use crate::lower::item::{GlobalBinding, LocalBinding, LocalStorage};
use crate::lower::table::interface::InterfaceSlot;
use crate::lower::table::{VirtualMethodKey, VtableGlobal};
use crate::lower::r#type::TypeLowerer;

/// Shared, immutable inputs for lowering a single function body.
pub(crate) struct FunctionEnv<'a> {
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to program metadata for remote symbol lookup.
    pub(crate) program: &'a Program,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Provide access to the program string pool for name resolution.
    pub(crate) strings: &'a StringPool,
    /// Resolve direct calls for known function symbols.
    pub(crate) functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Resolve MIR signature types for known functions.
    pub(crate) function_signature_types:
        &'a HashMap<mir::LocalNodeId<mir::Function>, mir::LocalNodeId<mir::Type>>,
    /// Resolve globals by symbol for module-level variable references.
    pub(crate) globals_by_symbol: &'a HashMap<GlobalSymbolId, GlobalBinding>,
    /// Resolve interface dispatch slots for call lowering.
    pub(crate) interface_slots_by_symbol: &'a HashMap<GlobalSymbolId, Vec<InterfaceSlot>>,
    /// Resolve interface itab ids for interface upcasts.
    pub(crate) interface_itab_ids:
        &'a HashMap<(GlobalSymbolId, GlobalSymbolId), mir::DispatchTableId>,
    /// Resolve virtual dispatch slot ids for method calls.
    pub(crate) virtual_method_slots_by_key: &'a HashMap<(GlobalSymbolId, VirtualMethodKey), u32>,
    /// Resolve vtable globals for class allocations.
    pub(crate) vtable_globals_by_symbol: &'a HashMap<GlobalSymbolId, VtableGlobal>,
    /// Synthetic name for call signatures in dispatch tables.
    pub(crate) dispatch_call_name: destack_base::StringId,
    /// Synthetic name for construct signatures in dispatch tables.
    pub(crate) dispatch_construct_name: destack_base::StringId,
    /// Runtime check configuration for this target.
    pub(crate) runtime_checks: RuntimeCheckConfig,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: &'a TypeLowerer,
}

/// Mutable bindings state while lowering a single function.
pub(crate) struct FunctionBindings {
    /// Track locals by symbol for variable resolution.
    pub(crate) locals_by_symbol: HashMap<GlobalSymbolId, LocalBinding>,
    /// Binding for `this` in method bodies.
    pub(crate) this_binding: Option<LocalBinding>,
    /// Symbols that require addressable locals.
    pub(crate) address_taken_locals: HashSet<GlobalSymbolId>,
    /// Whether `this` is address taken in the function.
    pub(crate) takes_this_address: bool,
}

/// Address taken bindings for a function body.
pub(crate) struct AddressTakenBindings {
    /// Symbols that require addressable locals.
    pub(crate) locals: HashSet<GlobalSymbolId>,
    /// Whether `this` is address taken in the function.
    pub(crate) takes_this: bool,
}

impl AddressTakenBindings {
    /// Create empty address-taken bindings.
    pub(crate) fn empty() -> Self {
        Self {
            locals: HashSet::new(),
            takes_this: false,
        }
    }
}

/// Mutable control flow state while lowering a single function.
pub(crate) struct FunctionControlFlow {
    /// Track loop contexts by symbol for labeled break/continue.
    pub(crate) loops_by_symbol: HashMap<GlobalSymbolId, LoopContext>,
    /// Track labelled block contexts by symbol for labeled breaks.
    pub(crate) labels_by_symbol: HashMap<GlobalSymbolId, BreakContext>,
    /// Track loop nesting for unlabeled break/continue.
    pub(crate) loop_stack: Vec<LoopContext>,
    /// Track breakable contexts for unlabeled breaks.
    pub(crate) break_stack: Vec<BreakContext>,
}

/// Mutable state for lowering a single function body into MIR.
pub(crate) struct FunctionState<'a> {
    /// Emit MIR into the current function builder.
    pub(crate) builder: mir::FunctionBuilder<'a>,
    /// Mutable binding state for locals and `this`.
    pub(crate) bindings: FunctionBindings,
    /// Mutable control flow state for loops and labels.
    pub(crate) control: FunctionControlFlow,
    /// Track constructor state when lowering a constructor body.
    pub(crate) constructor_state: Option<ConstructorState>,
}

impl<'a> FunctionState<'a> {
    /// Create a new function state with an initialized builder.
    pub(crate) fn new(
        builder: mir::FunctionBuilder<'a>,
        address_taken: AddressTakenBindings,
    ) -> Self {
        Self {
            builder,
            bindings: FunctionBindings {
                locals_by_symbol: HashMap::new(),
                this_binding: None,
                address_taken_locals: address_taken.locals,
                takes_this_address: address_taken.takes_this,
            },
            control: FunctionControlFlow {
                loops_by_symbol: HashMap::new(),
                labels_by_symbol: HashMap::new(),
                loop_stack: Vec::new(),
                break_stack: Vec::new(),
            },
            constructor_state: None,
        }
    }
}

/// Mutable context for lowering a single function body into MIR.
///
/// Owns the function builder and local state (variables, loops).
/// All expression and statement lowering methods are implemented on this type.
pub(crate) struct FunctionContext<'a> {
    /// Shared inputs for function lowering.
    pub(crate) env: FunctionEnv<'a>,
    /// Mutable state for function lowering.
    pub(crate) state: FunctionState<'a>,
}

impl<'a> FunctionContext<'a> {
    /// Create a new function context with the given builder.
    pub(crate) fn new(env: FunctionEnv<'a>, state: FunctionState<'a>) -> Self {
        Self { env, state }
    }

    /// Lower a function body to MIR and return whether it terminates.
    pub(crate) fn lower_body(
        &mut self,
        body_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<Terminates> {
        self.lower_statement_expression(body_id)
    }

    /// Check whether a receiver expression resolves to a namespace symbol.
    pub(crate) fn receiver_is_namespace_reference(
        &self,
        receiver_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let symbol = match self.env.dir_tree.get(receiver_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            _ => return false,
        };

        let module = self.env.program.modules.get(symbol.module_id);
        let module = module.read();
        let symbols = module.dir(self.env.profile).symbols.read();
        symbols.get_symbol(symbol.local_id).kind == dir::SymbolKind::Namespace
    }

    /// Create an UnsupportedConstruct error for the given expression.
    #[allow(dead_code)]
    pub(crate) fn error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        message: impl Into<String>,
    ) -> LowerError {
        LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
            message: message.into(),
        }
    }

    /// Create a MissingType error for the given expression.
    #[allow(dead_code)]
    pub(crate) fn missing_type_error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerError {
        LowerError::MissingType {
            node: expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
        }
    }

    /// Check whether a symbol requires an addressable local.
    pub(crate) fn symbol_needs_addressable_local(&self, symbol: GlobalSymbolId) -> bool {
        self.state.bindings.address_taken_locals.contains(&symbol)
    }

    /// Check whether `this` requires an addressable local.
    pub(crate) fn this_needs_addressable_local(&self) -> bool {
        self.state.bindings.takes_this_address
    }

    /// Load a local binding value.
    pub(crate) fn binding_value(&mut self, binding: LocalBinding) -> mir::Value {
        match binding.storage {
            LocalStorage::Variable(variable) => self.state.builder.use_variable(variable),
            LocalStorage::Local(local) => self.state.builder.local_get(local),
        }
    }

    /// Store a value into a local binding.
    pub(crate) fn set_binding_value(&mut self, binding: LocalBinding, value: mir::Value) {
        match binding.storage {
            LocalStorage::Variable(variable) => {
                self.state.builder.define_variable(variable, value);
            }
            LocalStorage::Local(local) => {
                self.state.builder.local_set(local, value);
            }
        }
    }

    /// Load the vtable pointer for a class symbol as a raw reference.
    pub(crate) fn vtable_pointer_for_class(
        &mut self,
        class_symbol: GlobalSymbolId,
        _node: AnchoredGlobalNodeId,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<mir::Value> {
        // resolve the vtable global for the class
        let vtable_global = self
            .env
            .vtable_globals_by_symbol
            .get(&class_symbol)
            .copied()
            .ok_or_else(|| LowerError::Internal {
                module: self.env.module_id,
                message: format!("missing vtable global for class {class_symbol:?}"),
            })?;

        // load the address of the vtable global
        let address = self
            .state
            .builder
            .global_addr(vtable_global.global_id, vtable_global.address_type);

        // bitcast to the desired pointer type when needed
        let value = if vtable_global.address_type == result_type {
            // reuse the existing address type
            address
        } else {
            // cast to the desired pointer type
            self.state
                .builder
                .cast(mir::CastOperator::Bitcast, address, result_type)
        };

        Ok(value)
    }

    /// Lower a value expression to its result value and type.
    pub(crate) fn lower_value_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let expression = self.env.dir_tree.get(expression_id);
        match expression {
            Expression::Parenthesized { expression } => self.lower_value_expression(*expression),

            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                self.lower_reference_expression(expression_id, *target_symbol)
            }

            Expression::ScalarLiteral { value } => self.lower_scalar_literal(expression_id, value),

            Expression::Cast {
                operator,
                value,
                target_type: _,
                source: _,
            } => self.lower_cast_expression(expression_id, *operator, *value),

            Expression::Binary {
                left,
                operator,
                right,
            } => self.lower_binary_expression(expression_id, *left, *operator, *right),

            Expression::Assign { left, right } => {
                self.lower_assign_expression(expression_id, *left, *right)
            }

            Expression::Call {
                left,
                dynamic_arguments,
                static_arguments,
            } => {
                self.lower_call_expression(expression_id, left, dynamic_arguments, static_arguments)
            }

            Expression::New {
                static_arguments,
                dynamic_arguments,
                ..
            } => self.lower_new_expression(expression_id, static_arguments, dynamic_arguments),

            Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression_id, elements)
            }

            Expression::ArrayExpression { elements } => {
                self.lower_array_expression(expression_id, elements)
            }

            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "static arguments on member access are not supported".to_string(),
                    })?;
                }
                self.lower_member_expression(expression_id, *left, *name)
            }

            Expression::Index { left, right } => {
                let index_expr = right.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "missing index expression".to_string(),
                })?;
                self.lower_index_expression(expression_id, *left, index_expr)
            }

            Expression::Unary { operator, right } => {
                self.lower_unary_expression(expression_id, *operator, *right)
            }

            Expression::ReferenceOf {
                mutability, right, ..
            } => self.lower_reference_of_expression(expression_id, *mutability, *right),

            Expression::ValueOf {
                mutability, right, ..
            } => self.lower_value_of_expression(expression_id, *mutability, *right),

            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => match condition {
                IfCondition::Expression { condition } => self.lower_conditional_expression(
                    expression_id,
                    *condition,
                    *then_expression,
                    *else_expression,
                ),
                IfCondition::Let { .. } => {
                    // if let should be elaborated before lowering
                    Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "unsupported if-let condition".to_string(),
                    })
                }
            },

            Expression::TaggedObjectExpression { ty, properties } => {
                self.lower_tagged_object_expression(expression_id, *ty, properties)
            }

            Expression::This => self.lower_this_expression(expression_id),

            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: format!("unsupported value expression '{}'", expression.kind_name()),
            })?,
        }
    }

    /// Lower a variable reference expression.
    pub(crate) fn lower_reference_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // first check locals
        if let Some(binding) = self
            .state
            .bindings
            .locals_by_symbol
            .get(&target_symbol)
            .copied()
        {
            let value = self.binding_value(binding);
            return Ok((value, binding.ty));
        }

        // use global_const for immutable, global_addr + load for mutable
        let global_binding = self.global_binding_for_symbol(expression_id, target_symbol)?;
        if global_binding.mutability == mir::Mutability::Mutable {
            let addr_type = self.state.builder.type_reference(
                mir::ReferenceKind::Raw,
                global_binding.ty,
                global_binding.mutability,
                mir::AddressSpace::Global,
                false,
            );
            let addr = self
                .state
                .builder
                .global_addr(global_binding.global, addr_type);
            let value = self.state.builder.load(addr, global_binding.ty);
            Ok((value, global_binding.ty))
        } else {
            let value = self.state.builder.global_const(global_binding.global);
            Ok((value, global_binding.ty))
        }
    }

    /// Lower a cast expression.
    fn lower_cast_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::CastOperator,
        value_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        match operator {
            dir::CastOperator::InstanceUpcast => {
                return self.lower_instance_upcast(expression_id, value_id);
            }
            dir::CastOperator::InstanceDowncast => {
                return self.lower_instance_downcast(expression_id, value_id);
            }
            dir::CastOperator::UnionUpcast => {
                return self.lower_union_upcast(expression_id, value_id);
            }
            dir::CastOperator::UnionDowncast => {
                return self.lower_union_downcast(expression_id, value_id);
            }
            dir::CastOperator::ObjectUpcast
            | dir::CastOperator::ObjectDowncast
            | dir::CastOperator::AnyUpcast
            | dir::CastOperator::AnyDowncast
            | dir::CastOperator::UnknownUpcast
            | dir::CastOperator::UnknownDowncast => {
                let (value, _) = self.lower_value_expression(value_id)?;
                let target_type = self.lower_type_for_expression(expression_id)?;
                return Ok((value, target_type));
            }
            dir::CastOperator::NullableUpcast => {
                return self.lower_nullable_upcast(expression_id, value_id);
            }
            dir::CastOperator::NullableDowncast => {
                return self.lower_nullable_downcast(expression_id, value_id);
            }
            _ => {}
        }

        // lower the cast input first
        let (value, _) = self.lower_value_expression(value_id)?;

        // resolve the target type for the cast
        let target_type = self.lower_type_for_expression(expression_id)?;

        // pick the mir cast operator
        let mir_operator = self.lower_cast_operator(expression_id, operator, value_id)?;

        // emit the cast when needed
        let value = if let Some(mir_operator) = mir_operator {
            self.state.builder.cast(mir_operator, value, target_type)
        } else {
            value
        };

        Ok((value, target_type))
    }

    /// Lower an assignment expression.
    fn lower_assign_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the assigned value
        let (value, value_type) = self.lower_value_expression(right)?;

        // update the assignment target
        match self.env.dir_tree.get(left) {
            Expression::LocalReference { target_symbol, .. } => {
                // resolve the target binding
                let binding = self.local_binding_for_symbol(left, *target_symbol)?;

                // update the variable binding
                self.set_binding_value(binding, value);
            }
            Expression::Member {
                left: receiver_id,
                name,
                static_arguments,
            } => {
                // reject static arguments on assignment
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "static arguments on member assignment not supported".to_string(),
                    });
                }

                // only support assignments to this fields in constructors
                let receiver = self.env.dir_tree.get(*receiver_id);
                if !matches!(receiver, Expression::This) {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "unsupported assignment target".to_string(),
                    });
                }

                // require a constructor context for this assignment
                if self.state.constructor_state.is_none() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "assignment to this fields is only supported in constructors"
                            .to_string(),
                    });
                }

                // resolve the this binding
                let binding = self.state.bindings.this_binding.ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "this reference outside of constructor context".to_string(),
                    }
                })?;

                // handle setter access using resolution
                if let Some(target_symbol) = self.resolved_member_symbol(left)
                    && matches!(
                        self.member_mode_for_symbol(target_symbol),
                        Some(dir::FunctionMode::Setter)
                    )
                {
                    self.lower_setter_call(expression_id, *receiver_id, target_symbol, value)?;
                    return Ok((value, value_type));
                }

                // resolve the field index for this assignment
                let field_index = self
                    .env
                    .type_lowerer
                    .field_index_for_type(
                        binding.ty,
                        *name,
                        self.env.strings,
                        self.state.builder.tree(),
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "field not found in aggregate type".to_string(),
                    })?;

                // update the aggregate value
                let current = self.binding_value(binding);
                let reference_pointee = match self.state.builder.tree().get(binding.ty) {
                    mir::Type::Reference { pointee, .. } => Some(*pointee),
                    _ => None,
                };
                if let Some(pointee) = reference_pointee {
                    self.emit_null_check(expression_id, current, binding.ty)?;
                    let aggregate = self.state.builder.load(current, pointee);
                    let updated =
                        self.state
                            .builder
                            .field_set(aggregate, field_index as u32, value);
                    self.state.builder.store(current, updated);
                } else {
                    let updated = self
                        .state
                        .builder
                        .field_set(current, field_index as u32, value);
                    self.set_binding_value(binding, updated);
                };
                self.mark_constructor_field_initialized(field_index as u32);
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "unsupported assignment target".to_string(),
                })?;
            }
        }

        Ok((value, value_type))
    }

    /// Lower a unary expression.
    fn lower_unary_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::UnaryOperator,
        right: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower operand and emit unary operation
        let (operand_value, operand_type) = self.lower_value_expression(right)?;
        let op = self.lower_unary_operator(expression_id, operator, right)?;
        let value = self.state.builder.unary_op(op, operand_value);

        // logical NOT produces bool, other unary ops preserve type
        let ty = if matches!(operator, dir::UnaryOperator::Not) {
            self.env.type_lowerer.ty_bool
        } else {
            operand_type
        };

        Ok((value, ty))
    }

    /// Lower a `this` expression.
    fn lower_this_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // get this binding set by lower_method
        let binding =
            self.state
                .bindings
                .this_binding
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "this reference outside of method context".to_string(),
                })?;

        let value = self.binding_value(binding);
        Ok((value, binding.ty))
    }
}
