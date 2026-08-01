use destack_artifact::{DirBound, DirChecked, DirDeclared, DirExpanded, DirImported, DirParsed};
use destack_core::StringPool;
use destack_js as js;
use destack_repository::Module;

use crate::EmitError;

use super::ModuleLowerer;

/// Structured JavaScript lowering output for one module.
#[derive(Debug)]
pub(in crate::emit::js) struct ModuleLowerOutput {
    /// The lowered JS module.
    pub(in crate::emit::js) module: js::Module,
    /// Non fatal errors encountered during emission.
    pub(in crate::emit::js) errors: Vec<EmitError>,
}

/// Lower one patched DIR module into a structured JavaScript module.
pub(in crate::emit::js) fn lower_module(
    module: &Module,
    parsed: &DirParsed,
    strings: &StringPool,
    bound: &DirBound,
    imported: &DirImported,
    expanded: &DirExpanded,
    declared: &DirDeclared,
    checked: &DirChecked,
) -> Result<ModuleLowerOutput, EmitError> {
    let bindings = expanded.binding_table(bound);
    let modules = expanded.module_table(imported);
    let types = checked.type_table(bound, expanded, declared);
    let statics = checked.static_table(bound, expanded, declared);
    let generics = checked.generic_table(declared);
    let mut lowerer = ModuleLowerer::new(
        module, parsed, strings, bound, bindings, &types, &statics, &generics, modules,
    );
    lowerer.lower_module()?;

    Ok(ModuleLowerOutput {
        module: js::Module {
            tree: lowerer.tree,
            roots: lowerer.roots,
            strings: lowerer.strings,
        },
        errors: lowerer.errors,
    })
}
