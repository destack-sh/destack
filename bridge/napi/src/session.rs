use napi::Result;
use napi_derive::napi;

use crate::{FileUpdate, Revision, SourceSnapshot, SourceUpdate, SourceUpdateResult};

/// Live language session exposed to Node API clients.
#[derive(Debug)]
#[napi]
pub struct Session {
    /// Shared bridge core session.
    inner: destack_bridge_core::Session,
}

#[napi]
impl Session {
    /// Open one session from an explicit source snapshot.
    #[napi(factory)]
    pub fn open_source(root: String, source: SourceSnapshot) -> Result<Self> {
        let source = source.into_core()?;
        let inner = destack_bridge_core::Session::open_source(root, source)
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;

        Ok(Self { inner })
    }

    /// Open one session from a native filesystem path.
    #[napi(factory)]
    pub fn open_path(path: String) -> Result<Self> {
        let inner = destack_bridge_core::Session::open_path(path)
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;

        Ok(Self { inner })
    }

    /// Return the current session revision.
    #[napi]
    pub fn revision(&self) -> Result<Revision> {
        let revision = self
            .inner
            .revision()
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;

        Ok(Revision::from_core(revision))
    }

    /// Return editable repository file paths at the current revision.
    #[napi]
    pub fn files(&self) -> Result<Vec<String>> {
        self.inner
            .files()
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }

    /// Apply one source update through the default session ref.
    #[napi]
    pub fn update(&self, update: SourceUpdate) -> Result<SourceUpdateResult> {
        let update = update.into_core()?;
        let result = self
            .inner
            .update(update)
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;

        Ok(SourceUpdateResult::from_core(&self.inner, result))
    }

    /// Reload tracked files from this session filesystem.
    #[napi]
    pub fn reload(&self) -> Result<Vec<FileUpdate>> {
        let updates = self
            .inner
            .reload()
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        let updates = updates
            .into_iter()
            .map(|update| FileUpdate::from_core(&self.inner, update))
            .collect();

        Ok(updates)
    }

    /// Load one module path into the default session ref.
    #[napi]
    pub fn load_module(&self, path: String) -> Result<String> {
        self.inner
            .load_module(path)
            .map_err(|error| napi::Error::from_reason(error.to_string()))
    }
}
