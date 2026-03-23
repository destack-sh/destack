use destack_artifact::Ast;
use destack_core::StringPool;
use destack_dir as dir;
use destack_dir::{SymbolTable, TypeTable};
use destack_workspace::{Module, Target};

use crate::lower::ModuleLowerer;
use crate::tree::NodeTree as JsTree;
use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning, LocalNodeIdAny};

/// Structured JavaScript emission for one module before assembly and printing.
#[derive(Debug)]
pub struct ModuleEmitOutput {
    /// The lowered JavaScript tree.
    pub tree: JsTree,
    /// Root nodes in the lowered tree.
    pub roots: Vec<LocalNodeIdAny>,
    /// The emission string pool.
    pub strings: StringPool,
    /// Warnings encountered during emission.
    pub warnings: Vec<CodegenJsWarning>,
    /// Non fatal errors encountered during emission.
    pub errors: Vec<CodegenJsError>,
}

/// Emit one lowered JavaScript tree from elaborated DIR.
pub fn emit_module(
    module: &Module,
    ast: &Ast,
    dir_tree: &dir::NodeTree,
    dir_roots: &Vec<dir::LocalNodeId<dir::Expression>>,
    symbols: &SymbolTable,
    types: &TypeTable,
    target: &Target,
) -> CodegenJsResult<ModuleEmitOutput> {
    let mut lowerer = ModuleLowerer::new(module, ast, dir_tree, dir_roots, symbols, types, target);
    lowerer.lower_module()?;

    Ok(ModuleEmitOutput {
        tree: lowerer.tree,
        roots: lowerer.roots,
        strings: lowerer.strings,
        warnings: lowerer.warnings,
        errors: lowerer.errors,
    })
}
