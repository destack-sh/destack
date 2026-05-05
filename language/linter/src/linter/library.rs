use destack_workspace::Module;

/// Return true when one module path belongs to one required library package.
pub(crate) fn is_library_module(module: &Module, libraries: &[&str]) -> bool {
    let Some(module_path) = module.uri.as_ref().strip_prefix("library://") else {
        return false;
    };
    let module_path = module_path.strip_prefix("library/").unwrap_or(module_path);

    libraries
        .iter()
        .any(|library| is_library_module_path(module_path, library))
}

/// Return true when a module path is inside one library package.
fn is_library_module_path(module_path: &str, library: &str) -> bool {
    module_path == library
        || module_path
            .strip_prefix(library)
            .is_some_and(|rest| rest.starts_with('/'))
}
