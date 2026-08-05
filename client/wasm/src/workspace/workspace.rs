use destack_source::Edit;
use destack_workspace::Server;
use js_sys::{Array, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::core::js_error;
use crate::panic::install_panic_hook;

/// In-process workspace protocol server exposed to WebAssembly.
#[derive(Debug)]
#[wasm_bindgen]
pub struct LocalWorkspaceServer {
    /// Local Rust server.
    server: Server,
}

#[wasm_bindgen]
impl LocalWorkspaceServer {
    /// Open an in-process workspace protocol server.
    #[wasm_bindgen(js_name = open)]
    pub fn open(home: String) -> Result<LocalWorkspaceServer, JsValue> {
        install_panic_hook();

        let server = Server::open(home).map_err(js_error)?;

        Ok(Self { server })
    }

    /// Open an in-process workspace protocol server from in-memory files.
    #[wasm_bindgen(js_name = memory)]
    pub fn memory(root: String, files: Array) -> Result<LocalWorkspaceServer, JsValue> {
        install_panic_hook();

        let files = memory_files(files)?;
        let server = Server::memory(root, files).map_err(js_error)?;

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

/// Convert JavaScript memory files into Rust memory files.
fn memory_files(files: Array) -> Result<Vec<Edit>, JsValue> {
    let mut output = Vec::with_capacity(files.length() as usize);

    for file in files {
        output.push(memory_file(file)?);
    }

    Ok(output)
}

/// Convert one JavaScript memory file into one Rust memory file.
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
