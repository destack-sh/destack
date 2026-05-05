use std::collections::{HashMap, HashSet};

use super::catalog::{LANGUAGE_LIBS, LIBRARY_PACKAGES};
use super::source::LibraryPackage;

/// Iterate every language library package.
pub fn library_packages() -> impl Iterator<Item = &'static LibraryPackage> {
    LANGUAGE_LIBS.iter().chain(LIBRARY_PACKAGES.iter())
}

/// Look up a library package by name.
pub fn library_package(name: &str) -> Option<&'static LibraryPackage> {
    library_packages().find(|library| library.name == name)
}

/// Map a canonical tsconfig types package name to a library package name.
pub fn library_package_name_for_types_package(package_name: &str) -> Option<String> {
    // keep direct library package names
    if library_package(package_name).is_some() {
        return Some(package_name.to_string());
    }

    // map declared types package names
    for library in library_packages() {
        if library.types_package_names.contains(&package_name) {
            return Some(library.name.to_string());
        }
    }

    None
}

/// Resolve one library package name against the requested profile lib set.
pub fn resolve_profile_library_name(name: &str, requested_libs: &[String]) -> Option<String> {
    let name = library_package_name_for_types_package(name)?;
    if split_versioned_lib_name(&name).is_some() {
        return Some(name);
    }

    let version_overrides = collect_library_version_overrides(requested_libs);
    Some(resolve_library_dependency_name(&name, &version_overrides))
}

/// Split a name like `example.v2` into base and version.
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
fn collect_library_version_overrides(libs: &[String]) -> HashMap<String, String> {
    let mut overrides = HashMap::new();
    let mut seen = HashSet::new();

    for lib in libs {
        collect_library_version_overrides_for_library(lib, &mut overrides, &mut seen);
    }

    overrides
}

/// Collect explicit versioned dependencies for one library package.
fn collect_library_version_overrides_for_library(
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

    let Some(library) = library_package(name) else {
        return;
    };

    for &dependency in library.dependencies {
        collect_library_version_overrides_for_library(dependency, overrides, seen);
    }

    for &dependency in library.reference_libs {
        collect_library_version_overrides_for_library(dependency, overrides, seen);
    }
}

/// Resolve one library dependency name through any version overrides.
fn resolve_library_dependency_name(
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
