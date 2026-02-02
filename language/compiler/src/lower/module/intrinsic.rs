use destack_dir as dir;
use destack_dir::{AnchoredGlobalNodeId, GlobalSymbolId};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};

use crate::lower::{FunctionContext, ModuleLowerer};
use crate::{LowerError, LowerResult};

impl ModuleLowerer<'_> {
    /// Resolve the intrinsic binding name for a symbol.
    pub(crate) fn resolve_intrinsic_binding_name_id(
        &self,
        node: AnchoredGlobalNodeId,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<Option<dir::StringId>> {
        self.require_analyzed_module(target_symbol.module_id)?;

        resolve_intrinsic_binding_name_id_inner(
            self.module_id,
            self.profile,
            &self.compiler.program,
            self.symbols,
            node,
            target_symbol,
        )
    }
}

impl FunctionContext<'_> {
    /// Resolve the intrinsic binding name for a symbol.
    pub(crate) fn resolve_intrinsic_binding_name_id(
        &self,
        node: AnchoredGlobalNodeId,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<Option<dir::StringId>> {
        resolve_intrinsic_binding_name_id_inner(
            self.env.module_id,
            self.env.profile,
            self.env.program,
            self.env.symbols,
            node,
            target_symbol,
        )
    }
}

/// Resolve the intrinsic binding name for a symbol.
fn resolve_intrinsic_binding_name_id_inner(
    module_id: ModuleId,
    profile: ProfileId,
    program: &Program,
    local_symbols: &dir::SymbolTable,
    node: AnchoredGlobalNodeId,
    target_symbol: GlobalSymbolId,
) -> LowerResult<Option<dir::StringId>> {
    // track visited symbols to avoid cycles
    let mut current_symbol = target_symbol;
    let mut visited = Vec::new();

    // walk target and canonical symbols to find intrinsic bindings
    loop {
        if visited.contains(&current_symbol) {
            return Ok(None);
        }
        visited.push(current_symbol);

        // load the symbol binding metadata
        let (binding, merge_group, symbol_name, canonical_symbol, next_symbol) =
            intrinsic_symbol_binding_info(
                module_id,
                profile,
                program,
                local_symbols,
                current_symbol,
            );

        // check the binding on the current symbol
        if let Some(name_id) =
            resolve_intrinsic_binding_name_id_from_decorator(node, binding, symbol_name)?
        {
            return Ok(Some(name_id));
        }

        // check merge group symbols for a binding
        if let Some(group_id) = merge_group
            && let Some(name_id) = resolve_merge_group_binding_name(
                node,
                module_id,
                profile,
                program,
                local_symbols,
                current_symbol,
                group_id,
            )?
        {
            return Ok(Some(name_id));
        }

        // follow canonical symbols before targets
        if let Some(canonical_symbol) = canonical_symbol {
            current_symbol = canonical_symbol;
            continue;
        }

        // follow the target symbol if present
        if let Some(target_symbol) = next_symbol {
            current_symbol = target_symbol;
            continue;
        }

        return Ok(None);
    }
}

/// Resolve the intrinsic binding name from a decorator.
fn resolve_intrinsic_binding_name_id_from_decorator(
    node: AnchoredGlobalNodeId,
    binding: Option<dir::IntrinsicBinding>,
    symbol_name: Option<dir::StringId>,
) -> LowerResult<Option<dir::StringId>> {
    let Some(binding) = binding.as_ref() else {
        return Ok(None);
    };

    let name_id = binding
        .name
        .or(symbol_name)
        .ok_or_else(|| LowerError::UnsupportedConstruct {
            node,
            message: "intrinsic binding missing symbol name".to_string(),
        })?;

    Ok(Some(name_id))
}

/// Read intrinsic binding metadata for a symbol.
#[allow(clippy::type_complexity)]
fn intrinsic_symbol_binding_info(
    module_id: ModuleId,
    profile: ProfileId,
    program: &Program,
    local_symbols: &dir::SymbolTable,
    symbol_id: GlobalSymbolId,
) -> (
    Option<dir::IntrinsicBinding>,
    Option<dir::LocalMergeGroupId>,
    Option<dir::StringId>,
    Option<GlobalSymbolId>,
    Option<GlobalSymbolId>,
) {
    if symbol_id.module_id == module_id {
        let symbol = local_symbols.get_symbol(symbol_id.local_id);
        (
            symbol.decorators.intrinsic_binding.clone(),
            symbol.merge_group,
            symbol.name(),
            symbol.canonical_symbol,
            symbol.target_symbol,
        )
    } else {
        let module = program.modules.get(symbol_id.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        (
            symbol.decorators.intrinsic_binding.clone(),
            symbol.merge_group,
            symbol.name(),
            symbol.canonical_symbol,
            symbol.target_symbol,
        )
    }
}

/// Resolve merge group binding names for the given symbol.
fn resolve_merge_group_binding_name(
    node: AnchoredGlobalNodeId,
    module_id: ModuleId,
    profile: ProfileId,
    program: &Program,
    local_symbols: &dir::SymbolTable,
    symbol_id: GlobalSymbolId,
    merge_group: dir::LocalMergeGroupId,
) -> LowerResult<Option<dir::StringId>> {
    if symbol_id.module_id == module_id {
        for symbol_id in local_symbols.merge_group_symbols(merge_group) {
            let symbol = local_symbols.get_symbol(*symbol_id);
            let binding = symbol.decorators.intrinsic_binding.clone();
            let symbol_name = symbol.name();
            if let Some(name_id) =
                resolve_intrinsic_binding_name_id_from_decorator(node, binding, symbol_name)?
            {
                return Ok(Some(name_id));
            }
        }
    } else {
        let module = program.modules.get(symbol_id.module_id);
        let module = module.read();
        let symbols = module.dir(profile).symbols.read();
        for symbol_id in symbols.merge_group_symbols(merge_group) {
            let symbol = symbols.get_symbol(*symbol_id);
            let binding = symbol.decorators.intrinsic_binding.clone();
            let symbol_name = symbol.name();
            if let Some(name_id) =
                resolve_intrinsic_binding_name_id_from_decorator(node, binding, symbol_name)?
            {
                return Ok(Some(name_id));
            }
        }
    }

    Ok(None)
}
