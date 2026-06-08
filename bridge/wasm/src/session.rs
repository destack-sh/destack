use js_sys::Array;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::{FileUpdate, Revision, SourceSnapshot, SourceUpdate, SourceUpdateResult, js_error};

/// Live language session exposed to WebAssembly clients.
#[derive(Debug)]
#[wasm_bindgen]
pub struct Session {
    inner: destack_bridge_core::Session,
}

#[wasm_bindgen]
impl Session {
    /// Open one session from an explicit source snapshot.
    #[wasm_bindgen(js_name = openSource)]
    pub fn open_source(root: String, source: SourceSnapshot) -> Result<Session, JsValue> {
        let source = source.into_core();
        let inner = destack_bridge_core::Session::open_source(root, source).map_err(js_error)?;

        Ok(Self { inner })
    }

    /// Return the current session revision.
    #[wasm_bindgen]
    pub fn revision(&self) -> Result<Revision, JsValue> {
        let revision = self.inner.revision().map_err(js_error)?;

        Ok(Revision::from_core(revision))
    }

    /// Return editable repository file paths at the current revision.
    #[wasm_bindgen]
    pub fn files(&self) -> Result<Array, JsValue> {
        let files = self.inner.files().map_err(js_error)?;
        let files = files.into_iter().map(JsValue::from).collect();

        Ok(files)
    }

    /// Apply one source update through the default session ref.
    #[wasm_bindgen]
    pub fn update(&self, update: SourceUpdate) -> Result<SourceUpdateResult, JsValue> {
        let update = update.into_core()?;
        let result = self.inner.update(update).map_err(js_error)?;

        Ok(SourceUpdateResult::from_core(&self.inner, result))
    }

    /// Reload tracked files from this session filesystem.
    #[wasm_bindgen]
    pub fn reload(&self) -> Result<Array, JsValue> {
        let updates = self.inner.reload().map_err(js_error)?;
        let updates = updates
            .into_iter()
            .map(|update| JsValue::from(FileUpdate::from_core(&self.inner, update)))
            .collect();

        Ok(updates)
    }

    /// Load one module path into the default session ref.
    #[wasm_bindgen(js_name = loadModule)]
    pub fn load_module(&self, path: String) -> Result<String, JsValue> {
        self.inner.load_module(path).map_err(js_error)
    }
}
