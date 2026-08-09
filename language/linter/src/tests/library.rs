use destack_artifact::ArtifactKey;
use destack_session::{ArtifactPriority, Session};
use destack_source::TargetId;
use futures::executor::block_on;

use super::session::{executor, render_diagnostics, shared_repository};

/// Lint the complete checked-in standard library.
#[test]
fn test_lint_library() {
    let (repository, revision) = shared_repository();
    let repository = repository.clone();
    let package = repository.embedded_builtin();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(*revision, target)
        .expect("library profile should resolve")
        .id();

    // provide every module lint and the complete program lint
    let mut keys = package
        .module_ids()
        .map(|module| ArtifactKey::module_linted(module, profile, target))
        .collect::<Vec<_>>();
    keys.push(ArtifactKey::program_linted(profile, target));
    let session =
        Session::new(repository.clone(), executor()).expect("library lint session should open");
    let run = session.provide(*revision, &keys, ArtifactPriority::Foreground);
    let result = block_on(run.wait());

    // collect every compiler and linter diagnostic together
    let diagnostics = repository
        .diagnostics(*revision, None)
        .expect("library diagnostics should be readable");
    let rendered = (!diagnostics.is_empty())
        .then(|| render_diagnostics(repository.as_ref(), *revision, &diagnostics));

    // report diagnostics and provider failures without suppressing either
    match (rendered, result) {
        (None, Ok(_)) => {}
        (Some(diagnostics), Ok(_)) => panic!("\n{diagnostics}"),
        (None, Err(error)) => panic!("{error}"),
        (Some(diagnostics), Err(error)) => panic!("\n{diagnostics}\n\n{error}"),
    }
}
