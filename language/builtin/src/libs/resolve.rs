use super::lib::LIBS;
use super::source::BuiltinLib;
use super::std::STD_LIB;

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
        if lib
            .types_package_names
            .iter()
            .any(|name| package_name == *name)
        {
            return Some(lib.name.to_string());
        }
    }

    None
}

/// Iterate every builtin lib, including std.
fn builtin_libs() -> impl Iterator<Item = &'static BuiltinLib> {
    std::iter::once(&STD_LIB).chain(LIBS.iter())
}
