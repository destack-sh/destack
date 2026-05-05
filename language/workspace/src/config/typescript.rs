use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_source::{FileSystem, PathExt};

use crate::config::TsConfigOptions;

const TYPESCRIPT_LIB_PREFIX: &str = "lib.";
const TYPESCRIPT_LIB_SUFFIX: &str = ".d.ts";
const TYPESCRIPT_DEFAULT_TYPE_ROOT_SUFFIX: &str = "node_modules/@types";
const TYPESCRIPT_TYPES_PACKAGE_PREFIX: &str = "@types/";

/// Normalize one TypeScript lib name to a library package name.
pub fn normalize_typescript_lib_name(lib: &str) -> String {
    // normalize casing and whitespace
    let lower = lib.trim().to_ascii_lowercase();
    let without_prefix = lower
        .strip_prefix(TYPESCRIPT_LIB_PREFIX)
        .unwrap_or(lower.as_str());
    let normalized = without_prefix
        .strip_suffix(TYPESCRIPT_LIB_SUFFIX)
        .unwrap_or(without_prefix);

    normalized.to_string()
}

/// Normalize TypeScript lib names to library package names.
pub fn normalize_typescript_lib_names(libs: &[String]) -> Vec<String> {
    libs.iter()
        .map(|lib| normalize_typescript_lib_name(lib))
        .collect()
}

/// Normalize one TypeScript types package name to canonical lookup form.
pub fn normalize_typescript_type_package_name(package_name: &str) -> String {
    // normalize casing and whitespace
    let lower = package_name.trim().to_ascii_lowercase();

    // normalize @types package names to canonical type names
    let normalized = lower
        .strip_prefix(TYPESCRIPT_TYPES_PACKAGE_PREFIX)
        .unwrap_or(lower.as_str());

    normalized.to_string()
}

/// Normalize TypeScript type entries to canonical lookup form.
pub fn normalize_typescript_type_entries(types: &[String]) -> Vec<String> {
    let mut normalized_types = Vec::new();
    let mut seen = HashSet::new();

    // normalize each type entry once
    for type_name in types {
        let normalized_type_name = normalize_typescript_type_package_name(type_name);
        if normalized_type_name.is_empty() {
            continue;
        }
        if !seen.insert(normalized_type_name.clone()) {
            continue;
        }

        normalized_types.push(normalized_type_name);
    }

    normalized_types
}

/// Map type entries to requested library package names.
pub fn library_packages_for_type_entries(type_entries: &[String]) -> Vec<String> {
    let mut library_types = Vec::new();
    let mut seen = HashSet::new();

    // project each type entry to one requested package
    for type_entry in type_entries {
        let library_name = normalize_typescript_type_package_name(type_entry);
        if library_name.is_empty() {
            continue;
        };
        if !seen.insert(library_name.clone()) {
            continue;
        }

        library_types.push(library_name);
    }

    library_types
}

/// Build default TypeScript libs for one tsconfig.
pub fn typescript_default_libs(tsconfig_options: &TsConfigOptions) -> Vec<String> {
    let ts_compiler_options = &tsconfig_options.compiler;

    // tsconfig noLib disables implicit libs
    if ts_compiler_options.no_lib {
        return Vec::new();
    }

    // explicit tsconfig lib entries define the base lib set
    if !ts_compiler_options.lib.is_empty() {
        return normalize_typescript_lib_names(&ts_compiler_options.lib);
    }

    vec!["core".to_string()]
}

/// Resolve effective type root directories for one module.
pub fn resolve_typescript_type_root_directories(
    tsconfig_options: &TsConfigOptions,
    tsconfig_directory: &Path,
    module_path: Option<&Path>,
) -> Vec<PathBuf> {
    let mut root_directories = Vec::new();
    let mut seen = HashSet::new();

    // prefer explicit typeRoots entries from tsconfig
    if !tsconfig_options.compiler.type_roots.is_empty() {
        for configured_root in &tsconfig_options.compiler.type_roots {
            let trimmed_root = configured_root.trim();
            if trimmed_root.is_empty() {
                continue;
            }

            let configured_path = Path::new(trimmed_root);
            let root_directory = if configured_path.is_absolute() {
                configured_path.to_path_buf()
            } else {
                tsconfig_directory.join(configured_path)
            };
            let root_directory = root_directory.normalize();

            if seen.insert(root_directory.clone()) {
                root_directories.push(root_directory);
            }
        }

        return root_directories;
    }

    // otherwise use default visible node_modules/@types ancestors
    let start_directory = module_path
        .and_then(|path| path.parent())
        .unwrap_or(tsconfig_directory);
    let mut current_directory = Some(start_directory);

    while let Some(directory) = current_directory {
        let root_directory = directory
            .join(TYPESCRIPT_DEFAULT_TYPE_ROOT_SUFFIX)
            .normalize();

        if seen.insert(root_directory.clone()) {
            root_directories.push(root_directory);
        }

        current_directory = directory.parent();
    }

    root_directories
}

/// Discover normalized type entries visible from one module.
pub fn discover_typescript_type_entries(
    fs: &dyn FileSystem,
    tsconfig_options: &TsConfigOptions,
    tsconfig_directory: &Path,
    module_path: Option<&Path>,
) -> Vec<String> {
    // resolve effective type root directories
    let root_directories =
        resolve_typescript_type_root_directories(tsconfig_options, tsconfig_directory, module_path);
    if root_directories.is_empty() {
        return Vec::new();
    }

    // collect package names from each type root
    let mut type_entries = Vec::new();
    let mut seen = HashSet::new();
    for root_directory in root_directories {
        let root_entries = types_package_names_from_type_root(fs, root_directory.as_path());
        for root_entry in root_entries {
            let normalized_entry = normalize_typescript_type_package_name(&root_entry);
            if normalized_entry.is_empty() {
                continue;
            }
            if !seen.insert(normalized_entry.clone()) {
                continue;
            }

            type_entries.push(normalized_entry);
        }
    }

    type_entries
}

/// Collect package names from one type root directory.
fn types_package_names_from_type_root(
    fs: &dyn FileSystem,
    type_root_directory: &Path,
) -> Vec<String> {
    let Ok(entries) = fs.read_dir(type_root_directory) else {
        return Vec::new();
    };

    let mut package_names = Vec::new();

    // collect direct and scoped package names
    for entry in entries {
        let Some(package_name) = entry.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if package_name.starts_with('@') {
            let Ok(scoped_entries) = fs.read_dir(&entry) else {
                continue;
            };

            for scoped_entry in scoped_entries {
                let Some(scoped_name) = scoped_entry.file_name().and_then(|name| name.to_str())
                else {
                    continue;
                };

                package_names.push(format!("{package_name}/{scoped_name}"));
            }
            continue;
        }

        package_names.push(package_name.to_string());
    }

    // keep deterministic ordering across filesystems
    package_names.sort();
    package_names.dedup();

    package_names
}
