use destack_artifact::{ArtifactKey, ArtifactVersion};
use destack_source::{FileType, Loader, ModuleId};
use destack_workspace::{Ref, Revision};

use crate::{Session, SessionError};

impl Session {
    /// Import one filesystem-backed ref and return its immutable revision.
    pub fn import_from_fs(&self, reference: &Ref) -> Result<Revision, SessionError> {
        // materialize the latest source state for this ref
        let _ = self.load_from_fs(reference)?;
        let revision = self.revision(reference)?;

        // make source artifacts available for this immutable revision
        self.import_revision(revision)?;

        Ok(revision)
    }

    /// Import one immutable revision.
    pub fn import_revision(&self, revision: Revision) -> Result<(), SessionError> {
        // import every visible module in this immutable revision
        let module_ids = self.repository().module_ids(revision)?;
        for module_id in module_ids {
            self.import_module(revision, module_id)?;
        }

        Ok(())
    }

    /// Import one module and its source artifacts.
    pub fn import_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<(), SessionError> {
        // every module has an AST, code gets syntax and other files get anchors
        self.require(revision, ArtifactKey::ast(module_id))?;

        // data artifacts exist only for parsed data formats
        self.import_module_data(revision, module_id)?;

        Ok(())
    }

    /// Import one module data artifact when the file type has parsed data.
    fn import_module_data(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Option<ArtifactVersion>, SessionError> {
        // read the module and file from the immutable revision
        let module = self
            .repository()
            .module(revision, module_id)?
            .ok_or(SessionError::ModuleIdNotTracked { module_id })?;
        let file = self.file_for_id(revision, module.file_id)?;

        // skip files that do not produce parsed data
        if !module_has_data(module.loader, file.ty) {
            return Ok(None);
        }

        // materialize the data artifact through the normal source provider
        let version = self.require(revision, ArtifactKey::data(module_id))?;

        Ok(Some(version))
    }
}

/// Return whether one module should have a data artifact.
fn module_has_data(loader: Loader, file_type: FileType) -> bool {
    // data loaders always produce data
    if loader.is_data() {
        return true;
    }

    // markup and style formats are parsed as data
    matches!(file_type, FileType::Html | FileType::Css)
}
