use destack_artifact::ArtifactKey;
use destack_source::TargetId;

use crate::tests::TestSession;

/// Every builtin module lowers through the artifact path.
#[test]
fn test_lower_library() {
    let session = TestSession::builder().cold().build();
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
    let session = TestSession::builder().cold().build();
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

        // print the rendered diagnostics, or the raw error when none render
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

/// Time one library module's checked artifact on a cold repository.
///
/// Run with `--ignored`; `DESTACK_TEST_TRACE_SLOW_MS=1` prints the run
/// breakdown with the module's own span.
#[test]
#[ignore = "timing probe for library modules"]
fn test_report_module_check_timing() {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let package_id = package.package_id();
    let target = TargetId::new(package_id, "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    let iterations = std::env::var("DESTACK_TIMING_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    for _ in 0..iterations {
        let session = TestSession::builder().cold().build();
        let keys = package
            .files()
            .iter()
            .map(|file| ArtifactKey::dir_checked(file.module_id(package_id), profile))
            .collect::<Vec<_>>();
        let started = std::time::Instant::now();
        session
            .require_all(keys)
            .expect("library modules should check");
        println!("cold library check {:?}", started.elapsed());
        session.print_trace("library-check", 400);
    }
}

/// Print one library module's lowered MIR for inspection.
#[test]
#[ignore = "MIR dump probe"]
fn test_dump_module_mir() {
    let path = std::env::var("DESTACK_DUMP_MODULE").expect("set DESTACK_DUMP_MODULE");
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let package_id = package.package_id();
    let target = TargetId::new(package_id, "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    let mut matched = false;
    for file in package.files() {
        if file.path != path {
            continue;
        }
        matched = true;
        let key = ArtifactKey::mir_lowered(file.module_id(package_id), profile, target);
        match session.require_all([key]) {
            Ok(()) => println!("{}", session.render_mir_snapshot(key)),
            Err(_) => println!("{}", session.render_terminal_diagnostics_for(&[key])),
        }
    }
    assert!(matched, "no builtin module at path {path}");
}
