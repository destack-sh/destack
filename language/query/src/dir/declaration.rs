use destack_core::StringPool;
use destack_dir as dir;

/// Resolve a display name for a declaration.
pub(crate) fn declaration_display_name(
    strings: &StringPool,
    declaration: &dir::Declaration,
) -> String {
    // default block declarations to keyword labels
    if matches!(declaration, dir::Declaration::Global(_)) {
        return "global".to_string();
    }
    if matches!(declaration, dir::Declaration::Module(_)) {
        return "module".to_string();
    }

    // prefer the explicit declaration name
    declaration
        .name()
        .map(|name| strings.get(name.string()).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string())
}
