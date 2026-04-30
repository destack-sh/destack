use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{File, FileContent, FileId, FileType, IgnoreSet, ModuleId, PackageId};
use destack_workspace::{Edit, Ref, Repository, Revision};

use crate::session::FileChange;
use crate::{FileChangeKind, FileMutation, Session, SessionChange, SessionError};

const IMPORT_IGNORED_DIRECTORY_NAMES: &[&str] = &[".git", "node_modules", "target"];

/// One filesystem path role during revision import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportPathKind {
    /// Directory that can be walked.
    Directory,
    /// Configuration file that can affect workspace shape.
    Config,
    /// Source module file that can enter the revision.
    Module,
    /// Path that does not contribute to revision import.
    Other,
}

impl Session {
    /// Import source files from disk into one ref.
    pub fn load_from_fs(&self, reference: &Ref) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let before = self.revision(reference)?;
        let (after, file_changes) = self.import_files(before)?;

        self.finish_change(reference, before, after, file_changes, None)
    }

    /// Refresh tracked source state from disk into one ref.
    pub fn refresh_from_fs(&self, reference: &Ref) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let before = self.revision(reference)?;
        let (after, file_changes) = self.refresh_files(before)?;

        self.finish_change(reference, before, after, file_changes, None)
    }

    /// Import untracked session files into a revision derived from one base revision.
    fn import_files(
        &self,
        base_revision: Revision,
    ) -> Result<(Revision, Vec<FileChange>), SessionError> {
        let mut pending_directories = vec![self.root().to_path_buf()];
        let mut visited_directories = HashSet::new();
        let mut ignore_set = IgnoreSet::new();
        let repository = self.repository();
        let mut revision = base_revision;
        let mut file_changes = Vec::new();

        while let Some(directory) = pending_directories.pop() {
            // skip directories already reached through another path
            if !visited_directories.insert(directory.clone()) {
                continue;
            }

            // load ignore rules before classifying child paths
            ignore_set.load_dir(&directory);
            let entries = self.import_directory_entries(repository.as_ref(), &directory)?;

            // fold imported files into the staged revision
            for entry in entries {
                let entry_kind =
                    self.import_path_kind(repository.as_ref(), &ignore_set, entry.as_path())?;
                let entry_change = self.import_path(
                    repository.as_ref(),
                    revision,
                    entry.as_path(),
                    entry_kind,
                    &mut pending_directories,
                )?;

                if let Some((next_revision, file_change)) = entry_change {
                    revision = next_revision;
                    file_changes.push(file_change);
                }
            }
        }

        Ok((revision, file_changes))
    }

    /// Read the child paths for one import directory.
    fn import_directory_entries(
        &self,
        repository: &Repository,
        directory: &Path,
    ) -> Result<Vec<PathBuf>, SessionError> {
        repository
            .file_system()
            .read_dir(directory)
            .map_err(|error| SessionError::ReadPathFailed {
                detail: format!(
                    "failed to read session directory {}: {error}",
                    directory.display(),
                ),
                path: directory.to_path_buf(),
            })
    }

    /// Classify one filesystem path for revision import.
    fn import_path_kind(
        &self,
        repository: &Repository,
        ignore_set: &IgnoreSet,
        path: &Path,
    ) -> Result<ImportPathKind, SessionError> {
        let metadata = repository.file_system().metadata(path).map_err(|error| {
            SessionError::ReadPathFailed {
                detail: format!(
                    "failed to read session metadata {}: {error}",
                    path.display(),
                ),
                path: path.to_path_buf(),
            }
        })?;

        // ignore matched paths
        if ignore_set.is_ignored(self.root(), path, metadata.is_directory) {
            Ok(ImportPathKind::Other)
        }
        // importable directory
        else if metadata.is_directory && self.is_import_directory(path) {
            Ok(ImportPathKind::Directory)
        }
        // importable config file
        else if metadata.is_file && self.is_import_config_path(path) {
            Ok(ImportPathKind::Config)
        }
        // importable module file
        else if metadata.is_file && self.is_import_module_path(path) {
            Ok(ImportPathKind::Module)
        }
        // ignored file
        else {
            Ok(ImportPathKind::Other)
        }
    }

    /// Import one classified path into a staged revision.
    fn import_path(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
        kind: ImportPathKind,
        pending_directories: &mut Vec<PathBuf>,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        match kind {
            ImportPathKind::Directory => {
                pending_directories.push(path.to_path_buf());

                Ok(None)
            }
            ImportPathKind::Config => self.import_config_file(repository, revision, path),
            ImportPathKind::Module => self.import_module_path(repository, revision, path),
            ImportPathKind::Other => Ok(None),
        }
    }

    /// Import one config file when it is not tracked yet.
    fn import_config_file(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        let file_id = repository.file_id_for_workspace_path(path);
        let file = repository
            .file(revision, file_id)
            .map_err(SessionError::from)?;

        // already tracked config
        if file.is_some() {
            return Ok(None);
        }

        let mutation =
            self.read_file_from_fs(path)
                .map_err(|error| SessionError::ReadPathFailed {
                    detail: format!("failed to read session config {}: {error}", path.display()),
                    path: path.to_path_buf(),
                })?;
        let revision = self.apply_file_update_to_revision(repository, revision, path, mutation)?;
        let file_change = self.file_change(repository, revision, path, file_id)?;

        Ok(Some((revision, file_change)))
    }

    /// Import one module path when it is not tracked yet.
    fn import_module_path(
        &self,
        repository: &Repository,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        let module_id = repository
            .module_id_for_path(revision, path)
            .map_err(SessionError::from)?;

        // already tracked module
        if module_id.is_some() {
            return Ok(None);
        }

        let (revision, _) = self.import_module_file(revision, path).map_err(|error| {
            SessionError::ResolvePathFailed {
                path: path.to_path_buf(),
                detail: format!("failed to import session module: {error}"),
            }
        })?;
        let file_id = repository.file_id_for_workspace_path(path);
        let file_change = self.file_change(repository, revision, path, file_id)?;

        Ok(Some((revision, file_change)))
    }

    /// Import one module file into a revision when it is not tracked yet.
    pub(crate) fn import_module_file(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<(Revision, ModuleId), SessionError> {
        let repository = self.repository();
        let path = path.to_path_buf();
        let module_id = repository
            .module_id_for_path(revision, &path)
            .map_err(SessionError::from)?;

        // reuse the existing module when it is already tracked
        if let Some(module_id) = module_id {
            return Ok((revision, module_id));
        }

        let logical_path = repository.normalize_workspace_path(&path);
        let content = repository
            .load_workspace_file_content(&path)
            .map_err(SessionError::from)?;
        let edit = Edit::SetFile {
            logical_path,
            content,
        };
        let revision = self.apply_edits(repository.as_ref(), revision, [edit])?;
        let module_id = repository
            .module_id_for_path(revision, &path)
            .map_err(SessionError::from)?;

        let Some(module_id) = module_id else {
            return Err(SessionError::ResolvePathFailed {
                path,
                detail: "imported source file did not produce a module".to_string(),
            });
        };

        Ok((revision, module_id))
    }

    /// Refresh tracked files into a revision derived from one base revision.
    fn refresh_files(
        &self,
        base_revision: Revision,
    ) -> Result<(Revision, Vec<FileChange>), SessionError> {
        let repository = self.repository();
        let repository = repository.as_ref();
        let files = self.collect_refresh_files(repository, base_revision)?;
        let mut revision = base_revision;
        let mut file_changes = Vec::new();

        for file in files {
            // fold changed tracked files into the staged revision
            if let Some((next_revision, file_change)) =
                self.refresh_file(repository, revision, file.as_ref())?
            {
                revision = next_revision;
                file_changes.push(file_change);
            }
        }

        Ok((revision, file_changes))
    }

    /// Return whether import should descend into one directory.
    fn is_import_directory(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };

        !IMPORT_IGNORED_DIRECTORY_NAMES.contains(&name)
    }

    /// Return whether import should include one config file.
    fn is_import_config_path(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        matches!(name, "package.json" | "destack.json")
            || (name.starts_with("tsconfig") && name.ends_with(".json"))
    }

    /// Return whether import should include one module file.
    pub(crate) fn is_import_module_path(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        // config files are imported as config
        if matches!(name, "package.json" | "destack.json")
            || (name.starts_with("tsconfig") && name.ends_with(".json"))
        {
            false
        }
        // source files are watched and imported as modules
        else {
            self.should_watch_path(path)
        }
    }

    /// Read one filesystem path as a file update.
    pub fn read_file_from_fs(&self, path: &Path) -> io::Result<FileMutation> {
        let repository = self.repository();

        // preserve bytes for binary formats
        if FileType::from_path(path).is_some_and(|file_type| file_type.is_binary()) {
            let content = repository.file_system().read(path)?;

            return Ok(FileMutation::Bytes { content });
        }

        // otherwise prefer text so downstream diagnostics keep source spans
        let content = repository.file_system().read_to_string(path)?;

        Ok(FileMutation::Text { content })
    }

    /// Return true when watch mode should track one path.
    pub fn should_watch_path(&self, path: &Path) -> bool {
        let Some(file_type) = FileType::from_path(path) else {
            return false;
        };

        file_type.is_code()
            || file_type.is_data()
            || file_type.is_text()
            || file_type.is_binary()
            || FileChangeKind::for_path(path).is_config_change()
    }

    /// Collect tracked files that refresh should read from disk.
    fn collect_refresh_files(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<Vec<Arc<File>>, SessionError> {
        let mut seen_file_ids = HashSet::new();
        let mut files = Vec::new();
        let mut package_ids = HashSet::new();

        // workspace config owned by this source root
        if let Some(workspace_config) = repository.destack_declaration_for_workspace(revision)? {
            let path = workspace_config.path.clone();
            if path.starts_with(self.root()) {
                self.push_refresh_file(
                    repository,
                    revision,
                    workspace_config.file_id,
                    &mut seen_file_ids,
                    &mut files,
                )?;
            }
        }

        // modules and their package config files
        for module_id in repository
            .module_ids(revision)
            .map_err(SessionError::from)?
        {
            let Some(module) = repository
                .module(revision, module_id)
                .map_err(SessionError::from)?
            else {
                return Err(SessionError::ModuleIdNotTracked { module_id });
            };
            let is_root_module = module
                .path
                .as_ref()
                .is_some_and(|path| module.is_user() && path.starts_with(self.root()));

            // module source file
            if is_root_module {
                self.push_refresh_file(
                    repository,
                    revision,
                    module.file_id,
                    &mut seen_file_ids,
                    &mut files,
                )?;

                // package config files
                if package_ids.insert(module.package_id) {
                    self.collect_package_refresh_files(
                        repository,
                        revision,
                        module.package_id,
                        &mut seen_file_ids,
                        &mut files,
                    )?;
                }

                // tsconfig file
                if let Some(tsconfig_file_id) = module.tsconfig_file_id
                    && let Some(tsconfig) = repository
                        .tsconfig_declaration_for_file(revision, tsconfig_file_id)
                        .map_err(SessionError::from)?
                {
                    self.push_refresh_file(
                        repository,
                        revision,
                        tsconfig.file_id,
                        &mut seen_file_ids,
                        &mut files,
                    )?;
                }
            }
        }

        Ok(files)
    }

    /// Collect tracked package files that refresh should read from disk.
    fn collect_package_refresh_files(
        &self,
        repository: &Repository,
        revision: Revision,
        package_id: PackageId,
        seen_file_ids: &mut HashSet<FileId>,
        files: &mut Vec<Arc<File>>,
    ) -> Result<(), SessionError> {
        // load the package that owns the source module
        let Some(package) = repository
            .package(revision, package_id)
            .map_err(SessionError::from)?
        else {
            return Err(SessionError::Internal {
                detail: format!("missing package for {package_id:?}"),
            });
        };

        // package manifest
        if let Some(package_file_id) = package.package_file_id {
            self.push_refresh_file(repository, revision, package_file_id, seen_file_ids, files)?;
        }

        // package destack config
        if let Some(config) = repository.destack_declaration_for_package(revision, &package)? {
            self.push_refresh_file(repository, revision, config.file_id, seen_file_ids, files)?;
        }

        Ok(())
    }

    /// Push one tracked refresh file when it has not been seen yet.
    fn push_refresh_file(
        &self,
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
        seen_file_ids: &mut HashSet<FileId>,
        files: &mut Vec<Arc<File>>,
    ) -> Result<(), SessionError> {
        // avoid duplicate config files reached through multiple modules
        if !seen_file_ids.insert(file_id) {
            return Ok(());
        }

        let Some(file) = repository
            .file(revision, file_id)
            .map_err(SessionError::from)?
        else {
            return Ok(());
        };

        files.push(file);

        Ok(())
    }

    /// Refresh one tracked file when its filesystem content changed.
    fn refresh_file(
        &self,
        repository: &Repository,
        revision: Revision,
        file: &File,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
            return Ok(None);
        };

        // read changed filesystem content
        let Some(mutation) = self.read_changed_file_from_fs(repository, file, &path)? else {
            return Ok(None);
        };

        let revision =
            self.apply_file_update_to_revision(repository, revision, path.as_path(), mutation)?;
        let file_change = self.file_change(repository, revision, path.as_path(), file.id)?;

        Ok(Some((revision, file_change)))
    }

    /// Read one changed filesystem file.
    fn read_changed_file_from_fs(
        &self,
        repository: &Repository,
        file: &File,
        path: &Path,
    ) -> Result<Option<FileMutation>, SessionError> {
        // make binary mutation
        if file.ty.is_binary() {
            let bytes = match repository.file_system().read(path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return self.removed_file_or_read_error(path, error);
                }
            };

            if self.bytes_changed(file, &bytes) {
                return Ok(Some(FileMutation::Bytes { content: bytes }));
            }

            Ok(None)
        }
        // make text mutation
        else {
            let content = match repository.file_system().read_to_string(path) {
                Ok(content) => content,
                Err(error) => {
                    return self.removed_file_or_read_error(path, error);
                }
            };

            if self.text_changed(file, &content) {
                return Ok(Some(FileMutation::Text { content }));
            }

            Ok(None)
        }
    }

    /// Decide whether a text update should be applied.
    fn text_changed(&self, file: &File, content: &str) -> bool {
        match file.content.payload() {
            FileContent::Text { content: current } => current != content,
            FileContent::Binary { .. } => true,
        }
    }

    /// Decide whether a byte update should be applied.
    fn bytes_changed(&self, file: &File, bytes: &[u8]) -> bool {
        match file.content.payload() {
            FileContent::Binary { content } => content.as_slice() != bytes,
            FileContent::Text { .. } => true,
        }
    }

    /// Handle file read errors during filesystem refresh.
    fn removed_file_or_read_error(
        &self,
        path: &Path,
        error: io::Error,
    ) -> Result<Option<FileMutation>, SessionError> {
        if error.kind() == io::ErrorKind::NotFound {
            return Ok(Some(FileMutation::Removed));
        }

        Err(SessionError::ReadPathFailed {
            detail: format!("failed to read watched file {}: {error}", path.display()),
            path: path.to_path_buf(),
        })
    }
}
