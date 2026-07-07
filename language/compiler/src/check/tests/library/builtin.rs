use destack_artifact::ArtifactKey;
use destack_source::TargetId;

use crate::tests::TestSession;

/// Check every builtin library module.
#[test]
fn test_check_library() {
    let session = TestSession::builder().build();
    let repository = session.repository();
    let package = repository.builtin_package();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .unwrap_or_else(|error| panic!("builtin library target profile should resolve:\n{error}"))
        .id();

    // check every builtin module through the normal artifact path
    let keys = package
        .module_ids()
        .map(|module| ArtifactKey::dir_checked(module, profile));
    let result = session.require_all(keys);

    let diagnostics = session.render_terminal_diagnostics(None);
    if !diagnostics.is_empty() {
        panic!("\n{diagnostics}");
    }

    if let Err(error) = result {
        panic!("{error}");
    }
}

/// Check one builtin library module for focused debugging.
#[test]
#[ignore = "debug helper for library triage"]
fn test_debug_library_module() {
    let Ok(path) = std::env::var("DESTACK_LIBRARY_MODULE") else {
        eprintln!("set DESTACK_LIBRARY_MODULE to a builtin module path");

        return;
    };
    let session = TestSession::builder().build();
    let repository = session.repository();
    let package = repository.builtin_package();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .unwrap_or_else(|error| panic!("builtin library target profile should resolve:\n{error}"))
        .id();

    let module = package
        .files()
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.module_id(package.package_id()))
        .unwrap_or_else(|| panic!("no builtin module at path {path:?}"));
    let result = session.require_all([ArtifactKey::dir_checked(module, profile)]);

    let diagnostics = session.render_terminal_diagnostics(None);
    if !diagnostics.is_empty() {
        panic!("\n{diagnostics}");
    }

    if let Err(error) = result {
        panic!("{error}");
    }
}
