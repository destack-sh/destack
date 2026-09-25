use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tspp_artifact::BuildId;
use tspp_repository::{DestackLayoutOverride, Environment, Execution, Host, Repository, Settings};
use tspp_session::Executor;
use tspp_source::{FileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem};
use tspp_workspace::Workspace;

use crate::{
    Daemon, DaemonConnectOptions, DaemonConnection, DaemonEndpoint, DaemonError, DaemonOptions,
};

/// Isolated running daemon for RPC tests.
pub(super) struct TestDaemon {
    /// Temporary repository owner.
    root: TemporaryPhysicalFileSystem,
    /// Daemon discovery endpoint.
    pub endpoint: DaemonEndpoint,
    /// Running daemon thread.
    handle: Option<JoinHandle<Result<(), DaemonError>>>,
}

impl TestDaemon {
    /// Start one isolated daemon.
    pub(super) fn start(name: &str) -> Self {
        let root = TemporaryPhysicalFileSystem::new_with_prefix(name);
        root.write_text(
            "package.json",
            "{ \"packageManager\": \"tspp@2026.9.0\", \"name\": \"test\" }\n",
        )
        .expect("test manifest should write");
        let workspace = Self::workspace(&root);
        let repository = workspace.session().repository();
        let endpoint = DaemonEndpoint::new(repository.layout().home.clone());
        let options = DaemonOptions {
            idle_timeout: None,
            ..DaemonOptions::default()
        };
        let daemon = Daemon::with_options(workspace, endpoint.clone(), options)
            .expect("test daemon should initialize");
        let handle = thread::spawn(move || daemon.serve());

        Self {
            root,
            endpoint,
            handle: Some(handle),
        }
    }

    /// Return the repository root.
    pub(super) fn root(&self) -> &Path {
        self.root.root()
    }

    /// Write one physical source file beneath the repository root.
    pub(super) fn write_text(&self, path: &str, content: &str) -> PathBuf {
        self.root
            .write_text(path, content)
            .expect("physical source should write")
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
            thread::sleep(Duration::from_millis(10));
        }

        panic!("daemon did not accept RPC connections: {last_error:?}")
    }

    /// Build one daemon workspace over a temporary physical root.
    fn workspace(root: &TemporaryPhysicalFileSystem) -> Workspace {
        let mut environment = Environment::capture_process();
        environment.cwd = Some(root.root().to_path_buf());
        let settings = Settings::default();
        let layout = DestackLayoutOverride {
            home: Some(root.root().join("home")),
            ..DestackLayoutOverride::default()
        };
        let physical: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let host = Host::new(BuildId::test(), environment, physical);
        let (repository, physical) =
            Repository::open(root.root().to_path_buf(), host, settings, layout)
                .expect("test repository should open");
        let repository = Arc::new(repository);
        let executor = Executor::new(Execution::Threaded, 1).expect("create executor");

        Workspace::new(repository, physical, executor).expect("test workspace should open")
    }

    /// Request shutdown and join the daemon.
    pub(super) fn shutdown(mut self, connection: DaemonConnection) {
        connection
            .daemon()
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
            let _shutdown = connection.daemon().shutdown(());
            let _closed = connection.close();
        }
        let _joined = handle.join();
    }
}
