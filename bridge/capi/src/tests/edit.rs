use std::ptr;

use crate::{
    DestackCommit, destack_commit_destroy, destack_edits_destroy, destack_edits_new,
    destack_session_edit,
};

use super::session::{TestSession, add_edit_text, assert_ok, string};

#[test]
fn test_edit_source() {
    let session = TestSession::open(&[
        ("destack.json", r#"{"name":"@test/app"}"#),
        ("src/index.ds", "export const value = 1;"),
    ]);
    let mut error = ptr::null_mut();
    let edits = destack_edits_new();
    let mut commit = DestackCommit::empty();

    // build one edit list
    add_edit_text(edits, "src/next.ds", "export const next = 2;");

    // apply edit through the C ABI
    let status = unsafe { destack_session_edit(session.inner, edits, &mut commit, &mut error) };
    assert_ok(status, error);

    // require a changed revision and file list
    assert_ne!(string(commit.before.id), string(commit.after.id));
    assert_eq!(
        session.paths(),
        ["destack.json", "src/index.ds", "src/next.ds"]
    );

    // release returned and input values
    unsafe {
        destack_commit_destroy(&mut commit);
        destack_edits_destroy(edits);
    }
}
