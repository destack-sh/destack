use destack_base::StringPool;
use destack_dir::Declaration;

/// Resolve a display name for a declaration.
pub(crate) fn declaration_display_name(strings: &StringPool, declaration: &Declaration) -> String {
    // default global declarations to the keyword label
    if matches!(declaration, Declaration::Global { .. }) {
        return "global".to_string();
    }

    // prefer descriptor name when available
    let descriptor = declaration.descriptor();
    descriptor
        .name
        .map(|name| strings.get(name.string()).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string())
}
