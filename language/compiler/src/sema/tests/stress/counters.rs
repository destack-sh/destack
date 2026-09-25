use crate::tests::TestSession;
use crate::tests::snapshot::assert_snapshot;

/// Assert the full check counters for one generated main module.
#[track_caller]
pub(super) fn assert_check_counters(source: &str, expected: &str) {
    let session = TestSession::single(source);
    let counters = session.artifact_counters(session.dir_checked_key("main.tspp"), "check.");

    assert_snapshot(counters, expected);
}
