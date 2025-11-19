use super::TestResolver;

/// Test resolving package-json-nested always uses package.json at correct location.
#[test]
fn test_resolve_package_json_nested_uses_correct_package_json() {
    let f = super::fixture_root().join("misc");

    let resolver = TestResolver::default();

    let data = [
        (f.clone(), "package-json-nested"),
        // Nested package.json do not participate in module resolution.
        (f.clone(), "package-json-nested/foo"),
        (f.clone(), "package-json-nested/foo/bar"),
    ];

    let resolved_package_json_path = f.join("node_modules/package-json-nested/package.json");

    for (path, request) in data {
        let package_json = resolver
            .resolve(&path, request)
            .ok()
            .and_then(|f| f.package_json().cloned());
        let package_json_path = package_json.as_ref().map(|p| &p.path);
        let package_json_name = package_json.as_ref().and_then(|p| p.name.clone());
        assert_eq!(
            package_json_path,
            Some(&resolved_package_json_path),
            "{path:?} {request}"
        );
        assert_eq!(
            package_json_name,
            Some("package-json-nested".to_string()),
            "{path:?} {request}"
        );
    }
}

/// Test returning fixture-root package.json when resolving file adjacent to node_modules.
#[test]
fn test_return_package_json_adjacent_to_node_modules() {
    let f = super::fixture_root().join("misc");

    let resolver = TestResolver::default();

    // populate cache
    let _ = resolver.resolve(&f, "package-json-nested");

    let path = f.join("dir-with-index");
    let request = "./index.js";
    let resolved_package_json_path = f.join("package.json");

    let package_json = resolver
        .resolve(&path, request)
        .unwrap()
        .package_json()
        .cloned();
    let package_json_path = package_json.as_ref().map(|p| &p.path);
    let package_json_name = package_json.as_ref().and_then(|p| p.name.clone());
    assert_eq!(package_json_path, Some(&resolved_package_json_path));
    assert_eq!(package_json_name, Some("misc".to_string()));
}

/// Test returning package.json when resolving with symlinks=true.
#[test]
fn test_return_package_json_with_symlinks_true() {
    use crate::ResolveOptions;

    let f = super::fixture_root().join("misc");
    let resolver = TestResolver::new(ResolveOptions {
        canonicalize_symlinks: true,
        ..ResolveOptions::default()
    });

    let path = f.join("dir-with-index");
    let request = "./index.js";
    let resolved_package_json_path = f.join("package.json");

    let package_json = resolver
        .resolve(&path, request)
        .unwrap()
        .package_json()
        .cloned();
    let package_json_path = package_json.as_ref().map(|p| &p.path);
    assert_eq!(package_json_path, Some(&resolved_package_json_path));
}

/// Test erroring on various corrupted package.json files.
#[test]
#[cfg(not(target_os = "windows"))]
fn test_error_on_corrupted_package_json() {
    use std::path::Path;

    use crate::{ResolveError, ResolveOptions, Resolver};
    use dyst_source::MemoryFileSystem;

    type MemoryResolver = Resolver<MemoryFileSystem>;

    // Test scenarios for various corrupted package.json files
    let scenarios = [
        ("empty_file", "", "File is empty"),
        ("null_byte_at_start", "\0", "expected value"),
        (
            "json_with_embedded_null",
            "{\"name\":\0\"test\"}",
            "expected value",
        ),
        ("trailing_comma", "{\"name\":\"test\",}", "trailing comma"),
        ("unclosed_brace", "{\"name\":\"test\"", "EOF while parsing"),
        ("invalid_escape", "{\"name\":\"test\\x\"}", "escape"),
    ];

    for (name, content, expected_message_contains) in scenarios {
        let fs = MemoryFileSystem::default();

        // Write corrupted package.json
        _ = fs.add_file(Path::new("/test/package.json"), content.as_bytes());

        // Create a simple index.js so resolution can proceed
        _ = fs.add_file(Path::new("/test/index.js"), b"export default 42;");

        // Create resolver with VFS
        let resolver = MemoryResolver::from_file_system(fs, ResolveOptions::default());

        // Attempt to resolve - should fail with JSONError
        let result = resolver.resolve(Path::new("/test"), "./index.js");

        match result {
            Err(ResolveError::Json { error: json_error }) => {
                assert!(
                    json_error
                        .message
                        .to_lowercase()
                        .contains(&expected_message_contains.to_lowercase()),
                    "Test case '{name}': Expected error message to contain '{expected_message_contains}', but got: {}",
                    json_error.message
                );
                assert!(
                    json_error.path.ends_with("package.json"),
                    "Test case '{name}': Expected path to end with 'package.json', but got: {:?}",
                    json_error.path
                );
            }
            Err(other_error) => {
                panic!("Test case '{name}': Expected JSONError but got: {other_error:?}");
            }
            Ok(resolution) => {
                panic!(
                    "Test case '{name}': Expected error but resolution succeeded: {resolution:?}"
                );
            }
        }
    }
}
