use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PathExt};

use crate::TsConfigDeclaration;
use crate::repository::{Repository, RepositoryError, Revision};

impl Repository {
    /// Return one parsed tsconfig declaration by file id.
    pub fn tsconfig_declaration_for_file(
        &self,
        revision: Revision,
        tsconfig_file_id: FileId,
    ) -> Result<Option<Arc<TsConfigDeclaration>>, RepositoryError> {
        let Some(content_id) = self.file_content_id(revision, tsconfig_file_id)? else {
            return Ok(None);
        };

        if let Some(tsconfig) = self.file_cache.tsconfig_declarations.get(&content_id) {
            return Ok(tsconfig.value().as_ref().ok().cloned());
        }

        // parse from source
        let Some(file) = self.file(revision, tsconfig_file_id)? else {
            return Ok(None);
        };
        let tsconfig = TsConfigDeclaration::parse(true, &file)
            .map(Arc::new)
            .map_err(|error| error.to_string());
        let declaration = tsconfig.as_ref().ok().cloned();

        self.file_cache
            .tsconfig_declarations
            .insert(content_id, tsconfig);

        Ok(declaration)
    }

    /// Return the effective tsconfig file id for one path.
    pub fn tsconfig_file_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, RepositoryError> {
        let Some(nearest_tsconfig_file_id) =
            self.nearest_tsconfig_file_id_for_path(revision, path)?
        else {
            return Ok(None);
        };

        Ok(Some(self.select_effective_tsconfig_file_id(
            revision,
            nearest_tsconfig_file_id,
            path,
        )?))
    }

    /// Return the nearest tsconfig file id for one path.
    fn nearest_tsconfig_file_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, RepositoryError> {
        let mut current = path.parent();
        while let Some(directory) = current {
            let tsconfig_path = directory.join("tsconfig.json");
            let tsconfig_file_id = self.file_id(&tsconfig_path);
            if self
                .tsconfig_declaration_for_file(revision, tsconfig_file_id)?
                .is_some()
            {
                return Ok(Some(tsconfig_file_id));
            }

            current = directory.parent();
        }

        Ok(None)
    }

    /// Return the most specific effective tsconfig file id.
    fn select_effective_tsconfig_file_id(
        &self,
        revision: Revision,
        tsconfig_file_id: FileId,
        path: &Path,
    ) -> Result<FileId, RepositoryError> {
        let mut visited = HashSet::new();

        Ok(self
            .select_effective_tsconfig_file_id_recursive(
                revision,
                tsconfig_file_id,
                path,
                &mut visited,
            )?
            .unwrap_or(tsconfig_file_id))
    }

    /// Walk tsconfig references to find the most specific match.
    fn select_effective_tsconfig_file_id_recursive(
        &self,
        revision: Revision,
        tsconfig_file_id: FileId,
        path: &Path,
        visited: &mut HashSet<FileId>,
    ) -> Result<Option<FileId>, RepositoryError> {
        if !visited.insert(tsconfig_file_id) {
            return Ok(None);
        }

        let Some(tsconfig) = self.tsconfig_declaration_for_file(revision, tsconfig_file_id)? else {
            return Ok(None);
        };
        if tsconfig.applies_to_path(path) {
            return Ok(Some(tsconfig_file_id));
        }

        for reference in &tsconfig.json.references {
            let reference_path = tsconfig.directory.normalize_with(&reference.path);
            let Some(reference_tsconfig_file_id) =
                self.tsconfig_file_id_from_reference_path(revision, &reference_path)?
            else {
                continue;
            };

            if let Some(reference_tsconfig_file_id) = self
                .select_effective_tsconfig_file_id_recursive(
                    revision,
                    reference_tsconfig_file_id,
                    path,
                    visited,
                )?
            {
                return Ok(Some(reference_tsconfig_file_id));
            }
        }

        Ok(None)
    }

    /// Resolve one tsconfig reference path to one file id.
    fn tsconfig_file_id_from_reference_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, RepositoryError> {
        let Some(path) = self.materialize_tsconfig_path(revision, path)? else {
            return Ok(None);
        };

        Ok(Some(self.file_id(&path)))
    }

    /// Materialize one tsconfig reference path.
    fn materialize_tsconfig_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<PathBuf>, RepositoryError> {
        let direct_file_id = self.file_id(path);
        if self.file(revision, direct_file_id)?.is_some() {
            return Ok(Some(path.to_path_buf()));
        }

        let nested_path = path.join("tsconfig.json");
        let nested_file_id = self.file_id(&nested_path);
        if self.file(revision, nested_file_id)?.is_some() {
            return Ok(Some(nested_path));
        }

        let json_path = PathBuf::from(format!("{}.json", path.display()));
        let json_file_id = self.file_id(&json_path);
        if self.file(revision, json_file_id)?.is_some() {
            return Ok(Some(json_path));
        }

        Ok(None)
    }
}
