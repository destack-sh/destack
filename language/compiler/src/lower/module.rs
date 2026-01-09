use std::collections::HashMap;

use destack_base::StringPool;
use destack_dir::{GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalSymbolId};
use destack_source::ModuleId;
use destack_workspace::{Module, TargetId};
use {destack_dir as dir, destack_mir as mir};

use crate::{Compiler, LowerError, LowerResult};

use super::{GlobalBinding, TypeLowerer};

/// Context for lowering a DIR module to MIR.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ModuleLowerer<'a> {
    /// Provide access to the compiler for shared resources.
    pub(crate) compiler: &'a Compiler,
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Provide access to the source module data.
    pub(crate) module: &'a Module,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to the root expressions for the module.
    pub(crate) dir_roots: &'a [LocalNodeId<dir::Expression>],
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Identify the target backend for lowering.
    pub(crate) target: &'a TargetId,

    /// Build MIR nodes for this module.
    pub(crate) builder: mir::ModuleBuilder,
    /// Map DIR symbols to MIR function ids.
    pub(crate) functions_by_symbol: HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Map DIR symbols to MIR global bindings.
    pub(crate) globals_by_symbol: HashMap<GlobalSymbolId, GlobalBinding>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: TypeLowerer,
}

#[allow(clippy::too_many_arguments)]
impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        dir_tree: &'a dir::NodeTree,
        dir_roots: &'a [LocalNodeId<dir::Expression>],
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        target: &'a TargetId,
        pointer_bytes: u8,
    ) -> Self {
        let mut builder = mir::ModuleBuilder::new();
        let type_lowerer = TypeLowerer::new(&mut builder, pointer_bytes);

        Self {
            compiler,
            module_id: module.id,
            module,
            dir_tree,
            dir_roots,
            symbols,
            types,
            target,
            builder,
            functions_by_symbol: HashMap::new(),
            globals_by_symbol: HashMap::new(),
            type_lowerer,
        }
    }

    /// Lower this entire DIR module to MIR (in-place).
    pub(crate) fn lower_module(&mut self) -> LowerResult<()> {
        for expression_id in self.dir_roots.iter().copied() {
            self.lower_root_expression(expression_id)?;
        }
        Ok(())
    }

    /// Finish the module lowering process and return the resulting MIR tree and string pool.
    pub(crate) fn finish(self) -> (mir::NodeTree, StringPool) {
        self.builder.finish_mutable()
    }

    /// Get the name of a symbol as a String.
    pub(crate) fn get_symbol_name(&self, symbol_id: LocalSymbolId) -> Option<String> {
        let symbol = self.symbols.get_symbol(symbol_id);
        symbol
            .name()
            .map(|n| self.compiler.program.strings.get(n).to_string())
    }

    /// Get the name of a symbol, returning an error if it has no name.
    pub(crate) fn symbol_name(
        &self,
        symbol_id: LocalSymbolId,
        node: GlobalNodeIdAny,
    ) -> LowerResult<String> {
        self.get_symbol_name(symbol_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node,
                message: "symbol must have a name".to_string(),
            })
    }
}
