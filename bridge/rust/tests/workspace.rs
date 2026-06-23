use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use destack_workspace::{
    CheckInput, Client, ClientOptions, RootOpenOptions, Transport, TransportError,
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

/// Transport that dispatches frames through a local workspace server.
#[derive(Debug)]
struct EmbeddedTransport {
    /// Local server under test.
    server: destack::LocalWorkspaceServer,
    /// Frames ready for the client to receive.
    queue: Mutex<VecDeque<Vec<u8>>>,
}

impl EmbeddedTransport {
    /// Create a transport for one embedded server.
    fn new(server: destack::LocalWorkspaceServer) -> Self {
        Self {
            server,
            queue: Mutex::new(VecDeque::new()),
        }
    }
}

impl Transport for EmbeddedTransport {
    fn send(&self, payload: &[u8]) -> Result<(), TransportError> {
        let responses = self
            .server
            .dispatch(payload)
            .map_err(|error| TransportError::Io(std::io::Error::other(error)))?;
        let mut queue = self.queue.lock();
        queue.extend(responses);

        Ok(())
    }

    fn recv(&self) -> Result<Vec<u8>, TransportError> {
        self.queue.lock().pop_front().ok_or(TransportError::Closed)
    }

    fn close(&self) {
        self.queue.lock().clear();
    }
}

/// Drive a local workspace server through the workspace protocol.
#[test]
fn test_dispatch_runs_workspace_protocol() -> destack::Result<()> {
    let root = create_root("workspace_protocol");
    write_text(&root.join("destack.json"), r#"{"name":"@test/app"}"#);
    write_text(&root.join("src/index.ds"), "export const value = 1;");

    // open an embedded server and protocol client
    let server = destack::LocalWorkspaceServer::open(&root)?;
    let transport = Arc::new(EmbeddedTransport::new(server));
    let client = Client::new(transport);
    client
        .handshake(ClientOptions::default())
        .map_err(destack::Error::new)?;

    // open the project root through the protocol
    let opened = client
        .open_root(root.clone(), RootOpenOptions::default())
        .map_err(destack::Error::new)?;

    // check the project through the protocol client
    let output = client
        .check(
            opened.handle,
            CheckInput::default(),
            Default::default(),
            &mut |_event| {},
        )
        .map_err(destack::Error::new)?;

    assert_eq!(output.diagnostics, []);

    Ok(())
}

/// Create one unique test root.
fn create_root(prefix: &str) -> PathBuf {
    let index = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "destack-bridge-rust-{prefix}-{}-{index}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create rust bridge test root");

    root
}

/// Write one text file.
fn write_text(path: &Path, text: &str) {
    let parent = path.parent().expect("test path should have a parent");
    std::fs::create_dir_all(parent).expect("create rust bridge test directory");
    std::fs::write(path, text).expect("write rust bridge test file");
}
