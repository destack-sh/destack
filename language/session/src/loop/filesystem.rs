use std::collections::HashSet;
use std::io;
use std::path::Path;

use destack_source::{File, FileContent, FileId, ModuleId};
use destack_workspace::{Module, Repository, Revision};

use crate::session::FileChange;
use crate::{FileMutation, FileUpdate, Session, SessionError};

impl Session {
    /// Discover filesystem state for this session root.
    pub fn discover_filesystem(&self) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();

        self.discover_filesystem_locked()
    }

    /// Reload tracked filesystem state for this session root.
    pub fn reload_filesystem(&self) -> Result<Vec<FileUpdate>, SessionError> {
        let _mutation_guard = self.enter_mutation();

        self.reload_filesystem_locked()
    }

    /// Discover session files into the current mutable revision.
    fn discover_session_files(&self) -> Result<(Revision, Vec<FileChange>), SessionError> {
        let mut pending_directories = vec![self.root().to_path_buf()];
        let mut visited_directories = HashSet::new();
        let repository = self.repository();
        let mut revision = self.revision();
        let mut file_changes = Vec::new();

        while let Some(directory) = pending_directories.pop() {
            if !visited_directories.insert(directory.clone()) {
                continue;
            }

            let entries = match repository.file_system().read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) => {
                    return Err(SessionError::ReadPathFailed {
                        detail: format!(
                            "failed to read session directory {}: {error}",
                            directory.display(),
                        ),
                        path: directory.clone(),
                    });
                }
            };

            for entry in entries {
                let metadata = match repository.file_system().metadata(&entry) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        return Err(SessionError::ReadPathFailed {
                            detail: format!(
                                "failed to read session metadata {}: {error}",
                                entry.display(),
                            ),
                            path: entry.clone(),
                        });
                    }
                };

                // recurse into visible directories
                if metadata.is_directory {
                    if self.should_scan_directory(&entry) {
                        pending_directories.push(entry);
                    }

                    continue;
                }

                // ignore non-file entries
                if !metadata.is_file {
                    continue;
                }

                // load newly discovered config files
                if self.is_scannable_config_path(&entry) {
                    let file_id = repository.file_id_for_workspace_path(&entry);
                    let has_file = repository
                        .file(revision, file_id)
                        .map_err(SessionError::from)?;
                    if has_file.is_some() {
                        continue;
                    }

                    let mutation = self.file_mutation_for_path(&entry).map_err(|error| {
                        SessionError::ReadPathFailed {
                            detail: format!(
                                "failed to read session config {}: {error}",
                                entry.display(),
                            ),
                            path: entry.clone(),
                        }
                    })?;
                    revision = self.apply_file_update_to_revision(
                        repository.as_ref(),
                        revision,
                        entry.as_path(),
                        mutation,
                    )?;

                    let file_change =
                        self.file_change(repository.as_ref(), revision, entry.as_path(), file_id)?;
                    file_changes.push(file_change);
                    continue;
                }

                // admit newly discovered module files
                if !self.is_scannable_module_path(&entry) {
                    continue;
                }

                if self
                    .existing_module_id_for_path(revision, &entry)?
                    .is_some()
                {
                    continue;
                }

                let (next_revision, _) =
                    self.admit_module_for_revision(revision, &entry)
                        .map_err(|error| SessionError::ResolvePathFailed {
                            path: entry.clone(),
                            detail: format!("failed to discover session module: {error}"),
                        })?;
                let file_id = repository.file_id_for_workspace_path(&entry);
                let file_change =
                    self.file_change(repository.as_ref(), next_revision, entry.as_path(), file_id)?;
                revision = next_revision;
                file_changes.push(file_change);
            }
        }

        Ok((revision, file_changes))
    }

    /// Discover filesystem state while already holding the mutation lock.
    pub(super) fn discover_filesystem_locked(&self) -> Result<Vec<FileUpdate>, SessionError> {
        let (revision, file_changes) = self.discover_session_files()?;

        self.publish_file_update(revision, file_changes, Default::default(), None)
    }

    /// Reload tracked filesystem state while already holding the mutation lock.
    pub(super) fn reload_filesystem_locked(&self) -> Result<Vec<FileUpdate>, SessionError> {
        let repository = self.repository();
        let repository = repository.as_ref();
        let mut revision = self.revision();
        let file_ids = self.reload_file_ids(repository, revision)?;
        let mut file_changes = Vec::new();

        for file_id in file_ids {
            let mutation = match self.reload_file_mutation(repository, revision, file_id) {
                Ok(Some(mutation)) => mutation,
                Ok(None) => continue,
                Err(error) => return Err(error),
            };

            let Some(file) = repository
                .file(revision, file_id)
                .map_err(SessionError::from)?
            else {
                continue;
            };
            let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
                continue;
            };

            revision =
                self.apply_file_update_to_revision(repository, revision, path.as_path(), mutation)?;

            let file_change = self.file_change(repository, revision, path.as_path(), file_id)?;
            file_changes.push(file_change);
        }

        self.publish_file_update(revision, file_changes, Default::default(), None)
    }

    /// Return true when session scanning should descend into one directory.
    fn should_scan_directory(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };

        !matches!(name, ".git" | "node_modules" | "target")
    }

    /// Return true when session scanning should admit one config file.
    fn is_scannable_config_path(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        matches!(name, "package.json" | "destack.json")
            || (name.starts_with("tsconfig") && name.ends_with(".json"))
    }

    /// Return true when session scanning should admit one module file.
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

        if destack_source::FileType::from_path(path).is_some_and(|file_type| file_type.is_binary())
        {
            let content = repository.file_system().read(path)?;

            return Ok(FileMutation::Bytes { content });
        }

        let content = repository.file_system().read_to_string(path)?;

        Ok(FileMutation::Text { content })
    }

    /// Return true when watch mode should track one path.
    pub fn is_watchable_path(&self, path: &Path) -> bool {
        let Some(file_type) = destack_source::FileType::from_path(path) else {
            return false;
        };

        file_type.is_code()
            || file_type.is_data()
            || file_type.is_text()
            || file_type.is_binary()
            || crate::FileChangeKind::for_path(path).is_config_change()
    }

    /// Return true when a module is part of this session surface.
    pub(super) fn is_session_module(&self, module: &Module) -> bool {
        let Some(path) = &module.path else {
            return false;
        };

        module.is_user() && path.starts_with(self.root())
    }

    /// Return true when the module id maps to one session visible module.
    pub(super) fn is_session_module_id(
        &self,
        repository: &Repository,
        revision: Revision,
        module_id: ModuleId,
    ) -> bool {
        let Ok(Some(module)) = repository.module(revision, module_id) else {
            return false;
        };

        self.is_session_module(&module)
    }

    /// Return the session module ids visible in one revision.
    fn session_module_ids(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<Vec<ModuleId>, SessionError> {
        let mut module_ids = Vec::new();

        for module_id in repository
            .workspace_module_ids(revision)
            .map_err(SessionError::from)?
        {
            if self.is_session_module_id(repository, revision, module_id) {
                module_ids.push(module_id);
            }
        }

        Ok(module_ids)
    }

    /// Collect the tracked file ids that filesystem reload should refresh.
    fn reload_file_ids(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<HashSet<FileId>, SessionError> {
        let mut file_ids = HashSet::new();
        let mut package_ids = HashSet::new();

        if let Some(workspace_config) = repository.workspace_destack_declaration(revision)? {
            let path = workspace_config.path.clone();
            if path.starts_with(self.root()) {
                file_ids.insert(workspace_config.file_id);
            }
        }

        for module_id in self.session_module_ids(repository, revision)? {
            let module = self.module_for_revision(repository, revision, module_id)?;
            file_ids.insert(module.file_id);

            if package_ids.insert(module.package_id) {
                let Some(package) = repository
                    .package(revision, module.package_id)
                    .map_err(SessionError::from)?
                else {
                    return Err(SessionError::Internal {
                        detail: format!("missing package snapshot for {:?}", module.package_id),
                    });
                };

                if let Some(config) = repository.package_destack_declaration(revision, &package)? {
                    file_ids.insert(config.file_id);
                }
            }

            if let Some(tsconfig_file_id) = module.tsconfig_file_id
                && let Some(tsconfig) = repository
                    .tsconfig_declaration(revision, tsconfig_file_id)
                    .map_err(SessionError::from)?
            {
                file_ids.insert(tsconfig.file_id);
            }
        }

        Ok(file_ids)
    }

    /// Build a file mutation by reading the latest content from disk.
    fn reload_file_mutation(
        &self,
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<FileMutation>, SessionError> {
        let Some(file) = repository
            .file(revision, file_id)
            .map_err(SessionError::from)?
        else {
            return Ok(None);
        };

        let Some(path) = file.path.clone().or_else(|| file.uri.to_path_buf()) else {
            return Ok(None);
        };

        if file.ty.is_binary() {
            let bytes = match repository.file_system().read(&path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return self.handle_reload_file_read_error(path.as_path(), error);
                }
            };

            if self.bytes_changed(&file, &bytes) {
                return Ok(Some(FileMutation::Bytes { content: bytes }));
            }

            return Ok(None);
        }

        let content = match repository.file_system().read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                return self.handle_reload_file_read_error(path.as_path(), error);
            }
        };

        if self.text_changed(&file, &content) {
            return Ok(Some(FileMutation::Text { content }));
        }

        Ok(None)
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

    /// Handle file read errors during filesystem reload.
    fn handle_reload_file_read_error(
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
