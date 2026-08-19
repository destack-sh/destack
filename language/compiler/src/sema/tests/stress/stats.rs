use std::collections::BTreeMap;

use crate::tests::TestSession;
use crate::tests::snapshot::assert_snapshot;

/// Assert the full check stats sidecar for one generated main module.
#[track_caller]
pub(super) fn assert_check_stats(source: &str, expected: &str) {
    let session = TestSession::single(source);
    let metadata = session.artifact_text_sidecar(
        session.dir_checked_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "check".to_string())]),
    );

    assert_snapshot(metadata, expected);
}
