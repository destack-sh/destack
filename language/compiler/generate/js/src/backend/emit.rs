use destack_artifact::{DirBound, DirChecked, DirExpanded, DirParsed};
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
    parsed: &DirParsed,
    strings: &StringPool,
    bound: &DirBound,
    expanded: &DirExpanded,
    checked: &DirChecked,
    target: &Target,
) -> CodegenJsResult<ModuleLowerOutput> {
    let bindings = expanded.binding_table(bound);
    let types = checked.type_table(bound, expanded);
    let mut lowerer = ModuleLowerer::new(module, parsed, strings, bound, bindings, &types, target);
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
