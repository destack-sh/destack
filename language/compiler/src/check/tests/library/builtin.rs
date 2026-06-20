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
