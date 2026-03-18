use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_ast::StringId;
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use destack_workspace::{CheckFailurePolicy, DirElaborated, Module, ProfileId, TargetId};
use {destack_dir as dir, destack_mir as mir};

use crate::lower::{
    AddressTakenBindings, BuiltinTypeLayouts, FunctionEnv, FunctionLowerer, FunctionState,
    InstanceKey, RuntimeCheckConfig, StructLayout, TypeLowerer, collect_expression_string_literals,
    string_literal_global_name_for_content,
};
use crate::{Compiler, ExecuteError, ExecuteResult, LowerError, ModuleLowerer};

#[allow(dead_code)]
impl Compiler {
    /// Require the elaborated DIR artifact for one module and profile.
    pub(crate) fn require_dir_elaborated_data(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> ExecuteResult<Arc<DirElaborated>> {
        self.require_artifact_dir_elaborated(module_id, profile)
            .map_err(|error| match error {
                crate::BuildRequirementError::NotReady { requirement } => {
                    ExecuteError::Yield { requirement }
                }
                crate::BuildRequirementError::Failed { requirement } => {
                    ExecuteError::UnsatisfiedRequirement { requirement }
                }
            })
    }

    /// Lower a module to MIR for comptime execution.
    pub(crate) fn lower_comptime_module(
        &self,
        module: &Arc<Module>,
        profile: ProfileId,
        target_id: &TargetId,
    ) -> ExecuteResult<(mir::NodeTree, destack_core::StringPool)> {
        // snapshot dir inputs for lowering
        let module = module.as_ref();
        let module_id = module.id;
        let dir = self.require_dir_elaborated_data(module_id, profile)?;

        // FUGU #Performance: avoid cloning whole node dir tree for comptime
        let dir_tree = dir.tree.clone();
        let dir_roots = dir.roots.clone();
        let anchor_node = dir.anchor_node;
        let symbols = dir.symbols.clone();
        let types = dir.types.clone();
        let captures = dir.captures.clone();

        // build the module lowerer
        let pointer_bytes = self
            .pointer_bytes_for_target(module_id, target_id)
            .map_err(|error| ExecuteError::FailedLower {
                module: module_id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;
        let mut lowerer = ModuleLowerer::new(
            self,
            module,
            profile,
            &dir_tree,
            &dir_roots,
            anchor_node,
            &symbols,
            &types,
            &captures,
            target_id,
            pointer_bytes,
        );

        // lower the full module for comptime execution
        lowerer
            .lower_module()
            .map_err(|error| ExecuteError::FailedLower {
                module: module_id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;

        // return the generated MIR and strings
        Ok(lowerer.finish())
    }

    /// Lower a comptime expression into a standalone MIR function.
    pub(crate) fn lower_comptime_expression(
        &self,
        module: &destack_workspace::Module,
        profile: destack_workspace::ProfileId,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> ExecuteResult<(
        mir::NodeTree,
        destack_core::StringPool,
        mir::LocalNodeId<mir::Function>,
    )> {
        let lowerer = ComptimeLowerer::new(self, module, profile)?;
        lowerer.lower_expression(expression_id)
    }

    /// Map lower errors into execute errors.
    fn execute_error_from_lower(
        &self,
        module_id: destack_source::ModuleId,
        error: LowerError,
    ) -> ExecuteError {
        // preserve dependency yields as is
        match error {
            LowerError::Yield { requirement } => ExecuteError::Yield { requirement },
            LowerError::UnsatisfiedRequirement { requirement } => {
                ExecuteError::UnsatisfiedRequirement { requirement }
            }
            error => ExecuteError::FailedLower {
                module: module_id,
                message: format!("{error}"),
                error: Box::new(error),
            },
        }
    }
}

/// Builder for lowering comptime expressions into MIR.
struct ComptimeLowerer<'a> {
    /// The active compiler instance.
    compiler: &'a Compiler,
    /// The module containing the expression.
    module: &'a Module,
    /// The profile used to resolve module context.
    profile: ProfileId,
    /// The elaborated DIR snapshot for the module.
    dir: Arc<DirElaborated>,
    /// DIR tree for the module.
    /// MIR builder for the comptime module.
    builder: mir::ModuleBuilder,
    /// Type lowerer for comptime MIR.
    type_lowerer: TypeLowerer,
    /// Cached string literal globals for this expression.
    string_literal_globals: HashMap<StringId, mir::LocalNodeId<mir::Global>>,
    /// Interned name for the dynamic call dispatcher.
    dispatch_call_name: StringId,
    /// Interned name for the dynamic constructor dispatcher.
    dispatch_construct_name: StringId,
    /// Cached empty closure env type.
    empty_closure_env_type: Option<mir::LocalNodeId<mir::Type>>,
    /// Cached empty closure env pointer type.
    empty_closure_env_pointer_type: Option<mir::LocalNodeId<mir::Type>>,
}

impl<'a> ComptimeLowerer<'a> {
    /// Create a comptime lowerer for a module.
    fn new(compiler: &'a Compiler, module: &'a Module, profile: ProfileId) -> ExecuteResult<Self> {
        // snapshot dir inputs
        let dir = compiler.require_dir_elaborated_data(module.id, profile)?;

        // initialize the mir builder
        let mut builder = mir::ModuleBuilder::unchecked();

        // seed the mir string pool with program strings
        let strings = compiler.program.strings.as_ref().clone().into_immutable();
        builder.strings().copy_from_immutable(&strings);

        // intern comptime dispatch names
        let dispatch_call_name = builder.intern("@call");
        let dispatch_construct_name = builder.intern("@new");

        // prepare the type lowerer
        // TODO #Broken: comptime uses host pointer width until execute is target aware
        let pointer_bytes = std::mem::size_of::<usize>() as u8;
        compiler
            .validate_pointer_bytes(module.id, pointer_bytes)
            .map_err(|error| ExecuteError::FailedLower {
                module: module.id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;
        // resolve vector builtin symbols for SIMD lowering
        let vector_symbol = compiler.get_well_known_symbol_from(
            profile,
            dir::WellKnownSymbol::Vector,
            dir::SymbolSpaceOrder::TypeThenValue,
        );

        let type_lowerer = TypeLowerer::new(
            &mut builder,
            pointer_bytes,
            compiler.program.artifacts.clone(),
            compiler.program.modules.clone(),
            compiler.program.packages.clone(),
            vector_symbol,
        );

        Ok(Self {
            compiler,
            module,
            profile,
            dir,
            builder,
            type_lowerer,
            string_literal_globals: HashMap::new(),
            dispatch_call_name,
            dispatch_construct_name,
            empty_closure_env_type: None,
            empty_closure_env_pointer_type: None,
        })
    }

    /// Lower a comptime expression into a standalone MIR function.
    fn lower_expression(
        mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> ExecuteResult<(
        mir::NodeTree,
        destack_core::StringPool,
        mir::LocalNodeId<mir::Function>,
    )> {
        // compute an anchor for diagnostics
        let anchor = expression_id
            .into_global_any(self.module.id)
            .into_anchored(Some(self.profile));

        // prepare builtin layouts for comptime lowering
        let mut builtin_layouts = BuiltinTypeLayouts::new(
            self.compiler,
            self.profile,
            &mut self.builder,
            &mut self.type_lowerer,
        );
        builtin_layouts
            .string_type_for_builtin(anchor)
            .map_err(|error| {
                self.compiler
                    .execute_error_from_lower(self.module.id, error)
            })?;

        // collect string literals for comptime globals
        self.collect_string_literal_globals(expression_id)?;

        // resolve the return type for the comptime expression
        let return_type = self.lower_return_type(expression_id, anchor)?;

        // create empty lookup tables for standalone expressions
        // TODO #Incomplete: hoist comptime environment into comptime lowering
        let functions_by_instance: HashMap<InstanceKey, mir::LocalNodeId<mir::Function>> =
            HashMap::new();
        let globals_by_symbol = HashMap::new();
        let interface_slots_by_symbol = HashMap::new();
        let interface_itab_ids = HashMap::new();
        let virtual_method_slots_by_key = HashMap::new();
        let vtable_globals_by_symbol = HashMap::new();
        let function_signature_types = HashMap::new();
        let closure_env_layouts = HashMap::new();
        let binding_symbols = HashSet::new();

        // resolve cached intrinsic bindings when available
        let well_known_intrinsics = self
            .compiler
            .program
            .artifacts
            .intrinsic_environment(self.profile)
            .map(|environment| environment.intrinsics.clone());

        // build the empty closure env type
        let empty_closure_env_pointer_type = self.empty_closure_env_pointer_type();

        // build a synthetic function to evaluate the expression
        let function_symbol =
            dir::LocalSymbolId::new_typed(0, dir::SymbolType::Function).into_global(self.module.id);
        let function_builder = self.builder.function("comptime", &[], return_type);

        // create function lowerer
        let env = FunctionEnv {
            module_id: self.module.id,
            profile: self.profile,
            program: &self.compiler.program,
            compiler: self.compiler,
            dir_tree: &self.dir.tree,
            symbols: &self.dir.symbols,
            types: &self.dir.types,
            captures: &self.dir.captures,
            strings: &self.compiler.program.strings,
            well_known_intrinsics: well_known_intrinsics.as_ref(),
            functions_by_instance: &functions_by_instance,
            function_signature_types: &function_signature_types,
            binding_symbols: &binding_symbols,
            binding_abi_lowering: false,
            runtime_status_layout: None,
            take_platform_error_function: None,
            globals_by_symbol: &globals_by_symbol,
            interface_slots_by_symbol: &interface_slots_by_symbol,
            interface_itab_ids: &interface_itab_ids,
            virtual_method_slots_by_key: &virtual_method_slots_by_key,
            vtable_globals_by_symbol: &vtable_globals_by_symbol,
            string_literal_globals: &self.string_literal_globals,
            dispatch_call_name: self.dispatch_call_name,
            dispatch_construct_name: self.dispatch_construct_name,
            checks: RuntimeCheckConfig {
                overflow: false,
                bounds: false,
                null: false,
                division: false,
                shift: false,
                failure: CheckFailurePolicy::Trap,
            },
            type_lowerer: &self.type_lowerer,
            return_type,
            symbol: function_symbol,
            closure_env_layouts: &closure_env_layouts,
            empty_closure_env_pointer_type,
        };
        let state = FunctionState::new(function_builder, AddressTakenBindings::empty());
        let mut function_lowerer = FunctionLowerer::new(env, state);

        // create entry block
        let entry_block = function_lowerer.state.builder.block();
        function_lowerer.state.builder.switch_to_block(entry_block);

        // lower the expression
        let (value, _) = function_lowerer
            .lower_value_expression(expression_id)
            .map_err(|error| ExecuteError::FailedLower {
                module: self.module.id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;

        // return and finish
        function_lowerer.state.builder.return_(Some(value));
        let function_id = function_lowerer.state.builder.finish();

        let (tree, strings) = self.builder.finish_mutable();
        Ok((tree, strings, function_id))
    }

    /// Collect string literal globals for the expression being lowered. (TODO #Cleanup?)
    fn collect_string_literal_globals(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> ExecuteResult<()> {
        // gather literal strings referenced by the expression
        let literals = collect_expression_string_literals(&self.dir.tree, expression_id);
        if literals.is_empty() {
            return Ok(());
        }

        // load the builtin string type for literal globals
        let Some(string_type) = self.type_lowerer.string_type() else {
            return Err(ExecuteError::FailedLower {
                module: self.module.id,
                error: Box::new(LowerError::Internal {
                    module: self.module.id,
                    message: "missing builtin String layout (load lib/native)".to_string(),
                }),
                message: "missing builtin String layout".to_string(),
            });
        };

        // order literals by stable string content
        let mut ordered: Vec<_> = literals.into_iter().collect();
        ordered.sort_by(|left, right| {
            let left_value = self.compiler.program.strings.get(*left);
            let right_value = self.compiler.program.strings.get(*right);
            left_value.as_ref().cmp(right_value.as_ref())
        });

        // emit a global constant for each literal
        for literal_id in ordered {
            let literal = self.compiler.program.strings.get(literal_id);
            let name = string_literal_global_name_for_content(literal.as_ref());
            let global = self.builder.global_constant(
                &name,
                string_type,
                mir::GlobalInitializer::string(literal.as_ref()),
            );
            self.string_literal_globals.insert(literal_id, global);
        }

        Ok(())
    }

    /// Resolve the return type for a comptime expression.
    fn lower_return_type(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        anchor: AnchoredGlobalNodeId,
    ) -> ExecuteResult<mir::LocalNodeId<mir::Type>> {
        // resolve the dir return type
        let return_type_id = dir_type_id_for_expression(
            &self.dir.tree,
            &self.dir.symbols,
            &self.dir.types,
            self.module.id,
            expression_id,
        )
        .ok_or_else(|| ExecuteError::FailedLower {
            module: self.module.id,
            error: Box::new(LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.module.id)
                    .into_anchored(Some(self.profile)),
            }),
            message: "missing type".to_string(),
        })?;

        let return_type = self
            .type_lowerer
            .lower_type(
                &self.dir.types,
                return_type_id,
                self.module.id,
                anchor,
                &mut self.builder,
            )
            .map_err(|error| ExecuteError::FailedLower {
                module: self.module.id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;

        Ok(return_type)
    }

    /// Get or create the empty closure env type for comptime lowering.
    fn empty_closure_env_type(&mut self) -> mir::LocalNodeId<mir::Type> {
        if let Some(env_type) = self.empty_closure_env_type {
            return env_type;
        }

        // build the empty closure env type
        let empty_env_layout = StructLayout::empty();
        let env_type = self
            .type_lowerer
            .create_struct_type(&empty_env_layout, &mut self.builder);
        self.type_lowerer.set_layout(env_type, empty_env_layout);
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            false,
        );
        self.empty_closure_env_type = Some(env_type);
        self.empty_closure_env_pointer_type = Some(env_pointer_type);

        env_type
    }

    /// Get or create the empty closure env pointer type for comptime lowering.
    fn empty_closure_env_pointer_type(&mut self) -> mir::LocalNodeId<mir::Type> {
        // reuse cached env pointer type
        if let Some(env_pointer_type) = self.empty_closure_env_pointer_type {
            return env_pointer_type;
        }

        // build the env pointer type
        let env_type = self.empty_closure_env_type();
        let env_pointer_type = self.builder.type_reference(
            mir::ReferenceKind::Managed,
            env_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            false,
        );
        self.empty_closure_env_pointer_type = Some(env_pointer_type);

        env_pointer_type
    }
}

/// Resolve the DIR type id for a typed expression.
fn dir_type_id_for_expression(
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
    module_id: destack_source::ModuleId,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalTypeId> {
    let expression = tree.get(expression_id);

    // prefer declared or inferred types over symbol derived types
    let type_id = if let Some(target_symbol) = expression.target_symbol() {
        types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            .or_else(|| types.get_value_type_id(target_symbol))
            .or_else(|| local_symbol_type_id(symbols, types, module_id, target_symbol))
    } else {
        types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
    }?;

    Some(type_id)
}

/// Resolve a local symbol to its declared or inferred type.
fn local_symbol_type_id(
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
    module_id: destack_source::ModuleId,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::LocalTypeId> {
    // skip cross module symbols
    if symbol_id.module_id != module_id {
        return None;
    }

    // use the primary declaration for local symbol types
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let primary_declaration = symbol.primary_declaration?;
    types.get_declared_or_inferred_type_id(primary_declaration)
}
