use std::collections::{HashMap, HashSet};

use super::language::LANGUAGE_LIBS;
use super::library::LIBRARY_LIBS;
use super::source::BuiltinLibrary;

/// Look up a builtin lib by name.
pub fn builtin_library(name: &str) -> Option<&'static BuiltinLibrary> {
    builtin_libraries().find(|library| library.name == name)
}

/// Map a canonical tsconfig types package name to a builtin library name.
pub fn builtin_library_name_for_types_package(package_name: &str) -> Option<String> {
    // keep direct builtin lib names
    if builtin_library(package_name).is_some() {
        return Some(package_name.to_string());
    }

    // map declared types package names
    for library in builtin_libraries() {
        if library.types_package_names.contains(&package_name) {
            return Some(library.name.to_string());
        }
    }

    None
}

/// Resolve one builtin lib name against the requested profile lib set.
pub fn resolve_profile_builtin_library_name(
    name: &str,
    requested_libs: &[String],
) -> Option<String> {
    let name = builtin_library_name_for_types_package(name)?;
    if split_versioned_lib_name(&name).is_some() {
        return Some(name);
    }

    let version_overrides = collect_builtin_library_version_overrides(requested_libs);
    Some(resolve_builtin_library_dependency_name(
        &name,
        &version_overrides,
    ))
}

/// Iterate every builtin language or library definition.
fn builtin_libraries() -> impl Iterator<Item = &'static BuiltinLibrary> {
    LANGUAGE_LIBS.iter().chain(LIBRARY_LIBS.iter())
}

/// Split a name like `node.v24` into base and version.
fn split_versioned_lib_name(name: &str) -> Option<(&str, &str)> {
    let index = name.find(".v")?;
    let version = &name[index + 2..];
    if version.is_empty() {
        return None;
    }

    let is_versioned = version.chars().next().is_some_and(|ch| ch.is_ascii_digit());
    if !is_versioned {
        return None;
    }

    Some((&name[..index], version))
}

/// Collect explicit version overrides from one requested lib set.
fn collect_builtin_library_version_overrides(libs: &[String]) -> HashMap<String, String> {
    let mut overrides = HashMap::new();
    let mut seen = HashSet::new();

    for lib in libs {
        collect_builtin_library_version_overrides_for_library(lib, &mut overrides, &mut seen);
    }

    overrides
}

/// Collect explicit versioned dependencies for one builtin lib.
fn collect_builtin_library_version_overrides_for_library(
    name: &str,
    overrides: &mut HashMap<String, String>,
    seen: &mut HashSet<String>,
) {
    if !seen.insert(name.to_string()) {
        return;
    }

    if let Some((base, _version)) = split_versioned_lib_name(name) {
        overrides
            .entry(base.to_string())
            .or_insert_with(|| name.to_string());
    }

    let Some(library) = builtin_library(name) else {
        return;
    };

    for &dependency in library.dependencies {
        collect_builtin_library_version_overrides_for_library(dependency, overrides, seen);
    }

    for &dependency in library.reference_libs {
        collect_builtin_library_version_overrides_for_library(dependency, overrides, seen);
    }
}

/// Resolve one builtin dependency name through any version overrides.
fn resolve_builtin_library_dependency_name(
    name: &str,
    version_overrides: &HashMap<String, String>,
) -> String {
    if split_versioned_lib_name(name).is_some() {
        return name.to_string();
    }

    if let Some(override_name) = version_overrides.get(name) {
        return override_name.clone();
    }

    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::resolve_profile_builtin_library_name;

    #[test]
    fn test_resolve_profile_builtin_library_name_uses_versioned_runtime_dependency() {
        let libs = vec![
            "bun.v1.3".to_string(),
            "js".to_string(),
            "esnext".to_string(),
        ];

        assert_eq!(
            resolve_profile_builtin_library_name("node", &libs),
            Some("node.v24".to_string())
        );
    }

    #[test]
    fn test_resolve_profile_builtin_library_name_uses_versioned_transitive_dependency() {
        let libs = vec!["node.v24".to_string(), "esnext".to_string()];

        assert_eq!(
            resolve_profile_builtin_library_name("undici-types", &libs),
            Some("undici-types.v7".to_string())
        );
    }
}
