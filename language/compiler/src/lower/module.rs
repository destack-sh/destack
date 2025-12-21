use std::collections::HashMap;

use destack_base::StringPool;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;
use destack_workspace::Module;
use {destack_dir as dir, destack_mir as mir};

use crate::{Compiler, LowerError, LowerResult};

use super::TypeLowerer;

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
    pub(crate) target: &'a str,

    /// Build MIR nodes for this module.
    pub(crate) builder: mir::ModuleBuilder,
    /// Map DIR symbols to MIR function ids.
    pub(crate) functions_by_symbol: HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: TypeLowerer,
}

impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        dir_tree: &'a dir::NodeTree,
        dir_roots: &'a [LocalNodeId<dir::Expression>],
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        target: &'a str,
    ) -> Self {
        let mut builder = mir::ModuleBuilder::new();
        let type_lowerer = TypeLowerer::new(&mut builder);

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

    /// Lower a root expression.
    pub(crate) fn lower_root_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<()> {
        match self.dir_tree.get(expression_id) {
            Expression::Declaration { declaration } => {
                let declaration_id = *declaration;
                let declaration = self.dir_tree.get(declaration_id);
                self.lower_declaration(declaration_id, declaration)
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: "unsupported non-declaration expression".to_string(),
            })?,
        }
    }

    /// Finish the module lowering process and return the resulting MIR tree and string pool.
    pub(crate) fn finish(self) -> (mir::NodeTree, StringPool) {
        self.builder.finish_mutable()
    }
}
