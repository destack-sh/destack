use std::path::PathBuf;

use crate::tests::TestDaemon;

/// Tracks roots independently.
#[test]
fn test_daemon_tracks_root_by_handle() {
    let root_a = PathBuf::from("/root/a");
    let root_b = PathBuf::from("/root/b");
    let test = TestDaemon::new_with_roots(vec![root_a.clone(), root_b.clone()]);

    let file_a = root_a.join("main.ds");
    let file_b = root_b.join("main.ds");
    test.update_file(&file_a, "export const a = 1;");
    test.update_file(&file_b, "export const b = 2;");

    // check that each root has its own live state
    assert_eq!(test.daemon.root_count(), 2);
}

/// Keeps updates isolated to the root that changed.
#[test]
fn test_daemon_updates_do_not_cross_roots() {
    let root_a = PathBuf::from("/root/a");
    let root_b = PathBuf::from("/root/b");
    let test = TestDaemon::new_with_roots(vec![root_a.clone(), root_b.clone()]);

    let file_a = root_a.join("main.ds");
    let file_b = root_b.join("main.ds");
    test.update_file(&file_b, "export const b = 2;");
    let file_b_id = test.file_id_for_path(&file_b);

    // update root a and collect the daemon updates
    let updates = test.update_file(&file_a, "export const a = 1;");

    // assertion block
    assert!(updates.iter().all(|update| update.file_id != file_b_id));
}
