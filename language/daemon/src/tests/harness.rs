use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use destack_artifact::{BuildId, DiskBlobStore};
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Environment, Host, Repository, Settings,
};
use destack_source::{FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem};

use crate::{
    DaemonConnectOptions, DaemonConnection, DaemonEndpoint, DaemonServer, DaemonServerError,
    DaemonServerOptions,
};

/// Isolated running daemon for RPC tests.
pub(super) struct TestDaemon {
    /// Temporary repository owner.
    root: TemporaryPhysicalFileSystem,
    /// Daemon discovery endpoint.
    pub endpoint: DaemonEndpoint,
    /// Running daemon thread.
    handle: Option<JoinHandle<Result<(), DaemonServerError>>>,
}

impl TestDaemon {
    /// Start one isolated daemon.
    pub(super) fn start(name: &str) -> Self {
        let root = TemporaryPhysicalFileSystem::new_with_prefix(name);
        let repository = repository(&root);
        let endpoint = DaemonEndpoint::new(repository.layout().home.clone());
        let options = DaemonServerOptions {
            worker_limit: 1,
            idle_shutdown: None,
            ..DaemonServerOptions::default()
        };
        let server = DaemonServer::with_options(repository, endpoint.clone(), options)
            .expect("daemon server should initialize");
        let handle = std::thread::spawn(move || server.serve());

        Self {
            root,
            endpoint,
            handle: Some(handle),
        }
    }

    /// Return the repository root.
    pub(super) fn root(&self) -> &std::path::Path {
        self.root.root()
    }

    /// Connect after the daemon has bound its socket.
    pub(super) fn connect(&self) -> DaemonConnection {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut last_error = None;

        while Instant::now() < deadline {
            match self.endpoint.connect(DaemonConnectOptions::default(), None) {
                Ok(connection) => return connection,
                Err(error) => last_error = Some(error),
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        panic!("daemon did not accept RPC connections: {last_error:?}")
    }

    /// Request shutdown and join the daemon.
    pub(super) fn shutdown(mut self, connection: DaemonConnection) {
        connection
            .control()
            .shutdown(())
            .expect("daemon shutdown should complete");
        connection.close().expect("connection should close");
        let handle = self.handle.take().expect("daemon thread should exist");
        handle
            .join()
            .expect("daemon thread should not panic")
            .expect("daemon should stop cleanly");
    }
}

impl Drop for TestDaemon {
    /// Stop a daemon left running by a failed test.
    fn drop(&mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };
        if let Ok(connection) = self.endpoint.connect(DaemonConnectOptions::default(), None) {
            let _shutdown = connection.control().shutdown(());
            let _closed = connection.close();
        }
        let _joined = handle.join();
    }
}

/// Build one repository over a temporary physical root.
fn repository(root: &TemporaryPhysicalFileSystem) -> Arc<Repository> {
    let environment = Environment::capture_process();
    let settings = Settings::default();
    let layout = DestackLayout::resolve(
        root.root(),
        root.root(),
        &environment,
        &settings,
        &DestackLayoutOverride {
            home: Some(root.root().join("home")),
            ..DestackLayoutOverride::default()
        },
        None,
    );
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
    let host = Host::new(
        BuildId::test(),
        environment,
        file_system,
        Arc::new(DiskBlobStore::new()),
    );
    let repository = Repository::new(root.root().to_path_buf(), host, settings, layout);

    Arc::new(repository)
}
