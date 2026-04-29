use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use destack_source::{File, FileContent, FileId, FileType, ModuleId};
use destack_workspace::{Edit, Ref, Repository, Revision};

use crate::session::FileChange;
use crate::{FileChangeKind, FileMutation, Session, SessionChange, SessionError};

impl Session {
    /// Load source state from disk into one ref.
    pub fn load(&self, reference: &Ref) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let before = self.revision(reference)?;
        let (after, file_changes) = self.load_revision(before)?;

        self.advance(
            reference,
            before,
            after,
            file_changes,
            Default::default(),
            None,
        )
    }

    /// Refresh tracked source state from disk into one ref.
    pub fn refresh(&self, reference: &Ref) -> Result<SessionChange, SessionError> {
        let _mutation_guard = self.enter_mutation();
        let before = self.revision(reference)?;
        let (after, file_changes) = self.refresh_revision(before)?;

        self.advance(
            reference,
            before,
            after,
            file_changes,
            Default::default(),
            None,
        )
    }

    /// Load session files into a revision derived from one base revision.
    fn load_revision(
        &self,
        base_revision: Revision,
    ) -> Result<(Revision, Vec<FileChange>), SessionError> {
        let mut pending_directories = vec![self.root().to_path_buf()];
        let mut visited_directories = HashSet::new();
        let repository = self.repository();
        let mut revision = base_revision;
        let mut file_changes = Vec::new();

        while let Some(directory) = pending_directories.pop() {
            // skip directories already reached through another path
            if visited_directories.insert(directory.clone()) {
                let entries = repository
                    .file_system()
                    .read_dir(&directory)
                    .map_err(|error| SessionError::ReadPathFailed {
                        detail: format!(
                            "failed to read session directory {}: {error}",
                            directory.display(),
                        ),
                        path: directory.clone(),
                    })?;

                // fold discovered files into the staged revision
                for entry in entries {
                    if let Some((next_revision, file_change)) = self.load_entry(
                        repository.as_ref(),
                        revision,
                        entry.as_path(),
                        &mut pending_directories,
                    )? {
                        revision = next_revision;
                        file_changes.push(file_change);
                    }
                }
            }
        }

        Ok((revision, file_changes))
    }

    /// Load one filesystem entry when it contributes to the session revision.
    fn load_entry(
        &self,
        repository: &Repository,
        revision: Revision,
        entry: &Path,
        pending_directories: &mut Vec<PathBuf>,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        let metadata = repository.file_system().metadata(entry).map_err(|error| {
            SessionError::ReadPathFailed {
                detail: format!(
                    "failed to read session metadata {}: {error}",
                    entry.display(),
                ),
                path: entry.to_path_buf(),
            }
        })?;

        // directory entry
        if metadata.is_directory {
            if self.should_scan_directory(entry) {
                pending_directories.push(entry.to_path_buf());
            }

            return Ok(None);
        }

        // special file entries
        if metadata.is_file && self.is_scannable_config_path(entry) {
            return self.load_config_entry(repository, revision, entry);
        }

        // module file entries
        if metadata.is_file && self.is_scannable_module_path(entry) {
            return self.load_module_entry(repository, revision, entry);
        }

        Ok(None)
    }

    /// Load one config file entry when it is not tracked yet.
    fn load_config_entry(
        &self,
        repository: &Repository,
        revision: Revision,
        entry: &Path,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        let file_id = repository.file_id_for_workspace_path(entry);
        let file = repository
            .file(revision, file_id)
            .map_err(SessionError::from)?;

        // already tracked config
        if file.is_some() {
            return Ok(None);
        }

        let mutation =
            self.file_mutation_for_path(entry)
                .map_err(|error| SessionError::ReadPathFailed {
                    detail: format!("failed to read session config {}: {error}", entry.display()),
                    path: entry.to_path_buf(),
                })?;
        let revision = self.apply_file_update_to_revision(repository, revision, entry, mutation)?;
        let file_change = self.file_change(repository, revision, entry, file_id)?;

        Ok(Some((revision, file_change)))
    }

    /// Load one module file entry when it is not tracked yet.
    fn load_module_entry(
        &self,
        repository: &Repository,
        revision: Revision,
        entry: &Path,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        let module_id = repository
            .module_id_for_path(revision, entry)
            .map_err(SessionError::from)?;

        // already tracked module
        if module_id.is_some() {
            return Ok(None);
        }

        let (revision, _) = self
            .load_module_revision(revision, entry)
            .map_err(|error| SessionError::ResolvePathFailed {
                path: entry.to_path_buf(),
                detail: format!("failed to load session module: {error}"),
            })?;
        let file_id = repository.file_id_for_workspace_path(entry);
        let file_change = self.file_change(repository, revision, entry, file_id)?;

        Ok(Some((revision, file_change)))
    }

    /// Load one module path into a revision when it is not tracked yet.
    pub(crate) fn load_module_revision(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<(Revision, ModuleId), SessionError> {
        let repository = self.repository();
        let path = path.to_path_buf();
        let module_id = repository
            .module_id_for_path(revision, &path)
            .map_err(SessionError::from)?;

        // reuse the existing module identity when it is already tracked
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
                detail: "loaded source file did not produce a module".to_string(),
            });
        };

        Ok((revision, module_id))
    }

    /// Refresh tracked files into a revision derived from one base revision.
    fn refresh_revision(
        &self,
        base_revision: Revision,
    ) -> Result<(Revision, Vec<FileChange>), SessionError> {
        let repository = self.repository();
        let repository = repository.as_ref();
        let file_ids = self.refresh_file_ids(repository, base_revision)?;
        let mut revision = base_revision;
        let mut file_changes = Vec::new();

        for file_id in file_ids {
            // fold changed tracked files into the staged revision
            if let Some((next_revision, file_change)) =
                self.refresh_file(repository, revision, file_id)?
            {
                revision = next_revision;
                file_changes.push(file_change);
            }
        }

        Ok((revision, file_changes))
    }

    /// Return true when session scanning should descend into one directory.
    fn should_scan_directory(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };

        !matches!(name, ".git" | "node_modules" | "target")
    }

    /// Return true when session scanning should include one config file.
    fn is_scannable_config_path(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        matches!(name, "package.json" | "destack.json")
            || (name.starts_with("tsconfig") && name.ends_with(".json"))
    }

    /// Return true when session scanning should include one module file.
    pub(crate) fn is_scannable_module_path(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        if matches!(name, "package.json" | "destack.json") {
            return false;
        }

        if name.starts_with("tsconfig") && name.ends_with(".json") {
            return false;
        }

        self.is_watchable_path(path)
    }

    /// Read one path into a file mutation payload.
    pub fn file_mutation_for_path(&self, path: &Path) -> io::Result<FileMutation> {
        let repository = self.repository();

        if FileType::from_path(path).is_some_and(|file_type| file_type.is_binary()) {
            let content = repository.file_system().read(path)?;

            return Ok(FileMutation::Bytes { content });
        }

        let content = repository.file_system().read_to_string(path)?;

        Ok(FileMutation::Text { content })
    }

    /// Return true when watch mode should track one path.
    pub fn is_watchable_path(&self, path: &Path) -> bool {
        let Some(file_type) = FileType::from_path(path) else {
            return false;
        };

        file_type.is_code()
            || file_type.is_data()
            || file_type.is_text()
            || file_type.is_binary()
            || FileChangeKind::for_path(path).is_config_change()
    }

    /// Collect the tracked file ids that refresh should read from disk.
    fn refresh_file_ids(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<HashSet<FileId>, SessionError> {
        let mut file_ids = HashSet::new();
        let mut package_ids = HashSet::new();

        if let Some(workspace_config) = repository.destack_declaration_for_workspace(revision)? {
            let path = workspace_config.path.clone();
            if path.starts_with(self.root()) {
                file_ids.insert(workspace_config.file_id);
            }
        }

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
            let is_session_module = module
                .path
                .as_ref()
                .is_some_and(|path| module.is_user() && path.starts_with(self.root()));

            // refresh source files and config files for modules owned by this session root
            if is_session_module {
                file_ids.insert(module.file_id);

                if package_ids.insert(module.package_id) {
                    // load package
                    let Some(package) = repository
                        .package(revision, module.package_id)
                        .map_err(SessionError::from)?
                    else {
                        return Err(SessionError::Internal {
                            detail: format!("missing package for {:?}", module.package_id),
                        });
                    };

                    // add package manifest file
                    if let Some(package_file_id) = package.package_file_id {
                        file_ids.insert(package_file_id);
                    }

                    // add package destack config file
                    if let Some(config) =
                        repository.destack_declaration_for_package(revision, &package)?
                    {
                        file_ids.insert(config.file_id);
                    }
                }

                // add tsconfig file
                if let Some(tsconfig_file_id) = module.tsconfig_file_id
                    && let Some(tsconfig) = repository
                        .tsconfig_declaration_for_file(revision, tsconfig_file_id)
                        .map_err(SessionError::from)?
                {
                    file_ids.insert(tsconfig.file_id);
                }
            }
        }

        Ok(file_ids)
    }

    /// Refresh one tracked file when its filesystem content changed.
    fn refresh_file(
        &self,
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<(Revision, FileChange)>, SessionError> {
        // load file
        let Some(file) = repository
            .file(revision, file_id)
            .map_err(SessionError::from)?
        else {
            return Ok(None);
        };
        let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
            return Ok(None);
        };

        // check for file mutation
        let Some(mutation) = self.get_file_mutation_maybe(repository, &file, &path)? else {
            return Ok(None);
        };

        let revision =
            self.apply_file_update_to_revision(repository, revision, path.as_path(), mutation)?;
        let file_change = self.file_change(repository, revision, path.as_path(), file_id)?;

        Ok(Some((revision, file_change)))
    }

    /// Build a file mutation when current filesystem content differs.
    fn get_file_mutation_maybe(
        &self,
        repository: &Repository,
        file: &File,
        path: &Path,
    ) -> Result<Option<FileMutation>, SessionError> {
        let path = path.to_path_buf();

        // make binary mutation
        if file.ty.is_binary() {
            let bytes = match repository.file_system().read(&path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return self.handle_refresh_file_read_error(path.as_path(), error);
                }
            };

            if self.bytes_changed(file, &bytes) {
                return Ok(Some(FileMutation::Bytes { content: bytes }));
            }

            Ok(None)
        }
        // make text mutation
        else {
            let content = match repository.file_system().read_to_string(&path) {
                Ok(content) => content,
                Err(error) => {
                    return self.handle_refresh_file_read_error(path.as_path(), error);
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
    fn handle_refresh_file_read_error(
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
