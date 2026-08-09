use std::sync::Arc;

use destack_rpc::{ConnectionOptions, Registry, Session};
use destack_source::Edit;
use destack_workspace::{LocalWorkspace, SharedWorkspace, Workspace, WorkspaceServer};
use js_sys::{Array, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::core::js_error;
use crate::panic::install_panic_hook;

/// In-process workspace RPC session exposed to WebAssembly.
#[derive(Debug)]
#[wasm_bindgen]
pub struct WorkspaceSession {
    /// Generic RPC session hosting the workspace service.
    session: Session,
}

#[wasm_bindgen]
impl WorkspaceSession {
    /// Open one in-memory workspace RPC session.
    #[wasm_bindgen(js_name = memory)]
    pub fn memory(root: String, files: Array) -> Result<WorkspaceSession, JsValue> {
        install_panic_hook();

        let files = memory_files(files)?;
        let workspace = LocalWorkspace::memory(root, files, 1).map_err(js_error)?;
        let workspace: Arc<dyn Workspace> = Arc::new(workspace);
        let service = WorkspaceServer::new(SharedWorkspace::new(workspace)).map_err(js_error)?;
        let mut services = Registry::new();
        services.insert(service).map_err(js_error)?;
        let options = ConnectionOptions::new("destack-wasm");
        let session = Session::new(services, options).map_err(js_error)?;

        Ok(Self { session })
    }

    /// Dispatch one complete inbound RPC message.
    #[wasm_bindgen]
    pub fn dispatch(&self, bytes: Vec<u8>) -> Result<Array, JsValue> {
        let messages = self.session.dispatch(&bytes).map_err(js_error)?;

        Ok(message_array(messages))
    }

    /// Poll ready RPC calls.
    #[wasm_bindgen]
    pub fn poll(&self) -> Result<Array, JsValue> {
        let messages = self.session.poll().map_err(js_error)?;

        Ok(message_array(messages))
    }

    /// Return whether cooperative workspace calls requested another poll.
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> Result<bool, JsValue> {
        self.session.is_ready().map_err(js_error)
    }

    /// Close this RPC session.
    #[wasm_bindgen]
    pub fn close(&self) -> Result<(), JsValue> {
        self.session.close().map_err(js_error)
    }
}

/// Convert complete RPC messages into JavaScript byte arrays.
fn message_array(messages: Vec<Vec<u8>>) -> Array {
    messages
        .into_iter()
        .map(|message| JsValue::from(Uint8Array::from(message.as_slice())))
        .collect()
}

/// Convert JavaScript memory files into source edits.
fn memory_files(files: Array) -> Result<Vec<Edit>, JsValue> {
    let mut output = Vec::with_capacity(files.length() as usize);

    for file in files {
        output.push(memory_file(file)?);
    }

    Ok(output)
}

/// Convert one JavaScript memory file into one source edit.
fn memory_file(file: JsValue) -> Result<Edit, JsValue> {
    let path = Reflect::get(&file, &JsValue::from_str("path"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("memory file path must be a string"))?;
    let text = Reflect::get(&file, &JsValue::from_str("text"))?;
    let bytes = Reflect::get(&file, &JsValue::from_str("bytes"))?;

    if let Some(text) = text.as_string() {
        if !bytes.is_undefined() {
            return Err(JsValue::from_str(
                "memory file cannot contain both text and bytes",
            ));
        }

        return Ok(Edit::SetText {
            path: path.into(),
            text,
        });
    }
    if !bytes.is_undefined() {
        let bytes = bytes
            .dyn_into::<Uint8Array>()
            .map_err(|_| JsValue::from_str("memory file bytes must be a Uint8Array"))?;

        return Ok(Edit::SetBytes {
            path: path.into(),
            bytes: bytes.to_vec(),
        });
    }

    Err(JsValue::from_str("memory file must contain text or bytes"))
}
