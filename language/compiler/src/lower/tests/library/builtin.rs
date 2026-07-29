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

/// Report the per-module lowering inventory behind test_lower_library.
///
/// Run with `--ignored` to print one line per builtin module: `ok` for
/// modules that lower, and each diagnostic or error otherwise.
#[test]
#[ignore = "diagnostic inventory for the library lowering ratchet"]
fn test_report_library_lowering_inventory() {
    let session = TestSession::builder().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let package_id = package.package_id();
    let target = TargetId::new(package_id, "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    for file in package.files() {
        let key = ArtifactKey::mir_lowered(file.module_id(package_id), profile, target);
        let Err(error) = session.require_all([key]) else {
            println!("ok {}", file.path);

            continue;
        };

        // print the rendered diagnostics, or the raw error without any
        let rendered = session.render_terminal_diagnostics_for(&[key]);
        let mut lines = rendered
            .lines()
            .filter(|line| line.contains("error["))
            .peekable();
        if lines.peek().is_none() {
            println!("fail {} :: {error}", file.path);
        }
        for line in lines {
            println!("fail {} :: {}", file.path, line.trim());
        }
    }
}
