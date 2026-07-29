use destack_artifact::ArtifactKey;
use destack_source::TargetId;

use crate::tests::TestSession;

/// Every builtin module lowers through the artifact path.
#[test]
fn test_lower_library() {
    let session = TestSession::builder().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let package_id = package.package_id();
    let target = TargetId::new(package_id, "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    let keys: Vec<_> = package
        .files()
        .iter()
        .map(|file| ArtifactKey::mir_lowered(file.module_id(package_id), profile, target))
        .collect();
    if let Err(error) = session.require_all(keys.iter().copied()) {
        panic!("{error}");
    }
}
