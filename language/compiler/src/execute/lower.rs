use std::collections::HashMap;

use crate::lower::{BuiltinTypeLayouts, FunctionEnv, FunctionState, TypeLowerer};
use crate::{Compiler, ExecuteError, ExecuteResult, FunctionContext, LowerError, ModuleLowerer};

use destack_workspace::{Module, ProfileId, TargetId};
use {destack_dir as dir, destack_mir as mir};

#[allow(dead_code)]
impl Compiler {
    /// Lower a module to MIR for comptime execution.
    pub(crate) fn lower_comptime_module(
        &self,
        module: &std::sync::Arc<parking_lot::RwLock<Module>>,
        profile: ProfileId,
        target_id: &TargetId,
    ) -> ExecuteResult<(mir::NodeTree, destack_base::StringPool)> {
        // snapshot DIR inputs for lowering
        let module_guard = module.read();
        let dir = module_guard.dir(profile);
        let module_id = module_guard.id;
        // TODO #Performance!: avoid cloning whole node dir tree for comptime
        let dir_tree = dir.tree.read().clone();
        let dir_roots = dir.roots.clone();
        let symbols = dir.symbols.read().clone();
        let types = dir.types.read().clone();

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
            &module_guard,
            profile,
            &dir_tree,
            &dir_roots,
            &symbols,
            &types,
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
        destack_base::StringPool,
        mir::LocalNodeId<mir::Function>,
    )> {
        // snapshot DIR inputs
        let dir = module.dir(profile);
        let dir_tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // initialize MIR builder and type lowerer
        let mut builder = mir::ModuleBuilder::new();

        // seed the mir string pool with program strings
        let strings = self.program.strings.as_ref().clone().into_immutable();
        builder.strings().copy_from_immutable(&strings);

        let dispatch_call_name = builder.intern("@call");
        let dispatch_construct_name = builder.intern("@new");
        let mut type_lowerer = {
            // TODO #Broken: comptime uses host pointer width (until execute is target-aware? should it?)
            let pointer_bytes = std::mem::size_of::<usize>() as u8;
            self.validate_pointer_bytes(module.id, pointer_bytes)
                .map_err(|error| ExecuteError::FailedLower {
                    module: module.id,
                    error: Box::new(error.clone()),
                    message: format!("{error}"),
                })?;
            TypeLowerer::new(
                &mut builder,
                pointer_bytes,
                self.program.modules.clone(),
                self.program.packages.clone(),
            )
        };

        // prepare builtin layouts for comptime lowering
        let anchor = expression_id
            .into_global_any(module.id)
            .into_anchored(Some(profile));
        let mut builtin_layouts =
            BuiltinTypeLayouts::new(self, profile, &mut builder, &mut type_lowerer);

        // load builtin String layout for comptime lowering
        builtin_layouts
            .string_type_for_builtin(anchor)
            .map_err(|error| self.execute_error_from_lower(module.id, error))?;

        // resolve the return type for the comptime expression
        let return_type_id =
            dir_type_id_for_expression(&dir_tree, &symbols, &types, module.id, expression_id)
                .ok_or_else(|| ExecuteError::FailedLower {
                    module: module.id,
                    error: Box::new(LowerError::MissingType {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    }),
                    message: "missing type".to_string(),
                })?;
        let return_type = type_lowerer
            .lower_type(&types, return_type_id, module.id, anchor, &mut builder)
            .map_err(|error| ExecuteError::FailedLower {
                module: module.id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;

        // build a synthetic function to evaluate the expression
        let function_builder = builder.function("comptime", &[], return_type);

        // create empty maps for function/global lookup (comptime expressions are standalone)
        let functions_by_symbol = HashMap::new();
        let globals_by_symbol = HashMap::new();
        let interface_slots_by_symbol = HashMap::new();
        let interface_itab_ids = HashMap::new();
        let virtual_method_slots_by_symbol = HashMap::new();
        let vtable_globals_by_symbol = HashMap::new();
        let function_signature_types = HashMap::new();

        // create function context
        let env = FunctionEnv {
            module_id: module.id,
            profile,
            program: &self.program,
            dir_tree: &dir_tree,
            symbols: &symbols,
            types: &types,
            strings: &self.program.strings,
            functions_by_symbol: &functions_by_symbol,
            function_signature_types: &function_signature_types,
            globals_by_symbol: &globals_by_symbol,
            interface_slots_by_symbol: &interface_slots_by_symbol,
            interface_itab_ids: &interface_itab_ids,
            virtual_method_slots_by_symbol: &virtual_method_slots_by_symbol,
            vtable_globals_by_symbol: &vtable_globals_by_symbol,
            dispatch_call_name,
            dispatch_construct_name,
            type_lowerer: &type_lowerer,
        };
        let state = FunctionState::new(function_builder);
        let mut function_ctx = FunctionContext::new(env, state);

        // create entry block
        let entry_block = function_ctx.state.builder.create_block();
        function_ctx.state.builder.switch_to_block(entry_block);

        // lower the expression
        let (value, _) = function_ctx
            .lower_value_expression(expression_id)
            .map_err(|error| ExecuteError::FailedLower {
                module: module.id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;

        // return and finish
        function_ctx.state.builder.return_(Some(value));
        let function_id = function_ctx.state.builder.finish();

        let (tree, strings) = builder.finish_mutable();
        Ok((tree, strings, function_id))
    }

    /// Map lower errors into execute errors.
    fn execute_error_from_lower(
        &self,
        module_id: destack_source::ModuleId,
        error: LowerError,
    ) -> ExecuteError {
        // preserve dependency yields as-is
        match error {
            LowerError::Yield { dependency } => ExecuteError::Yield { dependency },
            LowerError::UnsatisfiedDependency { dependency } => {
                ExecuteError::UnsatisfiedDependency { dependency }
            }
            error => ExecuteError::FailedLower {
                module: module_id,
                message: format!("{error}"),
                error: Box::new(error),
            },
        }
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
    if symbol_id.module_id != module_id {
        return None;
    }

    let symbol = symbols.get_symbol(symbol_id.local_id);
    let primary_declaration = symbol.primary_declaration?;
    types.get_declared_or_inferred_type_id(primary_declaration)
}
