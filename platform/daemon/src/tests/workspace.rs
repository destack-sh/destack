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

    // check that each root has its own handle
    assert_eq!(test.daemon.program_handle_count(), 2);
}

/// Keeps updates isolated to the root that changed.
#[test]
fn test_daemon_updates_do_not_cross_roots() {
    let root_a = PathBuf::from("/workspace/a");
    let root_b = PathBuf::from("/workspace/b");
    let test = TestDaemon::new_with_roots(vec![root_a.clone(), root_b.clone()]);

    let file_a = root_a.join("main.ds");
    let file_b = root_b.join("main.ds");
    test.update_file(&file_b, "export const b = 2;");
    let file_b_id = test
        .session
        .files
        .get_id_by_path(&file_b)
        .expect("missing file id for root b");

    // update root a and collect the daemon updates
    let updates = test.update_file(&file_a, "export const a = 1;");

    // assertion block
    assert!(updates.iter().all(|update| update.file_id != file_b_id));
}
