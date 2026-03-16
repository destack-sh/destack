use destack_dir::{Extension, ExtensionKind, GlobalSymbolId};
use destack_source::ModuleId;

use crate::common::{QueryContext, get_canonical_symbol};
use destack_workspace::Session;

/// Visit each visible extension that targets the given symbol.
///
/// Returns early when the visitor returns true.
pub(crate) fn for_each_visible_extension(
    session: &Session,
    target_symbol: GlobalSymbolId,
    current_module_id: ModuleId,
    mut visit: impl FnMut(&QueryContext<'_>, &Extension) -> bool,
) {
    // normalize the target symbol across imports and re exports
    let canonical_target = get_canonical_symbol(session, target_symbol);

    // scan all modules for extensions that target the canonical symbol
    for module in session.modules.iter() {
        // read the module and query context
        let module = module.as_ref();
        let Some(ctx) = crate::query_context(session, &module) else {
            continue;
        };

        // resolve extension ids for the canonical target
        let types = ctx.types();
        let Some(extension_ids) = types.get_extensions_for_target(canonical_target) else {
            continue;
        };

        // iterate extensions and apply visibility rules
        for extension_id in extension_ids {
            let extension = types.get_extension(*extension_id);

            // filter out not visible extensions
            if !extension_is_visible(extension, current_module_id) {
                continue;
            }

            // stop scanning once the visitor is satisfied
            if visit(&ctx, extension) {
                return;
            }
        }
    }
}

/// Check whether an extension is visible from a module.
fn extension_is_visible(extension: &Extension, current_module_id: ModuleId) -> bool {
    // choose visibility rules by extension kind
    match extension.kind {
        ExtensionKind::Inherent => true,
        ExtensionKind::Local => extension.symbol.module_id == current_module_id,
        ExtensionKind::Nominal => true,
    }
}
