use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use destack_artifact::MemoryCacheStore;
use destack_session::open_repository_from_fs;
use destack_source::{
    FileId, FileSystem, FileWatchEvent, FileWatchEventKind, FileWatchOptions, MemoryFileSystem,
    MemoryFileWatcher,
};
use destack_workspace::{DestackLayoutOverride, Ref, Repository, Revision, Settings};

use crate::protocol::{
    DaemonRequest, DaemonResponse, OpenRootRequest, ProtocolClient, ProtocolClientOptions,
    ProtocolErrorCode, ProtocolServer, ProtocolServerError, ProtocolServerOptions, RootHandleId,
    RootOpenOptions, loopback_transport_pair,
};
use crate::{
    Daemon, DaemonUpdate, DaemonUpdateResult, DaemonWorkspace, WatchBatch, WatchCoordinator,
    WatchPolicy,
};

/// Test harness for daemon flows.
#[derive(Debug, Clone)]
pub struct TestDaemon {
    /// The in memory file system.
    pub fs: Arc<MemoryFileSystem>,
    /// The in memory file watcher.
    pub watcher: Arc<MemoryFileWatcher>,
    /// The shared repository.
    pub repository: Arc<Repository>,
    /// The daemon under test.
    pub daemon: Daemon,
    /// The primary root.
    pub root: PathBuf,
    /// The roots for the daemon.
    roots: Vec<PathBuf>,
}

/// Harness for watch coordination in tests.
#[derive(Debug)]
pub struct TestWatchHarness {
    /// The daemon test state.
    pub test: TestDaemon,
    /// The watch coordinator under test.
    coordinator: WatchCoordinator,
}

/// Wrapper for watch batches in tests.
#[derive(Debug)]
pub struct TestWatchBatch {
    /// The captured watch batch.
    batch: WatchBatch,
}

/// Harness for daemon protocol server/client.
#[derive(Debug)]
pub struct TestProtocolHarness {
    /// The daemon test state.
    pub test: TestDaemon,
    /// The protocol client.
    pub client: ProtocolClient,
    /// Captured server error, if any.
    server_error: Arc<Mutex<Option<String>>>,
    /// Server thread handle.
    server_handle: Option<JoinHandle<Result<(), ProtocolServerError>>>,
}

/// Describe retry behavior for protocol requests.
#[derive(Debug, Clone, Copy)]
pub struct RequestRetryPolicy {
    /// The maximum number of attempts.
    pub max_attempts: usize,
    /// The initial delay in milliseconds.
    pub base_delay_ms: u64,
    /// The incremental delay added per attempt in milliseconds.
    pub backoff_step_ms: u64,
}

impl Default for RequestRetryPolicy {
    /// Return a default retry policy for protocol requests.
    fn default() -> Self {
        Self {
            max_attempts: 25,
            base_delay_ms: 10,
            backoff_step_ms: 5,
        }
    }
}

impl TestDaemon {
    /// Create a test daemon with a default root.
    pub fn new() -> Self {
        Self::new_with_roots(vec![PathBuf::from("/root")])
    }

    /// Create a test daemon with explicit roots.
    pub fn new_with_roots(mut roots: Vec<PathBuf>) -> Self {
        // ensure we have a primary root
        let root = roots
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("/root"));
        if roots.is_empty() {
            roots.push(root.clone());
        }

        // materialize root directories for canonicalization
        let fs = Arc::new(MemoryFileSystem::new());
        for root_path in &roots {
            let _ = fs.create_dir_all(root_path);
        }

        // build shared state
        let watcher = Arc::new(MemoryFileWatcher::new());
        let workspace_root = if roots.len() == 1 {
            root.clone()
        } else {
            common_workspace_root(&roots)
        };
        let repository = Arc::new(
            open_repository_from_fs(
                workspace_root,
                fs.clone(),
                destack_workspace::Environment::capture_process(),
                Settings::default(),
                DestackLayoutOverride::default(),
            )
            .expect("failed to import repository from test file system")
            .with_cache(Arc::new(MemoryCacheStore::new())),
        );

        let daemon = Daemon::new_with_watcher(repository.clone(), 1, None, watcher.clone())
            .expect("daemon should initialize");

        Self {
            fs,
            watcher,
            repository,
            daemon,
            root,
            roots,
        }
    }

    /// Return the primary root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a path relative to the primary root when needed.
    pub fn path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        resolve_test_path(&self.root, path)
    }

    /// Return the primary daemon workspace.
    pub fn workspace(&self) -> Arc<DaemonWorkspace> {
        self.daemon
            .workspace(self.repository.workspace_root())
            .expect("test workspace should be opened")
    }

    /// Write a text file into the test file system.
    pub fn write_text(&self, path: impl AsRef<Path>, contents: &str) -> PathBuf {
        let path = self.path_for(path);
        if let Some(parent) = path.parent() {
            let _ = self.fs.create_dir_all(parent);
        }
        self.fs
            .write(&path, contents.as_bytes())
            .unwrap_or_else(|error| panic!("write failed for {}: {error}", path.display()));
        path
    }

    /// Update a file and return all daemon updates.
    pub fn update_file(&self, path: impl AsRef<Path>, content: &str) -> Vec<DaemonUpdate> {
        let path = self.path_for(path);
        self.workspace()
            .update_file(&path, content.to_string())
            .unwrap_or_else(|error| panic!("update failed for {}: {error}", path.display()))
            .updates
    }

    /// Update an in-memory file and return all daemon updates.
    pub fn update_memory_file(&self, path: impl AsRef<Path>, content: &str) -> Vec<DaemonUpdate> {
        let path = self.path_for(path);
        self.workspace()
            .update_memory_file(&path, content.to_string())
            .unwrap_or_else(|error| panic!("virtual update failed for {}: {error}", path.display()))
            .updates
    }

    /// Resolve the tracked file id for a path.
    pub fn file_id_for_path(&self, path: impl AsRef<Path>) -> FileId {
        let path = self.path_for(path);
        let view = self
            .workspace()
            .language_service
            .file_view(&path)
            .unwrap_or_else(|error| panic!("missing file view for {}: {error}", path.display()));

        view.file_id
    }

    /// Return the current revision scoped file snapshot for a path.
    pub fn file_for_path(&self, path: impl AsRef<Path>) -> Arc<destack_source::File> {
        let path = self.path_for(path);
        let view = self
            .workspace()
            .language_service
            .file_view(&path)
            .unwrap_or_else(|error| panic!("missing file view for {}: {error}", path.display()));

        view.file
    }

    /// Return the current revision scoped module id for a path.
    pub fn module_id_for_path(&self, path: impl AsRef<Path>) -> destack_source::ModuleId {
        let path = self.path_for(path);
        let view = self
            .workspace()
            .language_service
            .file_view(&path)
            .unwrap_or_else(|error| panic!("missing file view for {}: {error}", path.display()));
        let repository = view.repository();

        repository
            .module_id_for_file(view.revision(), view.file_id)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to resolve module id for '{}' in revision: {error}",
                    path.display()
                )
            })
            .unwrap_or_else(|| panic!("missing module for {}", path.display()))
    }

    /// Return the update for a specific file id.
    pub fn update_for_file_id<'a>(
        &self,
        updates: &'a [DaemonUpdate],
        file_id: FileId,
    ) -> &'a DaemonUpdate {
        updates
            .iter()
            .find(|update| update.file_id == file_id)
            .unwrap_or_else(|| panic!("missing update for file id {file_id}"))
    }

    /// Return the update for a specific path.
    pub fn update_for_path<'a>(
        &self,
        updates: &'a [DaemonUpdate],
        path: impl AsRef<Path>,
    ) -> &'a DaemonUpdate {
        let file_id = self.file_id_for_path(path);
        self.update_for_file_id(updates, file_id)
    }

    /// Update a file and return the update for the target file.
    pub fn update_file_for_path(&self, path: impl AsRef<Path>, content: &str) -> DaemonUpdate {
        let path = self.path_for(path);
        let updates = self.update_file(&path, content);
        self.update_for_path(&updates, &path).clone()
    }

    /// Build a watch coordinator for the test roots.
    pub fn watch_coordinator(&self, policy: WatchPolicy) -> WatchCoordinator {
        WatchCoordinator::new(
            self.watcher.clone(),
            self.roots.clone(),
            FileWatchOptions::default(),
            policy,
        )
    }

    /// Build a watch event for tests.
    pub fn watch_event(&self, path: impl AsRef<Path>, kind: FileWatchEventKind) -> FileWatchEvent {
        FileWatchEvent {
            path: self.path_for(path),
            previous_path: None,
            kind,
        }
    }

    /// Build a rename watch event for tests.
    pub fn watch_rename_event(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> FileWatchEvent {
        FileWatchEvent {
            path: self.path_for(to),
            previous_path: Some(self.path_for(from)),
            kind: FileWatchEventKind::Renamed,
        }
    }

    /// Apply a watch batch to the daemon and return the batch result.
    pub fn apply_watch_batch(&self, batch: &WatchBatch) -> DaemonUpdateResult {
        self.workspace().apply_watch_batch(batch)
    }

    /// Build a protocol harness for this daemon.
    pub fn protocol(&self) -> TestProtocolHarness {
        TestProtocolHarness::from_test(self.clone())
    }

    /// Build a protocol harness with custom server options.
    pub fn protocol_with_options(&self, options: ProtocolServerOptions) -> TestProtocolHarness {
        TestProtocolHarness::from_test_with_options(self.clone(), options)
    }
}

/// Return the current root revision for one repository.
pub fn current_root_revision(repository: &Repository) -> Revision {
    // resolve the root ref first
    let reference = Ref::for_workspace_root(repository.workspace_root());

    // return the current published root revision
    repository
        .current(&reference)
        .expect("expected current root revision")
}

/// Return the shallowest common root for the provided paths.
fn common_workspace_root(paths: &[PathBuf]) -> PathBuf {
    let mut components = paths[0]
        .components()
        .map(|component| component.as_os_str().to_owned())
        .collect::<Vec<_>>();

    for path in &paths[1..] {
        let path_components = path
            .components()
            .map(|component| component.as_os_str().to_owned())
            .collect::<Vec<_>>();
        let shared_len = components
            .iter()
            .zip(path_components.iter())
            .take_while(|(left, right)| left == right)
            .count();
        components.truncate(shared_len);
    }

    let mut root = PathBuf::new();
    for component in components {
        root.push(component);
    }

    root
}

impl Default for TestDaemon {
    /// Return a test daemon with a default root.
    fn default() -> Self {
        Self::new()
    }
}

impl TestWatchHarness {
    /// Create a watch harness with the provided policy.
    pub fn new(policy: WatchPolicy) -> Self {
        let test = TestDaemon::new();
        let coordinator = test.watch_coordinator(policy);
        Self { test, coordinator }
    }

    /// Consume the initial watch batch.
    pub fn skip_startup(&self) {
        let _ = self
            .coordinator
            .next_batch()
            .expect("expected startup batch");
    }

    /// Emit a watch event.
    pub fn emit(&self, path: impl AsRef<Path>, kind: FileWatchEventKind) {
        let event = self.test.watch_event(path, kind);
        self.test.watcher.emit(event);
    }

    /// Emit a rename watch event.
    pub fn emit_rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) {
        let event = self.test.watch_rename_event(from, to);
        self.test.watcher.emit(event);
    }

    /// Return the next watch batch.
    pub fn next_batch(&self) -> TestWatchBatch {
        let batch = self.coordinator.next_batch().expect("expected watch batch");
        TestWatchBatch::new(batch)
    }

    /// Apply a watch batch and return the batch result.
    pub fn apply_batch(&self, batch: &TestWatchBatch) -> DaemonUpdateResult {
        self.test.apply_watch_batch(batch.batch())
    }

    /// Stop the watch coordinator.
    pub fn stop(&self) {
        self.coordinator.stop();
    }
}

impl TestWatchBatch {
    /// Wrap a watch batch for test assertions.
    pub fn new(batch: WatchBatch) -> Self {
        Self { batch }
    }

    /// Return the number of events in the batch.
    pub fn event_count(&self) -> usize {
        self.batch.events.len()
    }

    /// Return true when the batch has any status updates.
    pub fn has_status(&self) -> bool {
        !self.batch.status.is_empty()
    }

    /// Return true when the batch overflowed.
    pub fn overflowed(&self) -> bool {
        self.batch.overflowed
    }

    /// Return a reference to the underlying batch.
    pub fn batch(&self) -> &WatchBatch {
        &self.batch
    }

    /// Assert the batch contains a path suffix.
    pub fn assert_event_suffix(&self, suffix: &str) {
        assert!(
            self.batch
                .events
                .iter()
                .any(|event| event.path.ends_with(suffix)),
            "expected {suffix} to be present"
        );
    }
}

impl TestProtocolHarness {
    /// Create a protocol harness with a default daemon.
    pub fn new() -> Self {
        Self::from_test(TestDaemon::new())
    }

    /// Create a protocol harness from an existing test daemon.
    pub fn from_test(test: TestDaemon) -> Self {
        Self::from_test_with_options(test, ProtocolServerOptions::default())
    }

    /// Create a protocol harness with explicit server options.
    pub fn from_test_with_options(test: TestDaemon, options: ProtocolServerOptions) -> Self {
        // create loopback transports
        let (client_transport, server_transport) = loopback_transport_pair(16);
        let daemon = Arc::new(test.daemon.clone());

        // track server errors for debugging
        let server_error = Arc::new(Mutex::new(None));
        let error_handle = server_error.clone();

        // start the protocol server
        let server = ProtocolServer::with_options(daemon, options);
        let server_handle = thread::spawn(move || {
            // run the server loop
            let result = server.serve(&server_transport);

            // capture errors for the harness
            if let Err(error) = &result
                && let Ok(mut slot) = error_handle.lock()
            {
                *slot = Some(error.to_string());
            }
            result
        });

        // create protocol client
        let client = ProtocolClient::new(Arc::new(client_transport));

        Self {
            test,
            client,
            server_error,
            server_handle: Some(server_handle),
        }
    }

    /// Perform a handshake and return the response.
    pub fn handshake(&self) -> crate::protocol::HandshakeResponse {
        self.handshake_with(ProtocolClientOptions::default())
    }

    /// Perform a handshake with explicit options.
    pub fn handshake_with(
        &self,
        options: ProtocolClientOptions,
    ) -> crate::protocol::HandshakeResponse {
        self.client.handshake(options).expect("handshake")
    }

    /// Send a daemon request through the protocol client.
    pub fn send_request(&self, request: DaemonRequest) -> DaemonResponse {
        match self.client.send_request(request) {
            Ok(response) => response,
            Err(error) => {
                let server_error = self.server_error.lock().ok().and_then(|slot| slot.clone());
                let message = match server_error {
                    Some(server_error) => format!("request: {error} (server: {server_error})"),
                    None => format!("request: {error}"),
                };
                panic!("{message}");
            }
        }
    }

    /// Send a daemon request, retrying on NotReady responses.
    pub fn send_request_with_retry<F>(
        &self,
        mut build: F,
        policy: RequestRetryPolicy,
    ) -> DaemonResponse
    where
        F: FnMut() -> DaemonRequest,
    {
        for attempt in 0..policy.max_attempts {
            let response = self.send_request(build());
            let should_retry = match &response {
                DaemonResponse::Error(error) => error.code == ProtocolErrorCode::NotReady,
                _ => false,
            };

            if !should_retry {
                return response;
            }

            let delay_ms = policy.base_delay_ms + (attempt as u64 * policy.backoff_step_ms);
            thread::sleep(Duration::from_millis(delay_ms));
        }

        panic!(
            "query state did not become ready after {max_attempts} attempts",
            max_attempts = policy.max_attempts
        );
    }

    /// Open the default root and return the handle id.
    pub fn open_root(&self) -> RootHandleId {
        self.open_root_path(self.test.root.clone())
    }

    /// Open one explicit root and return the handle id.
    pub fn open_root_path(&self, root: PathBuf) -> RootHandleId {
        let open = OpenRootRequest {
            workspace: self.test.repository.workspace_root().to_path_buf(),
            root,
            options: RootOpenOptions::default(),
        };
        match self.send_request(DaemonRequest::OpenRoot(open)) {
            DaemonResponse::RootOpened(response) => response.handle,
            other => panic!("unexpected response: {other:?}"),
        }
    }

    /// Shutdown the server and join the thread.
    pub fn shutdown(mut self) {
        let _ = self.send_request(DaemonRequest::Shutdown);
        if let Some(handle) = self.server_handle.take() {
            handle.join().expect("server join").expect("serve");
        }
    }

    /// Join the server thread without sending a shutdown request.
    pub fn join(mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.join().expect("server join").expect("serve");
        }
    }
}

impl Default for TestProtocolHarness {
    /// Return a protocol harness backed by a default daemon.
    fn default() -> Self {
        Self::new()
    }
}

/// Wait until a predicate evaluates to true within a timeout.
pub fn wait_for_condition(
    timeout: Duration,
    poll_interval: Duration,
    mut condition: impl FnMut() -> bool,
) -> bool {
    // compute the deadline for the condition check
    let deadline = Instant::now() + timeout;

    // poll until the condition passes or the deadline expires
    while Instant::now() < deadline {
        if condition() {
            return true;
        }

        thread::sleep(poll_interval);
    }

    condition()
}

/// Resolve a path relative to the provided root when needed.
fn resolve_test_path(root: &Path, path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
