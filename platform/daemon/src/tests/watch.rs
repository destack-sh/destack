use std::time::{Duration, Instant};

use destack_source::{FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};

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

    // check that the batch captures events without overflow
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

    // check that the overflow is flagged
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
    let result = harness.apply_batch(&batch);

    // check that the daemon updates include diagnostics
    assert!(
        result
            .updates
            .iter()
            .any(|update| !update.diagnostics.is_empty()),
        "expected diagnostics for watched update"
    );

    harness.stop();
}

/// Requests rescan when configuration files change.
#[test]
fn test_watch_batch_requests_rescan_for_config() {
    let policy = WatchPolicy {
        coalesce_window: Duration::from_millis(50),
        max_batch_size: 8,
    };
    let harness = TestWatchHarness::new(policy);

    harness.skip_startup();

    harness
        .test
        .write_text("dsconfig.json", "{ \"compilerOptions\": {} }");
    harness.emit("dsconfig.json", FileWatchEventKind::Modified);

    let batch = harness.next_batch();
    let result = harness.apply_batch(&batch);

    // check that the rescan is requested for config updates
    assert!(!result.updated());
    assert!(result.rescan);

    harness.stop();
}

/// Surfaces rescan status updates from watchers.
#[test]
fn test_watch_batch_handles_status_rescan() {
    let harness = TestWatchHarness::new(WatchPolicy::default());

    let mut batch = crate::WatchBatch::new(Instant::now());
    batch.status.push(FileWatchStatus::RescanRequested {
        roots: vec![],
        reason: FileWatchRescanReason::Manual,
    });
    batch.ended_at = Instant::now();

    let result = harness.test.apply_watch_batch(&batch);

    // check that the status rescan is surfaced without updates
    assert!(!result.updated());
    assert!(result.rescan);
    assert!(!result.messages.is_empty());

    harness.stop();
}
