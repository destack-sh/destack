use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use {destack_dir as dir, destack_mir as mir};

use destack_artifact::{DirAnalyzed, DirDeclared, WellKnownIntrinsics};
use destack_core::{StringId, StringPool};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Repository, Revision};

use crate::{Compiler, LowerError, LowerResult, RequirementError};

use super::constructor::ConstructorState;
use super::policy::RuntimeCheckConfig;
use crate::lower::{
    BreakContext, FunctionEnvironmentLayout, GlobalBinding, InstanceKey, InterfaceEntry,
    LocalBinding, LoopContext, MethodKey, RuntimeStatusLayout, Terminates, TypeLowerer,
    VtableGlobal,
};

/// Shared, immutable inputs for lowering a single function body.
pub(crate) struct FunctionLoweringContext<'a> {
    /// Identify the function symbol currently being lowered.
    pub(crate) symbol: dir::GlobalSymbolId,
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Pinned revision for cross-module reads.
    pub(crate) revision: Revision,
    /// Provide access to program metadata for remote symbol lookup.
    pub(crate) program: &'a Repository,
    /// Provide access to compiler helpers for artifact-backed reads.
    pub(crate) compiler: &'a Compiler,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Provide access to capture metadata for closures.
    pub(crate) captures: &'a dir::CaptureTable,
    /// Provide access to the program string pool for name resolution.
    pub(crate) strings: &'a StringPool,
    /// Well-known intrinsic bindings for this profile.
    pub(crate) well_known_intrinsics: Option<&'a WellKnownIntrinsics>,
    /// Runtime check configuration for this target.
    pub(crate) checks: RuntimeCheckConfig,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: &'a TypeLowerer,
    /// MIR return type for this function.
    pub(crate) return_type: mir::LocalNodeId<mir::Type>,

    /// Resolve direct calls for known function instances.
    pub(crate) functions_by_instance: &'a HashMap<InstanceKey, mir::LocalNodeId<mir::Function>>,
    /// Resolve MIR signature types for known functions.
    pub(crate) function_signature_types:
        &'a HashMap<mir::LocalNodeId<mir::Function>, mir::LocalNodeId<mir::Type>>,
    /// Resolve binding symbols for ABI lowering.
    pub(crate) binding_symbols: &'a HashSet<dir::GlobalSymbolId>,
    /// Enable ABI lowering for bindings.
    pub(crate) binding_abi_lowering: bool,
    /// Cached runtime status layout for binding ABI calls.
    pub(crate) runtime_status_layout: Option<RuntimeStatusLayout>,
    /// Cached runtime error take binding for ABI calls.
    pub(crate) take_platform_error_function: Option<mir::LocalNodeId<mir::Function>>,

    /// Resolve globals by symbol for module-level variable references.
    pub(crate) globals_by_symbol: &'a HashMap<dir::GlobalSymbolId, GlobalBinding>,
    /// Resolve string literal globals by literal content.
    pub(crate) string_literal_globals: &'a HashMap<StringId, mir::LocalNodeId<mir::Global>>,

    /// Resolve interface dispatch slots for call lowering.
    pub(crate) interface_slots_by_symbol: &'a HashMap<dir::GlobalSymbolId, Vec<InterfaceEntry>>,
    /// Resolve interface itab ids for interface upcasts.
    pub(crate) interface_itab_ids:
        &'a HashMap<(dir::GlobalSymbolId, dir::GlobalSymbolId), mir::ItabId>,
    /// Resolve virtual dispatch slot ids for method calls.
    pub(crate) virtual_method_slots_by_key: &'a HashMap<(dir::GlobalSymbolId, MethodKey), u32>,
    /// Resolve vtable globals for class allocations.
    pub(crate) vtable_globals_by_symbol: &'a HashMap<dir::GlobalSymbolId, VtableGlobal>,
    /// Synthetic name for call signatures in dispatch tables.
    pub(crate) dispatch_call_name: destack_core::StringId,
    /// Synthetic name for construct signatures in dispatch tables.
    pub(crate) dispatch_construct_name: destack_core::StringId,
    /// Resolve function environment layouts by function symbol.
    pub(crate) function_environment_layouts:
        &'a HashMap<dir::GlobalSymbolId, FunctionEnvironmentLayout>,
    /// Fallback environment pointer type for non-capturing closures.
    pub(crate) empty_function_environment_pointer_type: mir::LocalNodeId<mir::Type>,
}

/// Mutable bindings state while lowering a single function.
pub(crate) struct FunctionBindings {
    /// Track locals by symbol for variable resolution.
    pub(crate) locals_by_symbol: HashMap<dir::GlobalSymbolId, LocalBinding>,
    /// Symbol id for the implicit `this` binding.
    pub(crate) this_symbol: Option<dir::GlobalSymbolId>,
    /// Binding for `this` in method bodies.
    pub(crate) this_binding: Option<LocalBinding>,
    /// Symbols that require boxed capture storage.
    pub(crate) reference_locals: HashSet<dir::GlobalSymbolId>,
    /// Symbols that require addressable locals.
    pub(crate) address_taken_locals: HashSet<dir::GlobalSymbolId>,
    /// Whether `this` is address taken in the function.
    pub(crate) takes_this_address: bool,
    /// Function environment value for captured bindings.
    pub(crate) environment: Option<mir::Value>,
}

/// Address taken bindings for a function body.
pub(crate) struct AddressTakenBindings {
    /// Symbols that require addressable locals.
    pub(crate) locals: HashSet<dir::GlobalSymbolId>,
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
    pub(crate) loops_by_symbol: HashMap<dir::GlobalSymbolId, LoopContext>,
    /// Track labelled block contexts by symbol for labeled breaks.
    pub(crate) labels_by_symbol: HashMap<dir::GlobalSymbolId, BreakContext>,
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
                this_symbol: None,
                this_binding: None,
                reference_locals: HashSet::new(),
                address_taken_locals: address_taken.locals,
                takes_this_address: address_taken.takes_this,
                environment: None,
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
pub(crate) struct FunctionLowerer<'a> {
    /// Shared inputs for function lowering.
    pub(crate) context: FunctionLoweringContext<'a>,
    /// Mutable state for function lowering.
    pub(crate) state: FunctionState<'a>,
}

#[allow(dead_code)]
impl<'a> FunctionLowerer<'a> {
    /// Create a new function lowerer with the given lowering context.
    pub(crate) fn new(context: FunctionLoweringContext<'a>, state: FunctionState<'a>) -> Self {
        Self { context, state }
    }

    /// Read one committed declared DIR snapshot for a module when available.
    pub(crate) fn artifact_dir_data_if_present(
        &self,
        module_id: ModuleId,
    ) -> Option<Arc<DirDeclared>> {
        self.context
            .compiler
            .dir_declared(module_id, self.context.profile)
    }

    /// Read one committed analyzed DIR snapshot for a module.
    pub(crate) fn require_analyzed_dir_data(
        &self,
        module_id: ModuleId,
    ) -> LowerResult<Arc<DirAnalyzed>> {
        let snapshot = self.context.compiler.require_artifact_dir_analyzed(
            self.context.revision,
            module_id,
            self.context.profile,
        );

        match snapshot {
            Ok(snapshot) => Ok(snapshot),
            Err(RequirementError::NotReady { requirement }) => {
                Err(LowerError::Yield { requirement })
            }
            Err(RequirementError::Failed { requirement }) => {
                Err(LowerError::UnsatisfiedRequirement { requirement })
            }
        }
    }

    /// Return the lowered function id for a symbol-backed instance.
    pub(crate) fn function_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        let instance = InstanceKey::symbol(symbol);
        self.context.functions_by_instance.get(&instance).copied()
    }

    /// Lower a function body to MIR and return whether it terminates.
    pub(crate) fn lower_body(
        &mut self,
        body_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<Terminates> {
        self.lower_statement_expression(body_id)
    }

    /// Create an UnsupportedConstruct error for the given expression.
    pub(crate) fn error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        message: impl Into<String>,
    ) -> LowerError {
        LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.context.module_id)
                .into_anchored(Some(self.context.profile)),
            message: message.into(),
        }
    }

    /// Resolve a symbol value type or return MissingType.
    pub(crate) fn value_type_id_for_symbol_or_error(
        &self,
        node_id: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<dir::LocalTypeId> {
        self.context
            .types
            .get_value_type_id(symbol)
            .ok_or_else(|| LowerError::MissingType {
                node: node_id
                    .into_global(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            })
    }

    /// Resolve a symbol instance type or return MissingType.
    pub(crate) fn instance_type_id_for_symbol_or_error(
        &self,
        node_id: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<dir::LocalTypeId> {
        self.context
            .types
            .get_instance_type_id(symbol)
            .ok_or_else(|| LowerError::MissingType {
                node: node_id
                    .into_global(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            })
    }

    /// Load a string literal value for an internal message.
    pub(crate) fn string_literal_value(
        &mut self,
        literal: &str,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // intern the message so the global lookup is consistent
        let literal_id = self.context.program.strings.intern(literal);
        self.string_literal_value_for_id(literal_id, None)
    }

    /// Load a string literal value by literal id.
    pub(crate) fn string_literal_value_for_id(
        &mut self,
        literal_id: StringId,
        anchor: Option<dir::AnchoredGlobalNodeId>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the literal global
        let global = self
            .context
            .string_literal_globals
            .get(&literal_id)
            .copied()
            .ok_or_else(|| LowerError::Internal {
                module: self.context.module_id,
                message: format!("missing string literal global for {literal_id:?}"),
            })?;

        // materialize the global constant value
        let value = self.state.builder.global_const(global);

        // resolve the string type for the literal
        let ty = self.context.type_lowerer.string_type().ok_or_else(|| {
            let message = "missing well known String layout (load library/native)".to_string();
            match anchor {
                Some(anchor) => LowerError::UnsupportedConstruct {
                    node: anchor,
                    message,
                },
                None => LowerError::Internal {
                    module: self.context.module_id,
                    message,
                },
            }
        })?;

        Ok((value, ty))
    }

    /// Load the vtable pointer for a class symbol as a raw reference.
    pub(crate) fn vtable_pointer_for_class(
        &mut self,
        class_symbol: dir::GlobalSymbolId,
        _node: dir::AnchoredGlobalNodeId,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<mir::Value> {
        // resolve the vtable global for the class
        let vtable_global = self
            .context
            .vtable_globals_by_symbol
            .get(&class_symbol)
            .copied()
            .ok_or_else(|| LowerError::Internal {
                module: self.context.module_id,
                message: format!("missing vtable global for class {class_symbol:?}"),
            })?;

        // load the address of the vtable global
        let address = self
            .state
            .builder
            .global_addr(vtable_global.global_id, vtable_global.address_type);

        // cast.bit to the desired pointer type when needed
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
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let expression = self.context.dir_tree.get(expression_id);
        match expression {
            dir::Expression::Parenthesized { expression } => {
                self.lower_value_expression(*expression)
            }

            dir::Expression::LocalReference { target_symbol, .. }
            | dir::Expression::ModuleReference { target_symbol, .. }
            | dir::Expression::GlobalReference { target_symbol, .. } => {
                self.lower_reference_expression(expression_id, *target_symbol)
            }

            dir::Expression::ScalarLiteral { value } => {
                self.lower_scalar_literal(expression_id, value)
            }

            dir::Expression::As {
                expression,
                target_type: _,
            } => {
                // use the analyzed cast result type, which should already be prelowered
                let (value, _) = self.lower_value_expression(*expression)?;
                let target_type_id = self.type_for_expression_or_error(expression_id)?;
                let target_type_id = self.unwrap_value_type_id(target_type_id);
                let target_type = self
                    .context
                    .type_lowerer
                    .cached_type(target_type_id)
                    .ok_or_else(|| {
                        self.missing_type_error_for_node(
                            expression_id.into_global_any(self.context.module_id),
                        )
                    })?;

                Ok((self.state.builder.bitcast(value, target_type), target_type))
            }

            dir::Expression::Satisfies { expression, .. } => {
                self.lower_value_expression(*expression)
            }

            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.lower_binary_expression(expression_id, *left, *operator, *right),

            dir::Expression::Is { value, target_type } => {
                let target_type_id = self.is_target_type_id(*target_type)?;
                self.lower_runtime_type_guard_expression(expression_id, *value, target_type_id)
            }

            dir::Expression::InstanceOf { value, target } => {
                let target_type_id = self.type_for_expression_or_error(*target)?;
                self.lower_runtime_type_guard_expression(expression_id, *value, target_type_id)
            }

            dir::Expression::Assign { left, right } => {
                self.lower_assign_expression(expression_id, *left, *right)
            }

            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
            } => self.lower_call_expression(expression_id, left, arguments, generic_arguments),

            dir::Expression::New {
                generic_arguments,
                arguments,
                ..
            } => self.lower_new_expression(expression_id, generic_arguments, arguments),

            dir::Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression_id, elements)
            }

            dir::Expression::ArrayExpression { elements } => {
                self.lower_array_expression(expression_id, elements)
            }

            dir::Expression::Member {
                left,
                name,
                generic_arguments,
            }
            | dir::Expression::PrivateMember {
                left,
                name,
                generic_arguments,
            } => {
                if !generic_arguments.is_empty() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "generic arguments on member access are not supported".to_string(),
                    })?;
                }
                let Some(name) = *name else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "missing member name".to_string(),
                    });
                };
                self.lower_member_expression(expression_id, *left, name)
            }

            dir::Expression::Index { left, right } => {
                let index_expr = right.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "missing index expression".to_string(),
                })?;
                self.lower_index_expression(expression_id, *left, index_expr)
            }

            dir::Expression::Unary { operator, right } => {
                self.lower_unary_expression(expression_id, *operator, *right)
            }

            dir::Expression::ReferenceOf {
                mutability, right, ..
            } => self.lower_reference_of_expression(expression_id, *mutability, *right),

            dir::Expression::ValueOf {
                mutability, right, ..
            } => self.lower_value_of_expression(expression_id, *mutability, *right),

            dir::Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => match condition {
                dir::IfCondition::Expression { condition } => self.lower_conditional_expression(
                    expression_id,
                    *condition,
                    *then_expression,
                    *else_expression,
                ),
                dir::IfCondition::Let { .. } => {
                    // if let should be elaborated before lowering
                    Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported if-let condition".to_string(),
                    })
                }
            },

            dir::Expression::TaggedObjectExpression { ty, properties } => {
                self.lower_tagged_object_expression(expression_id, *ty, properties)
            }

            dir::Expression::TaggedScalarExpression { ty, value } => {
                self.lower_tagged_scalar_expression(expression_id, *ty, *value)
            }

            dir::Expression::TaggedTupleExpression { ty, elements } => {
                self.lower_tagged_tuple_expression(expression_id, *ty, elements)
            }

            dir::Expression::Declaration(declaration_id) => {
                // lower function declarations used as values
                let declaration = self.context.dir_tree.get(*declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported declaration value".to_string(),
                    });
                };

                let symbol = declaration.symbol.into_global(self.context.module_id);
                self.lower_reference_expression(expression_id, symbol)
            }

            dir::Expression::This => self.lower_this_expression(expression_id),

            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: format!("unsupported value expression '{}'", expression.kind_name()),
            })?,
        }
    }

    /// Lower a variable reference expression.
    pub(crate) fn lower_reference_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let symbol_data = self.context.symbols.get_symbol(target_symbol.local_id);
        if symbol_data.ty == dir::SymbolType::Function {
            return self.lower_function_value_for_symbol(expression_id, target_symbol);
        }

        // handle captured bindings first
        if let Some(field) = self.capture_field_for_symbol(target_symbol) {
            return self.captured_binding_value(expression_id, &field);
        }

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

    /// Lower a function symbol reference to a closure value.
    fn lower_function_value_for_symbol(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the closure value type
        let closure_type = self.lower_type_for_expression(expression_id)?;

        // resolve the target function and environment type
        let target_function = self
            .function_for_symbol(target_symbol)
            .ok_or_else(|| self.error(expression_id, "missing function binding"))?;

        // materialize the function environment
        let (env_value, env_value_type) =
            self.build_function_environment_for_symbol(expression_id, target_symbol)?;

        // register the function environment type on the callee
        self.set_function_environment(expression_id, target_function, env_value_type)?;
        let closure_value =
            self.state
                .builder
                .function_bind(target_function, closure_type, env_value);

        Ok((closure_value, closure_type))
    }

    /// Lower a cast expression.
    fn lower_cast_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::CastOperator,
        value_id: dir::LocalNodeId<dir::Expression>,
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
        let (value, source_type) = self.lower_value_expression(value_id)?;

        // resolve the target type for the cast
        let target_type = self.lower_type_for_expression(expression_id)?;

        // pick the mir cast operator
        let mir_operator = self.lower_cast_operator(expression_id, operator, value_id)?;

        // emit the cast when needed
        let value = if let Some(mir_operator) = mir_operator {
            self.state.builder.cast(mir_operator, value, target_type)
        } else if source_type != target_type {
            self.state.builder.bitcast(value, target_type)
        } else {
            value
        };

        Ok((value, target_type))
    }

    /// Lower an assignment expression.
    fn lower_assign_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower the assigned value
        let (value, value_type) = self.lower_value_expression(right)?;

        // update the assignment target
        match self.context.dir_tree.get(left) {
            dir::Expression::LocalReference { target_symbol, .. } => {
                if let Some(field) = self.capture_field_for_symbol(*target_symbol) {
                    self.store_captured_binding(expression_id, &field, value)?;
                    return Ok((value, value_type));
                }

                // resolve the target binding
                let binding = self.local_binding_for_symbol(left, *target_symbol)?;

                // update the variable binding
                self.set_binding_value(binding, value);
            }
            dir::Expression::Member {
                left: receiver_id,
                name,
                generic_arguments,
            }
            | dir::Expression::PrivateMember {
                left: receiver_id,
                name,
                generic_arguments,
            } => {
                // reject generic arguments on assignment
                if !generic_arguments.is_empty() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "generic arguments on member assignment not supported".to_string(),
                    });
                }

                // only support assignments to this fields in constructors
                let receiver = self.context.dir_tree.get(*receiver_id);
                if !matches!(receiver, dir::Expression::This) {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "unsupported assignment target".to_string(),
                    });
                }

                // require a constructor context for this assignment
                if self.state.constructor_state.is_none() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "assignment to this fields is only supported in constructors"
                            .to_string(),
                    });
                }

                // resolve the this binding
                let binding = self.state.bindings.this_binding.ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
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
                    .context
                    .type_lowerer
                    .field_index_for_type(
                        binding.ty,
                        name.ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                            message: "missing member name".to_string(),
                        })?,
                        self.context.strings,
                        self.state.builder.tree(),
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                        message: "field not found in aggregate type".to_string(),
                    })?;

                // update the aggregate value
                let current = self.binding_value(binding);
                let reference_pointee = match self.state.builder.tree().get(binding.ty) {
                    mir::Type::Reference { pointee, .. } => pointee.ty(),
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
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported assignment target".to_string(),
                })?;
            }
        }

        Ok((value, value_type))
    }

    /// Lower a unary expression.
    fn lower_unary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower operand and emit unary operation
        let (operand_value, operand_type) = self.lower_value_expression(right)?;
        let op = self.lower_unary_operator(expression_id, operator, right)?;
        let value = self.state.builder.unary_op(op, operand_value);

        // logical NOT produces bool, other unary ops preserve type
        let ty = if matches!(operator, dir::UnaryOperator::Not) {
            self.context.type_lowerer.ty_bool
        } else {
            operand_type
        };

        Ok((value, ty))
    }

    /// Lower a `this` expression.
    pub(crate) fn lower_this_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // prefer the method-local binding when present
        if let Some(binding) = self.state.bindings.this_binding {
            let value = self.binding_value(binding);
            return Ok((value, binding.ty));
        }

        // fall back to captured this bindings
        if let Some(this_symbol) = self.state.bindings.this_symbol
            && let Some(field) = self.capture_field_for_symbol(this_symbol)
        {
            return self.captured_binding_value(expression_id, &field);
        }

        Err(LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.context.module_id)
                .into_anchored(Some(self.context.profile)),
            message: "this reference outside of method context".to_string(),
        })
    }
}
