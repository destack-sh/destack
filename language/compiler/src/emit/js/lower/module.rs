use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirExpanded, DirImported, DirMaterialized, DirParsed,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_js as js;
use destack_repository::Module;

use crate::EmitError;

use super::ModuleLowerer;

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
) -> (js::Module, Vec<EmitError>) {
    let bindings = materialized.binding_table(bound, expanded, declared, checked);
    let modules = expanded.module_table(imported);
    let patches = [expanded.patch.clone(), materialized.patch.clone()];
    let view = dir::View::with_patches(&parsed.tree, &patches);
    let mut lowerer = ModuleLowerer::new(
        module,
        view,
        materialized.roots.as_ref(),
        strings,
        bindings,
        modules,
    );
    lowerer.lower_module();

    (
        js::Module {
            tree: lowerer.tree,
            roots: lowerer.roots,
            strings: lowerer.strings,
        },
        lowerer.errors,
    )
}
