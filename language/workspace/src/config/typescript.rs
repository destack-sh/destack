use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_builtin::{builtin_lib, builtin_lib_name_for_types_package};
use destack_source::{FileSystem, PathExt};

use crate::TsConfigOptions;

const TYPESCRIPT_LIB_PREFIX: &str = "lib.";
const TYPESCRIPT_LIB_SUFFIX: &str = ".d.ts";
const TYPESCRIPT_IMPLICIT_HOST_LIBS: &[&str] = &["dom", "dom.iterable", "scripthost"];
const TYPESCRIPT_DEFAULT_TYPE_ROOT_SUFFIX: &str = "node_modules/@types";
const TYPESCRIPT_TYPES_PACKAGE_PREFIX: &str = "@types/";

/// Normalize one TypeScript lib name to builtin lookup form.
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

/// Normalize TypeScript lib names to builtin lookup form.
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

/// Map type entries to builtin ambient library names.
pub fn builtin_libs_for_type_entries(type_entries: &[String]) -> Vec<String> {
    let mut builtin_types = Vec::new();
    let mut seen = HashSet::new();

    // project each type entry to one builtin lib when possible
    for type_entry in type_entries {
        let normalized_type_name = normalize_typescript_type_package_name(type_entry);
        if normalized_type_name.is_empty() {
            continue;
        }

        let builtin_name = if builtin_lib(&normalized_type_name).is_some() {
            Some(normalized_type_name.clone())
        } else {
            builtin_lib_name_for_types_package(&normalized_type_name)
        };

        let Some(builtin_name) = builtin_name else {
            continue;
        };
        if !seen.insert(builtin_name.clone()) {
            continue;
        }

        builtin_types.push(builtin_name);
    }

    builtin_types
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

    // start with TypeScript default ambient libs
    let mut libs = vec![
        "js".to_string(),
        ts_compiler_options.es_target.default_lib_name().to_string(),
    ];

    // add host libs
    for lib in TYPESCRIPT_IMPLICIT_HOST_LIBS {
        libs.push((*lib).to_string());
    }

    libs
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

#[cfg(test)]
mod tests {
    use super::*;
    use destack_source::MemoryFileSystem;

    #[test]
    fn test_normalize_typescript_lib_name_supports_prefixed_file_names() {
        let normalized = normalize_typescript_lib_name("lib.ES2022.d.ts");
        assert_eq!(normalized, "es2022");
    }

    #[test]
    fn test_normalize_typescript_type_package_name_supports_at_types_prefix() {
        let normalized = normalize_typescript_type_package_name(" @types/Node ");
        assert_eq!(normalized, "node");
    }

    #[test]
    fn test_normalize_typescript_type_entries_removes_duplicates() {
        let types = vec![
            "@types/node".to_string(),
            "NODE".to_string(),
            "vitest/globals".to_string(),
        ];

        let normalized_types = normalize_typescript_type_entries(&types);
        assert_eq!(
            normalized_types,
            vec!["node".to_string(), "vitest/globals".to_string()]
        );
    }

    #[test]
    fn test_builtin_libs_for_type_entries_drops_non_builtin_entries() {
        let types = vec![
            "@types/node".to_string(),
            "bun-types".to_string(),
            "vitest/globals".to_string(),
            "dom.iterable".to_string(),
        ];

        let builtin_types = builtin_libs_for_type_entries(&types);
        assert_eq!(
            builtin_types,
            vec![
                "node".to_string(),
                "bun".to_string(),
                "dom.iterable".to_string()
            ]
        );
    }

    #[test]
    fn test_typescript_default_libs_uses_implicit_host_set() {
        let tsconfig_options = TsConfigOptions::default();
        let libs = typescript_default_libs(&tsconfig_options);
        assert_eq!(
            libs,
            vec![
                "js".to_string(),
                "esnext".to_string(),
                "dom".to_string(),
                "dom.iterable".to_string(),
                "scripthost".to_string(),
            ]
        );
    }

    #[test]
    fn test_resolve_typescript_type_root_directories_uses_explicit_type_roots() {
        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.type_roots = vec![
            "./types".to_string(),
            "../shared/types".to_string(),
            "./types".to_string(),
        ];

        let roots = resolve_typescript_type_root_directories(
            &tsconfig_options,
            Path::new("/workspace/pkg/config"),
            Some(Path::new("/workspace/pkg/src/index.ts")),
        );

        assert_eq!(
            roots,
            vec![
                PathBuf::from("/workspace/pkg/config/types"),
                PathBuf::from("/workspace/pkg/shared/types"),
            ]
        );
    }

    #[test]
    fn test_resolve_typescript_type_root_directories_uses_default_ancestor_roots() {
        let tsconfig_options = TsConfigOptions::default();

        let roots = resolve_typescript_type_root_directories(
            &tsconfig_options,
            Path::new("/workspace/pkg"),
            Some(Path::new("/workspace/pkg/src/sub/index.ts")),
        );

        assert!(roots.starts_with(&[
            PathBuf::from("/workspace/pkg/src/sub/node_modules/@types"),
            PathBuf::from("/workspace/pkg/src/node_modules/@types"),
            PathBuf::from("/workspace/pkg/node_modules/@types"),
            PathBuf::from("/workspace/node_modules/@types"),
        ]));
    }

    #[test]
    fn test_resolve_typescript_type_root_directories_explicit_overrides_default_ancestors() {
        let mut tsconfig_options = TsConfigOptions::default();
        tsconfig_options.compiler.type_roots = vec!["./types".to_string()];

        let roots = resolve_typescript_type_root_directories(
            &tsconfig_options,
            Path::new("/workspace/pkg"),
            Some(Path::new("/workspace/pkg/src/index.ts")),
        );

        assert_eq!(roots, vec![PathBuf::from("/workspace/pkg/types")]);
    }

    #[test]
    fn test_discover_typescript_type_entries_normalizes_visible_packages() {
        let fs = MemoryFileSystem::new();
        fs.create_dir_all(Path::new("/workspace/pkg/node_modules/@types/node"))
            .expect("create @types/node directory");
        fs.create_dir_all(Path::new("/workspace/pkg/node_modules/@types/react"))
            .expect("create @types/react directory");
        fs.create_dir_all(Path::new(
            "/workspace/pkg/node_modules/@types/@scope/toolkit",
        ))
        .expect("create scoped package directory");

        let tsconfig_options = TsConfigOptions::default();
        let type_entries = discover_typescript_type_entries(
            &fs,
            &tsconfig_options,
            Path::new("/workspace/pkg"),
            Some(Path::new("/workspace/pkg/src/index.ts")),
        );

        assert!(type_entries.iter().any(|entry| entry == "node"));
        assert!(type_entries.iter().any(|entry| entry == "react"));
        assert!(type_entries.iter().any(|entry| entry == "@scope/toolkit"));
    }
}
