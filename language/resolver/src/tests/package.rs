use super::TestResolver;

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
