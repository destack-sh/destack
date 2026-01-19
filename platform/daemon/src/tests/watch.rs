use std::time::Duration;

use destack_source::FileWatchEventKind;

use crate::WatchPolicy;
use crate::tests::TestWatchHarness;

/// Coalesces multiple watch events into one batch.
#[test]
fn test_watch_coalesces_events() {
    let policy = WatchPolicy {
        coalesce_window: Duration::from_millis(50),
        max_batch_size: 16,
    };
    let harness = TestWatchHarness::new(policy);

    harness.skip_startup();
    harness.emit("source/a.ds", FileWatchEventKind::Modified);
    harness.emit("source/b.ds", FileWatchEventKind::Created);

    let batch = harness.next_batch();

    // assertion block: batch captures events without overflow
    assert_eq!(batch.event_count(), 2);
    assert!(!batch.has_status(), "expected no status updates");
    batch.assert_event_suffix("a.ds");
    batch.assert_event_suffix("b.ds");
    assert!(!batch.overflowed(), "expected no overflow");

    harness.stop();
}

/// Marks overflow when the watch stream reports it.
#[test]
fn test_watch_marks_overflow() {
    let policy = WatchPolicy {
        coalesce_window: Duration::from_millis(50),
        max_batch_size: 8,
    };
    let harness = TestWatchHarness::new(policy);

    harness.skip_startup();
    harness.emit("source/overflow.ds", FileWatchEventKind::Overflow);

    let batch = harness.next_batch();

    // assertion block: overflow is flagged
    assert_eq!(batch.event_count(), 1);
    assert!(batch.overflowed(), "expected overflow to be flagged");

    harness.stop();
}

/// Drives daemon updates from watch events.
#[test]
fn test_watch_batch_updates_daemon() {
    let policy = WatchPolicy {
        coalesce_window: Duration::from_millis(50),
        max_batch_size: 8,
    };
    let harness = TestWatchHarness::new(policy);

    harness.skip_startup();

    harness
        .test
        .write_text("watched.ds", "export const value = ;");
    harness.emit("watched.ds", FileWatchEventKind::Modified);

    let batch = harness.next_batch();
    let updates = harness.apply_batch(&batch);

    // assertion block: daemon updates include diagnostics
    assert_eq!(updates.len(), 1);
    assert!(
        !updates[0].diagnostics.is_empty(),
        "expected diagnostics for watched update"
    );

    harness.stop();
}
