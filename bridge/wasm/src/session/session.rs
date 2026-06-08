use std::path::Path;
use std::sync::Arc;

use destack_bridge_language as bridge;
use destack_compiler as compiler;
use destack_linter as linter;
use destack_query as query;
use destack_repository as repository;
use destack_session as session;
use js_sys::Array;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::{
    FileUpdate, Module, Revision, SessionFile, SourceSnapshot, SourceUpdate, SourceUpdateResult,
    js_error,
};

/// Live language session exposed to WebAssembly clients.
#[derive(Debug)]
#[wasm_bindgen]
pub struct Session {
    session: session::Session,
}

#[wasm_bindgen]
impl Session {
    /// Open one session from an explicit source snapshot.
    #[wasm_bindgen(js_name = openSource)]
    pub fn open_source(root: String, source: SourceSnapshot) -> Result<Session, JsValue> {
        let source = session::SourceSnapshot::try_from(source.into_bridge()).map_err(js_error)?;
        let repository = session::open_repository_from_source(
            root.into(),
            source,
            repository::Environment::default(),
            repository::Settings::default(),
            repository::DestackLayoutOverride::default(),
        )
        .map_err(js_error)?;

        Self::open(repository)
    }

    /// Return the current session revision.
    #[wasm_bindgen]
    pub fn revision(&self) -> Result<Revision, JsValue> {
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(js_error)?;
        let revision = bridge::Revision::from_repository(revision);

        Ok(Revision::from_bridge(revision))
    }

    /// Return editable repository file paths at the current revision.
    #[wasm_bindgen]
    pub fn files(&self) -> Result<Array, JsValue> {
        let repository = self.session.repository();
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(js_error)?;
        let mut paths = repository
            .editable_file_logical_paths(revision)
            .map_err(js_error)?
            .into_iter()
            .map(|(_, path)| repository.string_pool().get(path).to_string())
            .collect::<Vec<_>>();
        paths.sort();

        let files = paths
            .into_iter()
            .map(|path| {
                let file = bridge::SessionFile::new(path);
                let file = SessionFile::from_bridge(file);

                JsValue::from(file)
            })
            .collect();

        Ok(files)
    }

    /// Apply one source update through the default session ref.
    #[wasm_bindgen]
    pub fn update(&self, update: SourceUpdate) -> Result<SourceUpdateResult, JsValue> {
        let update = session::SourceUpdate::try_from(update.into_bridge()).map_err(js_error)?;
        let result = self
            .session
            .update(self.session.head(), update)
            .map_err(js_error)?;
        let result = bridge::SourceUpdateResult::from_session_update(&self.session, result);

        Ok(SourceUpdateResult::from_bridge(result))
    }

    /// Reload tracked files from this session filesystem.
    #[wasm_bindgen]
    pub fn reload(&self) -> Result<Array, JsValue> {
        let updates = self
            .session
            .reload_from_fs(self.session.head())
            .map_err(js_error)?;
        let updates = updates
            .into_iter()
            .map(|update| {
                let update = bridge::FileUpdate::from_session_update(&self.session, update);
                let update = FileUpdate::from_bridge(update);

                JsValue::from(update)
            })
            .collect();

        Ok(updates)
    }

    /// Load one module path into the default session ref.
    #[wasm_bindgen(js_name = loadModule)]
    pub fn load_module(&self, path: String) -> Result<Module, JsValue> {
        let module = self
            .session
            .load_module_from_fs(self.session.head(), Path::new(&path))
            .map_err(js_error)?;
        let module = bridge::Module::new(format!("{module:?}"));

        Ok(Module::from_bridge(module))
    }
}

impl Session {
    /// Open one WASM session from one prepared repository.
    fn open(repository: repository::Repository) -> Result<Self, JsValue> {
        let repository = Arc::new(repository);
        let root = repository.path().to_path_buf();
        let head = repository::Ref::for_root(&root);
        let compiler = Arc::new(compiler::Compiler::new(Arc::clone(&repository)));
        let linter = Arc::new(linter::Linter::new(Arc::clone(&repository)));
        let query = Arc::new(query::Query::new(Arc::clone(&repository)));
        let worker_count = session::Session::default_worker_count();
        let session = session::Session::new(
            root.clone(),
            root,
            repository,
            head,
            compiler,
            linter,
            query,
            worker_count,
            None,
        )
        .map_err(js_error)?;

        Ok(Self { session })
    }
}
