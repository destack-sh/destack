use destack_artifact::DirView;
use destack_core::StringPool;
use destack_dir as dir;
use destack_js as js;
use destack_repository::Module;

use crate::EmitError;

use super::ModuleLowerer;

/// Lower one patched DIR module into a structured JavaScript module.
pub(in crate::emit::js) fn lower_module(
    module: &Module,
    view: &DirView,
    strings: &StringPool,
) -> (js::Module, Vec<EmitError>) {
    let parsed = &view.parsed;
    let expanded = &view.expanded;
    let materialized = view.materialized.as_ref().unwrap_or_else(|| unreachable!());
    let bindings = view.bindings().clone();
    let modules = view.modules().clone();
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
