use destack_artifact::{Ast, DirChecked, DirDeclared};
use destack_core::StringPool;
use destack_js as js;
use destack_workspace::{Module, Target};

use crate::lower::ModuleLowerer;
use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning};

/// Structured JavaScript lowering output for one module.
#[derive(Debug)]
pub struct ModuleLowerOutput {
    /// The lowered script module.
    pub module: js::Module,
    /// Warnings encountered during emission.
    pub warnings: Vec<CodegenJsWarning>,
    /// Non fatal errors encountered during emission.
    pub errors: Vec<CodegenJsError>,
}

/// Lower one patched DIR module into a structured JavaScript module.
pub fn lower_module(
    module: &Module,
    ast: &Ast,
    strings: &StringPool,
    declared: &DirDeclared,
    checked: &DirChecked,
    target: &Target,
) -> CodegenJsResult<ModuleLowerOutput> {
    let mut lowerer = ModuleLowerer::new(module, ast, strings, declared, checked, target);
    lowerer.lower_module()?;

    Ok(ModuleLowerOutput {
        module: js::Module {
            tree: lowerer.tree,
            roots: lowerer.roots,
            strings: lowerer.strings,
        },
        warnings: lowerer.warnings,
        errors: lowerer.errors,
    })
}
