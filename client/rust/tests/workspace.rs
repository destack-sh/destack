use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use destack_workspace::{CheckInput, Client, ClientOptions, Transport, TransportError};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

/// Temporary repository root for one client test.
#[derive(Debug)]
struct TestRoot {
    /// Root directory path.
    path: PathBuf,
}

impl TestRoot {
    /// Create one unique test root.
    fn create(prefix: &str) -> Self {
        let index = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "destack-client-rust-{prefix}-{}-{index}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create rust client test root");

        Self { path }
    }

    /// Return this root path.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Transport that dispatches frames through a local workspace server.
#[derive(Debug)]
struct EmbeddedTransport {
    /// Local server under test.
    server: destack::workspace::Server,
    /// Frames ready for the client to receive.
    queue: Mutex<VecDeque<Vec<u8>>>,
}

impl EmbeddedTransport {
    /// Create a transport for one embedded server.
    fn new(server: destack::workspace::Server) -> Self {
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
            .map_err(|error| TransportError::Io(std::io::Error::other(error.to_string())))?;
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

/// Check one local workspace through an embedded server.
#[test]
fn test_check_local_workspace_through_embedded_server() -> Result<(), Box<dyn std::error::Error>> {
    let root = TestRoot::create("check_local_workspace");
    write_text(&root.path().join("destack.json"), r#"{"name":"@test/app"}"#);
    write_text(&root.path().join("src/index.ds"), "export const value = 1;");

    // open an embedded server and client
    let server = destack::workspace::Server::open(root.path())?;
    let transport = Arc::new(EmbeddedTransport::new(server));
    let client = Client::new(transport);
    client.handshake(ClientOptions::default())?;

    // open the project root through the client
    let opened = client.open_root(root.path().to_path_buf())?;

    // check the project through the embedded server
    let output = client.check(
        opened.handle,
        CheckInput::default(),
        Default::default(),
        &mut |_event| {},
    )?;

    assert_eq!(output.diagnostics, []);

    Ok(())
}
/// Write one text file.
fn write_text(path: &Path, text: &str) {
    let parent = path.parent().expect("test path should have a parent");
    std::fs::create_dir_all(parent).expect("create rust client test directory");
    std::fs::write(path, text).expect("write rust client test file");
}
