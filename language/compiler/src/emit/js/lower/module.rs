use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirExpanded, DirImported, DirMaterialized, DirParsed,
};
use destack_core::StringPool;
use destack_dir as dir;
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
    materialized: &DirMaterialized,
) -> Result<ModuleLowerOutput, EmitError> {
    let bindings = materialized.binding_table(bound, expanded);
    let modules = expanded.module_table(imported);
    let types = materialized.type_table(bound, expanded, declared, checked);
    let statics = materialized.static_table(bound, expanded, declared, checked);
    let generics = materialized.generic_table(declared, checked);
    let patches = [expanded.patch.clone(), materialized.patch.clone()];
    let view = dir::View::with_patches(&parsed.tree, &patches);
    let mut lowerer = ModuleLowerer::new(
        module,
        view,
        materialized.roots.as_ref(),
        strings,
        bindings,
        &types,
        &statics,
        &generics,
        modules,
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
