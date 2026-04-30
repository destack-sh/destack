use destack_builtin::builtin_libraries;
use destack_workspace::Module;

/// Return true when one module path belongs to one required builtin library.
/// TODO #Cleanup: is_builtin_library_module seems a bit sus?
pub(crate) fn is_builtin_library_module(module: &Module, libraries: &[&str]) -> bool {
    let Some(module_path) = module.uri.as_ref().strip_prefix("builtin://") else {
        return false;
    };

    builtin_libraries()
        .filter(|library| libraries.contains(&library.name))
        .flat_map(|library| library.sources.iter())
        .any(|source| source.module_path() == module_path)
}
