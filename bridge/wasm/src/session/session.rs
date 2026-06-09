use destack as rust;
use js_sys::Array;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::{
    ArtifactKey, ArtifactRecord, ArtifactSidecar, ArtifactVersion, Change, Commit, Diagnostic,
    DirChecked, DirParsed, DirResolved, Edit, Module, ProfileId, Revision, SessionFile, Source,
    js_error,
};

/// Live language session exposed to WebAssembly bindings.
#[derive(Debug)]
#[wasm_bindgen]
pub struct Session {
    session: rust::Session,
}

#[wasm_bindgen]
impl Session {
    /// Open one session from one source input.
    #[wasm_bindgen(js_name = open)]
    pub fn open(source: Source) -> Result<Session, JsValue> {
        let session = rust::Session::open(source.into_bridge()).map_err(js_error)?;

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

    /// Edit files through the default session ref.
    #[wasm_bindgen]
    pub fn edit(&self, edits: Vec<Edit>) -> Result<Commit, JsValue> {
        let edits = edits.into_iter().map(Edit::into_bridge).collect();
        let result = self.session.edit(edits).map_err(js_error)?;

        Ok(Commit::from_bridge(result))
    }

    /// Edit files when the current revision still matches.
    #[wasm_bindgen(js_name = editAt)]
    pub fn edit_at(&self, revision: Revision, edits: Vec<Edit>) -> Result<Commit, JsValue> {
        let edits = edits.into_iter().map(Edit::into_bridge).collect();
        let result = self
            .session
            .edit_at(revision.into_bridge(), edits)
            .map_err(js_error)?;

        Ok(Commit::from_bridge(result))
    }

    /// Reload tracked files from this session backing source.
    #[wasm_bindgen]
    pub fn reload(&self) -> Result<Array, JsValue> {
        let changes = self
            .session
            .reload()
            .map_err(js_error)?
            .into_iter()
            .map(|change| {
                let change = Change::from_bridge(change);

                JsValue::from(change)
            })
            .collect();

        Ok(changes)
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

    /// Return one raw artifact record for one immutable revision.
    #[wasm_bindgen(js_name = artifactRecord)]
    pub fn artifact_record(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> Result<ArtifactRecord, JsValue> {
        let revision = revision.into_bridge();
        let key = key.into_bridge();
        let record = self
            .session
            .artifact_record(revision, key)
            .map_err(js_error)?;

        Ok(ArtifactRecord::from_bridge(record))
    }

    /// Return the parsed DIR artifact for one loaded module.
    #[wasm_bindgen]
    pub fn parse(&self, revision: Revision, module: Module) -> Result<DirParsed, JsValue> {
        let revision = revision.into_bridge();
        let module = module.into_bridge();
        let parsed = self.session.parse(revision, module).map_err(js_error)?;

        Ok(DirParsed::from_bridge(parsed))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    #[wasm_bindgen]
    pub fn resolve(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<DirResolved, JsValue> {
        let revision = revision.into_bridge();
        let module = module.into_bridge();
        let profile = profile.into_bridge();
        let resolved = self
            .session
            .resolve(revision, module, profile)
            .map_err(js_error)?;

        Ok(DirResolved::from_bridge(resolved))
    }

    /// Return the checked DIR facade artifact for one loaded module profile.
    #[wasm_bindgen]
    pub fn check(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<DirChecked, JsValue> {
        let revision = revision.into_bridge();
        let module = module.into_bridge();
        let profile = profile.into_bridge();
        let checked = self
            .session
            .check(revision, module, profile)
            .map_err(js_error)?;

        Ok(DirChecked::from_bridge(checked))
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
