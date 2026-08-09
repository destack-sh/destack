use std::sync::Arc;

use destack_rpc::{ConnectionOptions, Registry, Session};
use destack_source::Edit;
use destack_workspace::{LocalWorkspace, SharedWorkspace, Workspace, WorkspaceServer};
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::{Result, Status};
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

/// In-process workspace RPC session exposed to Node.
#[derive(Debug)]
#[napi]
pub struct WorkspaceSession {
    /// Generic RPC session hosting the workspace service.
    session: Session,
}

#[napi]
impl WorkspaceSession {
    /// Open one physical workspace RPC session.
    #[napi(factory)]
    pub fn open(path: String) -> Result<Self> {
        let workers = LocalWorkspace::default_worker_count();
        let workspace = LocalWorkspace::open(path, workers).map_err(to_error)?;

        Self::from_workspace(Arc::new(workspace))
    }

    /// Open one in-memory workspace RPC session.
    #[napi(factory)]
    pub fn memory(root: String, files: Vec<MemoryFile>) -> Result<Self> {
        let files = files
            .into_iter()
            .map(memory_file)
            .collect::<Result<Vec<_>>>()?;
        let workers = LocalWorkspace::default_worker_count();
        let workspace = LocalWorkspace::memory(root, files, workers).map_err(to_error)?;

        Self::from_workspace(Arc::new(workspace))
    }

    /// Dispatch one complete inbound RPC message.
    #[napi]
    pub fn dispatch(&self, bytes: Vec<u8>) -> Result<Vec<Vec<u8>>> {
        self.session.dispatch(&bytes).map_err(to_error)
    }

    /// Poll ready RPC calls.
    #[napi]
    pub fn poll(&self) -> Result<Vec<Vec<u8>>> {
        self.session.poll().map_err(to_error)
    }

    /// Return whether cooperative workspace calls requested another poll.
    #[napi]
    pub fn is_ready(&self) -> Result<bool> {
        self.session.is_ready().map_err(to_error)
    }

    /// Install the JavaScript callback invoked when a cooperative call becomes ready.
    #[napi]
    pub fn on_ready(&self, callback: ThreadsafeFunction<(), (), (), Status, false>) {
        self.session.set_wake_handler(Arc::new(move || {
            let status = callback.call((), ThreadsafeFunctionCallMode::NonBlocking);
            if !matches!(status, Status::Ok | Status::Closing) {
                eprintln!("failed to wake the JavaScript RPC host: {status:?}");
            }
        }));
    }

    /// Close this RPC session.
    #[napi]
    pub fn close(&self) -> Result<()> {
        self.session.close().map_err(to_error)
    }

    /// Host one workspace implementation in a generic RPC session.
    fn from_workspace(workspace: Arc<dyn Workspace>) -> Result<Self> {
        let service = WorkspaceServer::new(SharedWorkspace::new(workspace)).map_err(to_error)?;
        let mut services = Registry::new();
        services.insert(service).map_err(to_error)?;
        let options = ConnectionOptions::new("destack-napi");
        let session = Session::new(services, options).map_err(to_error)?;

        Ok(Self { session })
    }
}

/// Convert one Node memory file into one source edit.
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
