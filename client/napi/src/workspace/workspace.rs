use destack_source::Edit;
use destack_workspace::{Server, WebSocketServer};
use napi::Result;
use napi_derive::napi;

use crate::core::to_error;

/// In-memory source file exposed to Node.
#[derive(Debug)]
#[napi(object)]
pub struct MemoryFile {
    /// Repository relative file path.
    pub path: String,
    /// UTF-8 text content.
    pub text: Option<String>,
    /// Binary content.
    pub bytes: Option<Vec<u8>>,
}

/// In-process workspace protocol server exposed to Node.
#[derive(Debug)]
#[napi]
pub struct LocalWorkspaceServer {
    /// Local Rust server.
    server: Server,
}

/// Remote workspace protocol server exposed to Node.
#[derive(Debug)]
#[napi]
pub struct RemoteWorkspaceServer {
    /// Workspace WebSocket server.
    server: WebSocketServer,
}

#[napi]
impl LocalWorkspaceServer {
    /// Open an in-process workspace protocol server.
    #[napi(factory)]
    pub fn open(home: String) -> Result<Self> {
        let server = Server::open(home).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Open an in-process workspace protocol server from in-memory files.
    #[napi(factory)]
    pub fn memory(root: String, files: Vec<MemoryFile>) -> Result<Self> {
        let files = files
            .into_iter()
            .map(memory_file)
            .collect::<Result<Vec<_>>>()?;
        let server = Server::memory(root, files).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Dispatch one encoded protocol message payload.
    #[napi]
    pub fn dispatch(&self, payload: Vec<u8>) -> Result<Vec<Vec<u8>>> {
        self.server.dispatch(&payload).map_err(to_error)
    }
}

#[napi]
impl RemoteWorkspaceServer {
    /// Open a remote workspace protocol server over WebSocket.
    #[napi(factory)]
    pub fn open(root: String) -> Result<Self> {
        let server = WebSocketServer::open(root).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Return the WebSocket URL for this server.
    #[napi]
    pub fn url(&self) -> String {
        self.server.url().to_string()
    }

    /// Close this server.
    #[napi]
    pub fn close(&mut self) -> Result<()> {
        self.server
            .close()
            .map_err(|_| to_error("remote workspace server thread panicked"))
    }
}

/// Convert one Node memory file into one Rust memory file.
fn memory_file(file: MemoryFile) -> Result<Edit> {
    match (file.text, file.bytes) {
        (Some(text), None) => Ok(Edit::SetText {
            path: file.path.into(),
            text,
        }),
        (None, Some(bytes)) => Ok(Edit::SetBytes {
            path: file.path.into(),
            bytes,
        }),
        (Some(_), Some(_)) => Err(to_error("memory file cannot contain both text and bytes")),
        (None, None) => Err(to_error("memory file must contain text or bytes")),
    }
}
