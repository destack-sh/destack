use std::path::PathBuf;

use crate::tests::TestDaemon;

/// Tracks program handles per workspace root.
#[test]
fn test_daemon_tracks_program_handles_per_root() {
    let root_a = PathBuf::from("/workspace/a");
    let root_b = PathBuf::from("/workspace/b");
    let test = TestDaemon::new_with_roots(vec![root_a.clone(), root_b.clone()]);

    let file_a = root_a.join("main.ds");
    let file_b = root_b.join("main.ds");
    test.update_file(&file_a, "export const a = 1;");
    test.update_file(&file_b, "export const b = 2;");

    // assertion block: each root has its own handle
    assert_eq!(test.daemon.program_handle_count(), 2);
}
