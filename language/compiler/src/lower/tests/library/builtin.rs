use destack_artifact::ArtifactKey;
use destack_source::TargetId;

use crate::tests::TestSession;

/// Every builtin module lowers through the artifact path.
#[test]
fn test_lower_library() {
    let session = TestSession::builder().build();
    let repository = session.repository();
    let package = repository.builtin_package();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    // lower every builtin module
    let package_id = package.package_id();
    let keys = package
        .files()
        .iter()
        .map(|file| ArtifactKey::mir_lowered(file.module_id(package_id), profile, target))
        .collect::<Vec<_>>();
    // artifacts must provide; unsupported constructs still report recoverably
    if let Err(error) = session.require_all(keys.iter().copied()) {
        panic!("{error}");
    }
}
