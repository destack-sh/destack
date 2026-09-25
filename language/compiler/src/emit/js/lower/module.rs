use tspp_artifact::DirView;
use tspp_core::StringPool;
use tspp_js as js;
use tspp_repository::Module;

use crate::EmitError;

use super::ModuleLowerer;

/// Lower one patched DIR module into a structured JavaScript module.
pub(in crate::emit::js) fn lower_module(
    module: &Module,
    view: &DirView,
    strings: &StringPool,
) -> (js::Module, Vec<EmitError>) {
    let materialized = view.materialized.as_ref().unwrap_or_else(|| unreachable!());
    let bindings = view.bindings().clone();
    let modules = view.modules().clone();
    let mut lowerer = ModuleLowerer::new(
        module,
        view.tree(),
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
