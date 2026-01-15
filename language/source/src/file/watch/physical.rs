use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, TrySendError, after, bounded, select};
use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::{
    FileWatchCommand, FileWatchCommandReceiver, FileWatchEvent, FileWatchEventKind,
    FileWatchFilter, FileWatchOptions, FileWatchRescanReason, FileWatchSender, FileWatchStatus,
    FileWatchStatusSender, FileWatchSubscription, FileWatchUpdate, FileWatcher,
};

/// File watcher backed by the host operating system.
#[derive(Debug, Default, Clone)]
pub struct PhysicalFileWatcher;

impl PhysicalFileWatcher {
    /// Create a new PhysicalFileWatcher.
    pub fn new() -> Self {
        Self
    }
}

impl FileWatcher for PhysicalFileWatcher {
    fn watch(&self, roots: Vec<PathBuf>, options: FileWatchOptions) -> FileWatchSubscription {
        // setup channels
        let (sender, receiver) = bounded(options.channel_capacity);
        let (status_sender, status_receiver) = bounded(options.status_channel_capacity);
        let (command_sender, command_receiver) = bounded(options.command_channel_capacity);

        // build initial watcher state
        let overflowed = Arc::new(AtomicBool::new(false));
        let Some(state) = build_state(roots, options, Arc::clone(&overflowed), &status_sender)
        else {
            send_status(
                &status_sender,
                FileWatchStatus::Error {
                    message: "failed to initialize watcher".to_string(),
                },
            );
            send_status(&status_sender, FileWatchStatus::Stopped);
            return FileWatchSubscription::new(receiver, status_receiver, command_sender);
        };

        // emit startup status
        send_ready(&status_sender, &state.roots, FileWatchRescanReason::Startup);

        // spawn forwarding loop
        thread::spawn(move || {
            watch_loop(state, sender, status_sender, command_receiver, overflowed);
        });

        FileWatchSubscription::new(receiver, status_receiver, command_sender)
    }
}

/// State for a physical watcher loop.
struct WatchState {
    /// Watched roots.
    roots: Vec<PathBuf>,
    /// Watch options for filtering and config.
    options: FileWatchOptions,
    /// Receiver for raw notify events.
    raw_receiver: Receiver<Event>,
    /// Underlying notify watcher.
    _watcher: RecommendedWatcher,
}

/// Result of receiving from watch loop channels.
enum WatchLoopInput {
    /// A raw notify event was received.
    Event(Event),
    /// A watch command was received.
    Command(FileWatchCommand),
    /// A receiver disconnected.
    Disconnected,
    /// The debounce timeout elapsed.
    Timeout,
}

/// Build a watch state from roots and options.
fn build_state(
    roots: Vec<PathBuf>,
    options: FileWatchOptions,
    overflowed: Arc<AtomicBool>,
    status_sender: &FileWatchStatusSender,
) -> Option<WatchState> {
    // reset overflow state
    overflowed.store(false, Ordering::Relaxed);

    // setup raw channel
    let (raw_sender, raw_receiver) = bounded(options.raw_channel_capacity);

    // configure notify watcher
    let mut config = Config::default();
    if let Some(poll_interval) = options.poll_interval {
        config = config.with_poll_interval(poll_interval);
    }

    // create notify watcher
    let callback_overflowed = Arc::clone(&overflowed);
    let callback_sender = raw_sender.clone();
    let callback_status = status_sender.clone();
    let init_status = status_sender.clone();
    let watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| match result {
            Ok(event) => {
                if callback_sender.try_send(event).is_err() {
                    callback_overflowed.store(true, Ordering::Relaxed);
                }
            }
            Err(error) => {
                callback_overflowed.store(true, Ordering::Relaxed);
                send_status(
                    &callback_status,
                    FileWatchStatus::Error {
                        message: error.to_string(),
                    },
                );
            }
        },
        config,
    )
    .map_err(|error| {
        send_status(
            &init_status,
            FileWatchStatus::Error {
                message: error.to_string(),
            },
        );
    })
    .ok()?;

    // watch each root
    let recursive_mode = if options.recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    let mut watcher = watcher;
    for root in roots.iter() {
        if let Err(error) = watcher.watch(root, recursive_mode) {
            overflowed.store(true, Ordering::Relaxed);
            send_status(
                status_sender,
                FileWatchStatus::Error {
                    message: error.to_string(),
                },
            );
        }
    }

    Some(WatchState {
        roots,
        options,
        raw_receiver,
        _watcher: watcher,
    })
}

/// Run the watch loop.
fn watch_loop(
    mut state: WatchState,
    sender: FileWatchSender,
    status_sender: FileWatchStatusSender,
    command_receiver: FileWatchCommandReceiver,
    overflowed: Arc<AtomicBool>,
) {
    // listen for events until stopped
    'outer: loop {
        // route loop input
        match recv_input(&state.raw_receiver, &command_receiver) {
            WatchLoopInput::Event(event) => {
                // collect events for this batch
                let mut pending = vec![event];
                if state.options.debounce != Duration::ZERO {
                    // collect until debounce deadline
                    let deadline = Instant::now() + state.options.debounce;
                    loop {
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        if remaining.is_zero() {
                            break;
                        }
                        match recv_input_with_timeout(
                            &state.raw_receiver,
                            &command_receiver,
                            remaining,
                        ) {
                            WatchLoopInput::Event(event) => pending.push(event),
                            WatchLoopInput::Command(command) => {
                                if handle_command(command, &mut state, &status_sender, &overflowed)
                                {
                                    break 'outer;
                                }
                                continue 'outer;
                            }
                            WatchLoopInput::Disconnected => break 'outer,
                            WatchLoopInput::Timeout => break,
                        }
                    }
                }

                // build watch events from notify events
                let mut watch_events = Vec::new();
                for raw_event in pending {
                    collect_notify_events(raw_event, &mut watch_events);
                }

                // dedupe while preserving order
                let mut seen = HashSet::new();
                let mut deduped = Vec::new();
                for event in watch_events {
                    if seen.insert(event.clone()) {
                        deduped.push(event);
                    }
                }

                // emit overflow before the batch if needed
                emit_overflow_if_needed(&sender, &status_sender, &state.roots, &overflowed);

                // send deduped events
                for event in deduped {
                    send_event(&sender, &overflowed, &state, event);
                }

                // emit overflow after the batch if events were dropped
                emit_overflow_if_needed(&sender, &status_sender, &state.roots, &overflowed);
            }
            WatchLoopInput::Command(command) => {
                if handle_command(command, &mut state, &status_sender, &overflowed) {
                    break 'outer;
                }
            }
            WatchLoopInput::Disconnected => break 'outer,
            WatchLoopInput::Timeout => {}
        }
    }

    // emit stopped status
    send_status(&status_sender, FileWatchStatus::Stopped);
}

/// Receive input from the raw and command channels.
fn recv_input(
    raw_receiver: &Receiver<Event>,
    command_receiver: &FileWatchCommandReceiver,
) -> WatchLoopInput {
    // wait for event or command
    select! {
        recv(command_receiver) -> command => match command {
            Ok(command) => WatchLoopInput::Command(command),
            Err(_) => WatchLoopInput::Command(FileWatchCommand::Stop),
        },
        recv(raw_receiver) -> event => match event {
            Ok(event) => WatchLoopInput::Event(event),
            Err(_) => WatchLoopInput::Disconnected,
        },
    }
}

/// Receive input with a timeout for debounce.
fn recv_input_with_timeout(
    raw_receiver: &Receiver<Event>,
    command_receiver: &FileWatchCommandReceiver,
    timeout: Duration,
) -> WatchLoopInput {
    // create timeout channel
    let timeout_receiver = after(timeout);

    // wait for event, command, or timeout
    select! {
        recv(command_receiver) -> command => match command {
            Ok(command) => WatchLoopInput::Command(command),
            Err(_) => WatchLoopInput::Command(FileWatchCommand::Stop),
        },
        recv(raw_receiver) -> event => match event {
            Ok(event) => WatchLoopInput::Event(event),
            Err(_) => WatchLoopInput::Disconnected,
        },
        recv(timeout_receiver) -> _ => WatchLoopInput::Timeout,
    }
}

/// Handle a watch command.
fn handle_command(
    command: FileWatchCommand,
    state: &mut WatchState,
    status_sender: &FileWatchStatusSender,
    overflowed: &Arc<AtomicBool>,
) -> bool {
    // interpret command
    match command {
        FileWatchCommand::Stop => true,
        FileWatchCommand::Rescan => {
            // emit notice for manual rescan
            send_status(
                status_sender,
                FileWatchStatus::RescanRequested {
                    roots: state.roots.clone(),
                    reason: FileWatchRescanReason::Manual,
                },
            );
            false
        }
        FileWatchCommand::Update(update) => {
            // rebuild watcher state with updates
            if let Some(new_state) =
                apply_update(state, update, Arc::clone(overflowed), status_sender)
            {
                *state = new_state;
                send_ready(status_sender, &state.roots, FileWatchRescanReason::Update);
            }
            false
        }
    }
}

/// Apply a watcher update to the state.
fn apply_update(
    state: &WatchState,
    update: FileWatchUpdate,
    overflowed: Arc<AtomicBool>,
    status_sender: &FileWatchStatusSender,
) -> Option<WatchState> {
    // compute updated roots and options
    let roots = update.roots.unwrap_or_else(|| state.roots.clone());
    let options = update.options.unwrap_or_else(|| state.options.clone());

    build_state(roots, options, overflowed, status_sender)
}

/// Emit startup or update readiness and rescan hints.
fn send_ready(
    status_sender: &FileWatchStatusSender,
    roots: &[PathBuf],
    reason: FileWatchRescanReason,
) {
    send_status(
        status_sender,
        FileWatchStatus::Ready {
            roots: roots.to_vec(),
        },
    );
    send_status(
        status_sender,
        FileWatchStatus::RescanRequested {
            roots: roots.to_vec(),
            reason,
        },
    );
}

/// Send a status update without blocking.
fn send_status(status_sender: &FileWatchStatusSender, status: FileWatchStatus) {
    let _ = status_sender.try_send(status);
}

/// Collect file watch events from a notify event.
fn collect_notify_events(event: Event, output: &mut Vec<FileWatchEvent>) {
    // handle rename events with from and to paths
    if matches!(event.kind, EventKind::Modify(ModifyKind::Name(_))) {
        let mut paths = event.paths.into_iter();
        let Some(from) = paths.next() else {
            return;
        };
        let to = paths.next();
        let (path, previous_path) = match to {
            Some(to) => (to, Some(from)),
            None => (from, None),
        };
        output.push(FileWatchEvent {
            path,
            previous_path,
            kind: FileWatchEventKind::Renamed,
        });

        // emit extra paths as modified
        for path in paths {
            output.push(FileWatchEvent {
                path,
                previous_path: None,
                kind: FileWatchEventKind::Modified,
            });
        }
        return;
    }

    // map remaining event kinds
    let kind = map_event_kind(&event.kind);
    for path in event.paths {
        output.push(FileWatchEvent {
            path,
            previous_path: None,
            kind: kind.clone(),
        });
    }
}

/// Map notify event kinds to file watch event kinds.
fn map_event_kind(kind: &EventKind) -> FileWatchEventKind {
    // classify event kind
    match kind {
        EventKind::Create(_) => FileWatchEventKind::Created,
        EventKind::Modify(ModifyKind::Name(_)) => FileWatchEventKind::Renamed,
        EventKind::Modify(_) => FileWatchEventKind::Modified,
        EventKind::Remove(_) => FileWatchEventKind::Deleted,
        EventKind::Access(_) => FileWatchEventKind::Modified,
        EventKind::Other => FileWatchEventKind::Modified,
        EventKind::Any => FileWatchEventKind::Modified,
    }
}

/// Send a watch event if it passes filters.
fn send_event(
    sender: &FileWatchSender,
    overflowed: &AtomicBool,
    state: &WatchState,
    event: FileWatchEvent,
) {
    // skip events rejected by the filter
    if !should_emit_event(state, &event) {
        return;
    }

    // send event without blocking
    match sender.try_send(event) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            overflowed.store(true, Ordering::Relaxed);
        }
        Err(TrySendError::Disconnected(_)) => {}
    }
}

/// Check whether a watch event should be emitted for a filter and roots.
fn should_emit_event(state: &WatchState, event: &FileWatchEvent) -> bool {
    // allow overflow events unconditionally
    if event.kind == FileWatchEventKind::Overflow {
        return true;
    }

    // accept matches on the current path
    if matches_roots(&state.roots, &event.path)
        && should_emit_filtered(state.options.filter.as_ref(), &event.path)
    {
        return true;
    }

    // accept matches on the previous path
    event.previous_path.as_ref().is_some_and(|path| {
        matches_roots(&state.roots, path)
            && should_emit_filtered(state.options.filter.as_ref(), path)
    })
}

/// Check whether a path is accepted by roots.
fn matches_roots(roots: &[PathBuf], path: &Path) -> bool {
    // allow all paths when roots are empty
    if roots.is_empty() {
        return true;
    }

    roots.iter().any(|root| path.starts_with(root))
}

/// Check whether a path is accepted by a filter.
fn should_emit_filtered(filter: Option<&FileWatchFilter>, path: &Path) -> bool {
    // apply filter when present
    let Some(filter) = filter else {
        return true;
    };

    filter(path)
}

/// Emit overflow events when the overflow flag is set.
fn emit_overflow_if_needed(
    sender: &FileWatchSender,
    status_sender: &FileWatchStatusSender,
    roots: &[PathBuf],
    overflowed: &AtomicBool,
) {
    // check overflow flag
    if !overflowed.swap(false, Ordering::Relaxed) {
        return;
    }

    // emit overflow per root
    for root in roots {
        let event = FileWatchEvent {
            path: root.clone(),
            previous_path: None,
            kind: FileWatchEventKind::Overflow,
        };
        match sender.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                overflowed.store(true, Ordering::Relaxed);
                break;
            }
            Err(TrySendError::Disconnected(_)) => break,
        }
    }

    // request rescan after overflow
    send_status(
        status_sender,
        FileWatchStatus::RescanRequested {
            roots: roots.to_vec(),
            reason: FileWatchRescanReason::Overflow,
        },
    );
}
