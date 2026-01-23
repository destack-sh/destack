use std::path::PathBuf;
use std::time::{Duration, Instant};

use destack_source::{FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};

use crate::WatchPolicy;
use crate::tests::{TestDaemon, TestWatchBatch, TestWatchHarness};

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

/// Applies a batch across multiple workspace roots.
#[test]
fn test_watch_batch_handles_multiple_roots() {
    let policy = WatchPolicy {
        coalesce_window: Duration::from_millis(50),
        max_batch_size: 8,
    };
    let root_a = PathBuf::from("/workspace/a");
    let root_b = PathBuf::from("/workspace/b");
    let test = TestDaemon::new_with_roots(vec![root_a.clone(), root_b.clone()]);
    let coordinator = test.watch_coordinator(policy);

    let _ = coordinator
        .next_batch()
        .expect("expected startup batch");

    let file_a = root_a.join("a.ds");
    let file_b = root_b.join("b.ds");
    test.write_text(&file_a, "export const a = ;");
    test.write_text(&file_b, "export const b = ;");

    test.watcher
        .emit(test.watch_event(&file_a, FileWatchEventKind::Modified));
    test.watcher
        .emit(test.watch_event(&file_b, FileWatchEventKind::Modified));

    let batch = coordinator
        .next_batch()
        .expect("expected watch batch");
    let batch = TestWatchBatch::new(batch);
    batch.assert_event_suffix("a.ds");
    batch.assert_event_suffix("b.ds");

    let result = test.apply_watch_batch(batch.batch());
    let file_a_id = test
        .session
        .files
        .get_id_by_path(&file_a)
        .expect("missing file id for root a");
    let file_b_id = test
        .session
        .files
        .get_id_by_path(&file_b)
        .expect("missing file id for root b");

    // assertion block
    assert!(result.updates.iter().any(|update| update.file_id == file_a_id));
    assert!(result.updates.iter().any(|update| update.file_id == file_b_id));

    coordinator.stop();
}
