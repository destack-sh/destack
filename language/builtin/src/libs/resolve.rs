use super::language::LANGUAGE_LIBS;
use super::library::LIBRARY_LIBS;
use super::source::BuiltinLib;

/// Look up a builtin lib by name.
pub fn builtin_lib(name: &str) -> Option<&'static BuiltinLib> {
    builtin_libs().find(|lib| lib.name == name)
}

/// Map a canonical tsconfig types package name to a builtin library name.
pub fn builtin_lib_name_for_types_package(package_name: &str) -> Option<String> {
    // keep direct builtin lib names
    if builtin_lib(package_name).is_some() {
        return Some(package_name.to_string());
    }

    // map declared types package names
    for lib in builtin_libs() {
        if lib.types_package_names.contains(&package_name) {
            return Some(lib.name.to_string());
        }
    }

    None
}

/// Iterate every builtin language or library definition.
fn builtin_libs() -> impl Iterator<Item = &'static BuiltinLib> {
    LANGUAGE_LIBS.iter().chain(LIBRARY_LIBS.iter())
}
