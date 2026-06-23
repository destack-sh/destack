use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::core::js_error;

/// In-process workspace protocol server exposed to WebAssembly.
#[derive(Debug)]
#[wasm_bindgen]
pub struct LocalWorkspaceServer {
    /// Local Rust server.
    server: destack::LocalWorkspaceServer,
}

#[wasm_bindgen]
impl LocalWorkspaceServer {
    /// Open an in-process workspace protocol server.
    #[wasm_bindgen(js_name = open)]
    pub fn open(home: String) -> Result<LocalWorkspaceServer, JsValue> {
        crate::panic::install_panic_hook();

        let server = destack::LocalWorkspaceServer::open(home).map_err(js_error)?;

        Ok(Self { server })
    }

    /// Dispatch one encoded protocol message payload.
    #[wasm_bindgen]
    pub fn dispatch(&self, payload: Vec<u8>) -> Result<Array, JsValue> {
        let frames = self.server.dispatch(&payload).map_err(js_error)?;
        let frames = frames
            .into_iter()
            .map(|frame| JsValue::from(Uint8Array::from(frame.as_slice())))
            .collect();

        Ok(frames)
    }
}
