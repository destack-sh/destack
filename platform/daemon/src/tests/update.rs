use std::path::PathBuf;
use std::sync::Arc;

use destack_source::MemoryFileSystem;
use destack_workspace::Session;

use crate::Daemon;

/// Ensure daemon updates emit diagnostics for invalid syntax.
#[test]
fn test_daemon_update_emits_diagnostics() {
    // create a session with an in memory filesystem
    let fs = Arc::new(MemoryFileSystem::new());
    let cwd = PathBuf::from("/workspace");
    let session = Arc::new(Session::new(cwd.clone()).with_fs(fs));
    session.add_root(cwd.clone());
    let daemon = Daemon::new(session);

    let path = cwd.join("main.ts");

    // apply initial valid content
    let initial = daemon
        .update_file(&path, "export const value = 1;".to_string())
        .expect("valid update failed");

    // assertion block: initial diagnostics are empty
    assert!(
        initial.diagnostics.is_empty(),
        "expected no diagnostics for valid content"
    );

    // apply invalid content
    let updated = daemon
        .update_file(&path, "export const value = ;".to_string())
        .expect("invalid update failed");

    // assertion block: invalid content emits diagnostics
    assert!(
        !updated.diagnostics.is_empty(),
        "expected diagnostics for invalid content"
    );
}
