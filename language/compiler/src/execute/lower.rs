use crate::{BlockLowerer, Compiler, ExecuteError, ExecuteResult, LowerError, ModuleLowerer};

use destack_mir as mir;
use destack_workspace::TargetId;
use destack_dir as dir;

#[allow(dead_code)]
impl Compiler {
    /// Lower a module to MIR for comptime execution.
    pub(crate) fn lower_comptime_module(
        &self,
        module: &std::sync::Arc<parking_lot::RwLock<destack_workspace::Module>>,
        profile: destack_workspace::ProfileId,
        target_id: &TargetId,
    ) -> ExecuteResult<(mir::NodeTree, destack_base::StringPool)> {
        let module_guard = module.read();
        let dir = module_guard.dir(profile);
        let module_id = module_guard.id;
        let dir_tree = dir.tree.read().clone();
        let dir_roots = dir.roots.clone();
        let symbols = dir.symbols.read().clone();
        let types = dir.types.read().clone();

        let mut lowerer = ModuleLowerer::new(
            self,
            &module_guard,
            &dir_tree,
            &dir_roots,
            &symbols,
            &types,
            target_id,
        );

        lowerer
            .lower_module()
            .map_err(|error| ExecuteError::FailedLower {
                module: module_id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;

        Ok(lowerer.finish())
    }

    /// Lower a comptime expression into a standalone MIR function.
    pub(crate) fn lower_comptime_expression(
        &self,
        module: &destack_workspace::Module,
        profile: destack_workspace::ProfileId,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> ExecuteResult<(mir::NodeTree, destack_base::StringPool, mir::LocalNodeId<mir::Function>)> {
        let dir = module.dir(profile);
        let dir_tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // resolve the return type for the comptime expression
        let return_type_id = dir_type_id_for_expression(
            &dir_tree,
            &symbols,
            &types,
            module.id,
            expression_id,
        )
        .ok_or_else(|| lower_failed(module.id, LowerError::MissingType {
            node: expression_id.into_global_any(module.id),
        }))?;

        let mut builder = mir::ModuleBuilder::new();
        let mut type_lowerer = crate::TypeLowerer::new(&mut builder);
        let return_type =
            type_lowerer
                .lower_type(
                    &types,
                    return_type_id,
                    module.id,
                    expression_id.into_global_any(module.id),
                    &mut builder,
                )
                .map_err(|error| lower_failed(module.id, error))?;

        // build a synthetic function to evaluate the expression
        let mut function_builder = builder.function("comptime_expression", &[], return_type);
        let entry_block = function_builder.create_block();
        function_builder.switch_to_block(entry_block);

        let mut locals_by_symbol = std::collections::HashMap::new();
        let functions_by_symbol = std::collections::HashMap::new();
        let mut block_lowerer = BlockLowerer {
            module_id: module.id,
            dir_tree: &dir_tree,
            symbols: &symbols,
            types: &types,
            type_lowerer: &type_lowerer,
            functions_by_symbol: &functions_by_symbol,
            builder: &mut function_builder,
            locals_by_symbol: &mut locals_by_symbol,
        };

        let (value, _) = block_lowerer
            .lower_value_expression(expression_id)
            .map_err(|error| lower_failed(module.id, error))?;
        function_builder.return_(Some(value));
        let function_id = function_builder.finish();

        let (tree, strings) = builder.finish_mutable();
        Ok((tree, strings, function_id))
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
    let type_id = match expression {
        dir::Expression::LocalReference { target_symbol, .. }
        | dir::Expression::ModuleReference { target_symbol, .. }
        | dir::Expression::GlobalReference { target_symbol, .. } => types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            .or_else(|| types.get_value_type_id(*target_symbol))
            .or_else(|| local_symbol_type_id(symbols, types, module_id, *target_symbol)),
        _ => types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id)),
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

fn lower_failed(module_id: destack_source::ModuleId, error: LowerError) -> ExecuteError {
    ExecuteError::FailedLower {
        module: module_id,
        error: Box::new(error.clone()),
        message: format!("{error}"),
    }
}
