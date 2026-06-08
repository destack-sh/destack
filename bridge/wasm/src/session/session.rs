use destack as rust;
use js_sys::Array;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::{
    ArtifactKey, ArtifactSidecar, ArtifactVersion, Diagnostic, FileUpdate, Module, Revision,
    SessionFile, SourceSnapshot, SourceUpdate, SourceUpdateResult, js_error,
};

/// Live language session exposed to WebAssembly bindings.
#[derive(Debug)]
#[wasm_bindgen]
pub struct Session {
    session: rust::Session,
}

#[wasm_bindgen]
impl Session {
    /// Open one session from an explicit source snapshot.
    #[wasm_bindgen(js_name = openSource)]
    pub fn open_source(root: String, source: SourceSnapshot) -> Result<Session, JsValue> {
        let session = rust::Session::open_source(root, source.into_bridge()).map_err(js_error)?;

        Ok(Self { session })
    }

    /// Return the current session revision.
    #[wasm_bindgen]
    pub fn revision(&self) -> Result<Revision, JsValue> {
        let revision = self.session.revision().map_err(js_error)?;

        Ok(Revision::from_bridge(revision))
    }

    /// Return editable repository file paths at the current revision.
    #[wasm_bindgen]
    pub fn files(&self) -> Result<Array, JsValue> {
        let files = self
            .session
            .files()
            .map_err(js_error)?
            .into_iter()
            .map(|file| {
                let file = SessionFile::from_bridge(file);

                JsValue::from(file)
            })
            .collect();

        Ok(files)
    }

    /// Apply one source update through the default session ref.
    #[wasm_bindgen]
    pub fn update(&self, update: SourceUpdate) -> Result<SourceUpdateResult, JsValue> {
        let result = self
            .session
            .update(update.into_bridge())
            .map_err(js_error)?;

        Ok(SourceUpdateResult::from_bridge(result))
    }

    /// Reload tracked files from this session filesystem.
    #[wasm_bindgen]
    pub fn reload(&self) -> Result<Array, JsValue> {
        let updates = self
            .session
            .reload()
            .map_err(js_error)?
            .into_iter()
            .map(|update| {
                let update = FileUpdate::from_bridge(update);

                JsValue::from(update)
            })
            .collect();

        Ok(updates)
    }

    /// Load one module path into the default session ref.
    #[wasm_bindgen(js_name = loadModule)]
    pub fn load_module(&self, path: String) -> Result<Module, JsValue> {
        let module = self.session.load_module(path).map_err(js_error)?;

        Ok(Module::from_bridge(module))
    }

    /// Provide root artifacts for one immutable revision.
    #[wasm_bindgen]
    pub fn provide(&self, revision: Revision, keys: Vec<ArtifactKey>) -> Result<(), JsValue> {
        let revision = revision.into_bridge();
        let keys = keys
            .into_iter()
            .map(ArtifactKey::into_bridge)
            .collect::<Vec<_>>();

        self.session.provide(revision, keys).map_err(js_error)
    }

    /// Require one root artifact for one immutable revision.
    #[wasm_bindgen]
    pub fn require(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> Result<ArtifactVersion, JsValue> {
        let revision = revision.into_bridge();
        let key = key.into_bridge();
        let version = self.session.require(revision, key).map_err(js_error)?;

        Ok(ArtifactVersion::from_bridge(version))
    }

    /// Return diagnostics for one immutable revision.
    #[wasm_bindgen]
    pub fn diagnostics(
        &self,
        revision: Revision,
        key: Option<ArtifactKey>,
    ) -> Result<Array, JsValue> {
        let revision = revision.into_bridge();
        let key = key.map(ArtifactKey::into_bridge);
        let diagnostics = self.session.diagnostics(revision, key).map_err(js_error)?;
        let diagnostics = diagnostics
            .into_iter()
            .map(|diagnostic| {
                let diagnostic = Diagnostic::from_bridge(diagnostic);

                JsValue::from(diagnostic)
            })
            .collect();

        Ok(diagnostics)
    }

    /// Return sidecars for one artifact key in one immutable revision.
    #[wasm_bindgen]
    pub fn sidecars(&self, revision: Revision, key: ArtifactKey) -> Result<Array, JsValue> {
        let revision = revision.into_bridge();
        let key = key.into_bridge();
        let sidecars = self.session.sidecars(revision, key).map_err(js_error)?;
        let sidecars = sidecars
            .into_iter()
            .map(|sidecar| {
                let sidecar = ArtifactSidecar::from_bridge(sidecar);

                JsValue::from(sidecar)
            })
            .collect();

        Ok(sidecars)
    }
}
