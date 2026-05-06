use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use {destack_dir as dir, destack_mir as mir};

use destack_artifact::{DiagnosticAnchor, DirChecked, DirDeclared, WellKnownIntrinsics};
use destack_core::{StringId, StringPool};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerError, CompilerResult, LowerError};

use super::constructor::ConstructorState;
use super::policy::RuntimeCheckConfig;
use crate::lower::{
    BreakContext, DispatchTableGlobal, FunctionEnvironmentLayout, GlobalBinding, InstanceKey,
    InterfaceEntry, LocalBinding, LoopContext, MethodKey, RuntimeStatusLayout, Terminates,
    TypeLowerer,
};

/// Shared, immutable inputs for lowering a single function body.
pub(crate) struct FunctionLoweringContext<'a> {
    /// Identify the function symbol currently being lowered.
    pub(crate) symbol: dir::GlobalSymbolId,
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to compiler helpers for artifact-backed reads.
    pub(crate) compiler: &'a Compiler,
    /// Provider attempt used for artifact-backed reads.
    pub(crate) provider: &'a dyn ProviderContext,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::Tree,
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
    pub(crate) type_lowerer: &'a TypeLowerer<'a>,
    /// MIR return type for this function.
    pub(crate) return_type: mir::LocalNodeId<mir::Type>,
    /// DIR return type for assignment conversion.
    pub(crate) return_type_id: Option<dir::LocalTypeId>,

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
    /// Resolve interface itab globals for interface upcasts.
    pub(crate) itab_globals_by_pair:
        &'a HashMap<(dir::GlobalSymbolId, dir::GlobalSymbolId), DispatchTableGlobal>,
    /// Resolve virtual dispatch slots for method calls.
    pub(crate) virtual_method_slots_by_key: &'a HashMap<(dir::GlobalSymbolId, MethodKey), u32>,
    /// Resolve vtable globals for class allocations.
    pub(crate) vtable_globals_by_symbol: &'a HashMap<dir::GlobalSymbolId, DispatchTableGlobal>,
    /// Synthetic name for call signatures in dispatch tables.
    pub(crate) dispatch_call_name: StringId,
    /// Synthetic name for construct signatures in dispatch tables.
    pub(crate) dispatch_construct_name: StringId,
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

    /// Return a diagnostic anchor for one DIR node.
    pub(crate) fn diagnostic_anchor(&self, node: dir::AnchoredGlobalNodeId) -> DiagnosticAnchor {
        self.context.type_lowerer.diagnostic_anchor(node)
    }

    /// Read one committed declared DIR snapshot for a module when available.
    pub(crate) fn artifact_dir_data_if_present(
        &self,
        module_id: ModuleId,
    ) -> Option<Arc<DirDeclared>> {
        self.context
            .compiler
            .dir_declared(self.context.provider, module_id, self.context.profile)
            .ok()
    }

    /// Read one committed analyzed DIR snapshot for a module.
    pub(crate) fn require_analyzed_dir_data(
        &self,
        module_id: ModuleId,
    ) -> CompilerResult<Arc<DirChecked>> {
        let snapshot = self.context.compiler.dir_checked(
            self.context.provider,
            module_id,
            self.context.profile,
        );

        match snapshot {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => Err(CompilerError::from(error)),
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
    ) -> CompilerResult<Terminates> {
        self.lower_tail_expression(body_id)
    }

    /// Lower a tail expression and emit an implicit return when needed.
    fn lower_tail_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Terminates> {
        if self.state.constructor_state.is_some() {
            return self.lower_statement_expression(expression_id);
        }

        match self.context.dir_tree.get(expression_id) {
            dir::Expression::Block(block_id) => self.lower_tail_block(*block_id),
            dir::Expression::Return { .. }
            | dir::Expression::Labelled { .. }
            | dir::Expression::Let { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::Match { .. } => self.lower_statement_expression(expression_id),
            _ if self.context.return_type == self.context.type_lowerer.ty_void => {
                self.lower_statement_expression(expression_id)
            }
            _ => {
                let (value, _) = self.lower_value_for_target(
                    expression_id,
                    expression_id,
                    self.context.return_type_id,
                    self.context.return_type,
                )?;
                self.state.builder.return_(Some(value));

                Ok(Terminates::Yes)
            }
        }
    }

    /// Lower a block body with tail-expression return semantics.
    fn lower_tail_block(
        &mut self,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> CompilerResult<Terminates> {
        let block = self.context.dir_tree.get(block_id);
        let expressions = block.iter_expressions().collect::<Vec<_>>();
        let Some((tail, statements)) = expressions.split_last() else {
            return Ok(Terminates::No);
        };

        // lower leading statements normally
        for expression_id in statements {
            let terminated = self.lower_statement_expression(*expression_id)?;
            if terminated.is_yes() {
                return Ok(Terminates::Yes);
            }
        }

        self.lower_tail_expression(*tail)
    }

    /// Create an UnsupportedConstruct error for the given expression.
    pub(crate) fn error(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        message: impl Into<String>,
    ) -> LowerError {
        LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
            message: message.into(),
        }
    }

    /// Resolve a symbol value type or return MissingType.
    pub(crate) fn value_type_id_for_symbol_or_error(
        &self,
        node_id: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::LocalTypeId> {
        self.context
            .types
            .get_value_type_id(symbol)
            .ok_or_else(|| LowerError::MissingType {
                anchor: self.diagnostic_anchor(
                    node_id
                        .into_global(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
            })
            .map_err(CompilerError::from)
    }

    /// Resolve a symbol instance type or return MissingType.
    pub(crate) fn instance_type_id_for_symbol_or_error(
        &self,
        node_id: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::LocalTypeId> {
        self.context
            .types
            .get_instance_type_id(symbol)
            .ok_or_else(|| LowerError::MissingType {
                anchor: self.diagnostic_anchor(
                    node_id
                        .into_global(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
            })
            .map_err(CompilerError::from)
    }

    /// Load a string literal value for an internal message.
    pub(crate) fn string_literal_value(
        &mut self,
        literal: &str,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // use content identity for runtime strings
        let literal_id = StringId::for_text(literal);
        self.string_literal_value_for_id(literal_id, None)
    }

    /// Load a string literal value by literal id.
    pub(crate) fn string_literal_value_for_id(
        &mut self,
        literal_id: StringId,
        anchor: Option<dir::AnchoredGlobalNodeId>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the literal global
        let global = self
            .context
            .string_literal_globals
            .get(&literal_id)
            .copied()
            .ok_or_else(|| LowerError::Internal {
                anchor: (self.context.module_id).into(),
                module: self.context.module_id,
                message: format!("missing string literal global for {literal_id:?}"),
            })
            .map_err(CompilerError::from)?;

        // load the string value from its static global
        let value = self.state.builder.load_global(global);

        // resolve the string type for the literal
        let ty = self.context.type_lowerer.string_type().ok_or_else(|| {
            let message = "missing well known String layout (load core)".to_string();
            match anchor {
                Some(anchor) => LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(anchor),
                    message,
                },
                None => LowerError::Internal {
                    anchor: (self.context.module_id).into(),
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
    ) -> CompilerResult<mir::Value> {
        // resolve the vtable global for the class
        let vtable_global = self
            .context
            .vtable_globals_by_symbol
            .get(&class_symbol)
            .copied()
            .ok_or_else(|| LowerError::Internal {
                anchor: (self.context.module_id).into(),
                module: self.context.module_id,
                message: format!("missing vtable global for class {class_symbol:?}"),
            })
            .map_err(CompilerError::from)?;

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
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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

            dir::Expression::UnresolvedPath { path, .. } => {
                let target_symbol = self.resolve_local_path_symbol(expression_id, path)?;
                self.lower_reference_expression(expression_id, target_symbol)
            }

            dir::Expression::ScalarLiteral { value } => {
                self.lower_scalar_literal(expression_id, value)
            }

            dir::Expression::As {
                operator,
                source: _,
                expression,
                target_type: _,
            } => {
                let operator = match operator {
                    Some(operator) => *operator,
                    None => self.classify_explicit_cast_operator(expression_id, *expression)?,
                };

                self.lower_cast_expression(expression_id, operator, *expression)
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

            dir::Expression::Member { left, name }
            | dir::Expression::PrivateMember { left, name } => {
                let Some(name) = *name else {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "missing member name".to_string(),
                    }
                    .into());
                };
                self.lower_member_expression(expression_id, *left, name)
            }

            dir::Expression::Index { left, right } => {
                let index_expr = right
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "missing index expression".to_string(),
                    })
                    .map_err(CompilerError::from)?;
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
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported if-let condition".to_string(),
                    }
                    .into())
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
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported declaration value".to_string(),
                    }
                    .into());
                };

                let symbol = declaration.symbol.into_global(self.context.module_id);
                self.lower_reference_expression(expression_id, symbol)
            }

            dir::Expression::This => self.lower_this_expression(expression_id),

            _ => Err(CompilerError::from(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: format!("unsupported value expression '{}'", expression.kind_name()),
            }))?,
        }
    }

    /// Lower a variable reference expression.
    pub(crate) fn lower_reference_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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

        let global_binding = self.global_binding_for_symbol(expression_id, target_symbol)?;
        if global_binding.mutability == mir::Mutability::Mutable {
            let addr_type = self.state.builder.type_reference(
                mir::ReferenceKind::Raw,
                global_binding.ty,
                global_binding.mutability,
                global_binding.space.clone(),
                false,
            );
            let addr = self
                .state
                .builder
                .global_addr(global_binding.global, addr_type);
            let value = self.state.builder.load(addr, global_binding.ty);
            Ok((value, global_binding.ty))
        } else {
            let value = self.state.builder.load_global(global_binding.global);
            Ok((value, global_binding.ty))
        }
    }

    /// Resolve an unresolved local path from its expression scope.
    pub(crate) fn resolve_local_path_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let Some(name) = path.last_segment() else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "empty path".to_string(),
            }
            .into());
        };
        let key = dir::StaticKey::Name(name);
        let (mut scope_id, mut mark) = self.context.dir_tree.get_scope(expression_id);

        loop {
            let scope = self.context.symbols.get_scope_by_id(scope_id);
            if let Some(symbol_id) = self
                .context
                .symbols
                .find_active_symbol_up_to(scope, key, mark)
            {
                return Ok(symbol_id.into_global(self.context.module_id));
            }

            let Some((parent_scope_id, _)) = scope.parent else {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unresolved path".to_string(),
                }
                .into());
            };

            scope_id = parent_scope_id;
            mark = dir::LocalScopeMark::end();
        }
    }

    /// Lower a function symbol reference to a closure value.
    fn lower_function_value_for_symbol(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the closure value type
        let closure_type = self.lower_type_for_expression(expression_id)?;

        // resolve the target function and environment type
        let target_function = self
            .function_for_symbol(target_symbol)
            .ok_or_else(|| self.error(expression_id, "missing function binding"))
            .map_err(CompilerError::from)?;

        // materialize the function environment
        let (env_value, env_value_type) =
            self.build_function_environment_for_symbol(expression_id, target_symbol)?;

        // register the function environment type on the callee
        self.set_function_environment(expression_id, target_function, env_value_type)?;
        let closure_value =
            self.state
                .builder
                .callable_bind(target_function, closure_type, env_value);

        Ok((closure_value, closure_type))
    }

    /// Lower a cast expression.
    fn lower_cast_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::CastOperator,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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

        // reject implicit heap address exposure before selecting the mir cast
        self.reject_implicit_heap_address_cast(expression_id, operator, source_type, target_type)?;

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
        left: dir::LocalNodeId<dir::AssignPattern>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let left = self.assign_pattern_target_expression(left)?;

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
            dir::Expression::UnresolvedPath { path, .. } => {
                let target_symbol = self.resolve_local_path_symbol(left, path)?;
                if let Some(field) = self.capture_field_for_symbol(target_symbol) {
                    self.store_captured_binding(expression_id, &field, value)?;
                    return Ok((value, value_type));
                }

                let binding = self.local_binding_for_symbol(left, target_symbol)?;
                self.set_binding_value(binding, value);
            }
            dir::Expression::Member {
                left: receiver_id,
                name,
            }
            | dir::Expression::PrivateMember {
                left: receiver_id,
                name,
            } => {
                // only support assignments to this fields in constructors
                let receiver = self.context.dir_tree.get(*receiver_id);
                if !matches!(receiver, dir::Expression::This) {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "unsupported assignment target".to_string(),
                    }
                    .into());
                }

                // require a constructor context for this assignment
                if self.state.constructor_state.is_none() {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "assignment to this fields is only supported in constructors"
                            .to_string(),
                    }
                    .into());
                }

                // resolve the this binding
                let binding = self.state.bindings.this_binding.ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
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
                            anchor: self.diagnostic_anchor(
                                expression_id
                                    .into_global_any(self.context.module_id)
                                    .into_anchored(Some(self.context.profile)),
                            ),
                            message: "missing member name".to_string(),
                        })
                        .map_err(CompilerError::from)?,
                        self.context.strings,
                        self.state.builder.tree(),
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "field not found in aggregate type".to_string(),
                    })
                    .map_err(CompilerError::from)?;

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
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "unsupported assignment target".to_string(),
                }
                .into());
            }
        }

        Ok((value, value_type))
    }

    /// Return one simple expression target from one assignment pattern.
    fn assign_pattern_target_expression(
        &self,
        mut assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        loop {
            let assign_pattern = self.context.dir_tree.get(assign_pattern_id);

            // keep direct expression targets
            if let dir::AssignPattern::Expression { value } = assign_pattern {
                return Ok(*value);
            }

            // unwrap defaulted targets before checking the base
            if let dir::AssignPattern::Assign { pattern, .. } = assign_pattern {
                assign_pattern_id = *pattern;
                continue;
            }

            // MIR lowering does not yet support destructuring assignment
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    assign_pattern_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "destructuring assignment is not supported in MIR lowering".to_string(),
            }
            .into());
        }
    }

    /// Lower a unary expression.
    fn lower_unary_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
            message: "this reference outside of method context".to_string(),
        }
        .into())
    }
}
