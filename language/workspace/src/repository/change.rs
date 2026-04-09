use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use destack_source::{FileContent, FileId, FileType};
use smallvec::smallvec;

use crate::RevisionMode;
use crate::repository::{
    FileContentId, FileEntry, FileOrigin, Ref, Repository, RepositoryError, Revision,
    RevisionState, SourceMap, normalize_logical_path_str,
};

/// The number of recent retained revisions to preserve per retained head.
const RETAINED_FILE_HISTORY_LIMIT: usize = 32;

/// One atomic source mutation inside one repository change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Add one file with one full content payload.
    AddFile {
        /// The workspace logical path.
        logical_path: String,
        content: FileContent,
    },
    /// Set one file with one full content payload.
    SetFile {
        /// The workspace logical path.
        logical_path: String,
        content: FileContent,
    },
    /// Remove one file from the revision source snapshot.
    RemoveFile {
        /// The workspace logical path.
        logical_path: String,
    },
    /// Move one file within the revision source snapshot.
    MoveFile {
        /// The source workspace logical path.
        from: String,
        /// The destination workspace logical path.
        to: String,
    },
}

impl Edit {
    /// Build one text add edit.
    pub fn add_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::AddFile {
            logical_path: normalize_logical_path_str(path.as_ref()),
            content: FileContent::Text {
                content: content.into(),
            },
        }
    }

    /// Build one text set edit.
    pub fn set_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::SetFile {
            logical_path: normalize_logical_path_str(path.as_ref()),
            content: FileContent::Text {
                content: content.into(),
            },
        }
    }

    /// Build one remove edit.
    pub fn remove_file(path: impl AsRef<str>) -> Self {
        Self::RemoveFile {
            logical_path: normalize_logical_path_str(path.as_ref()),
        }
    }

    /// Build one move edit.
    pub fn move_file(from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::MoveFile {
            from: normalize_logical_path_str(from.as_ref()),
            to: normalize_logical_path_str(to.as_ref()),
        }
    }
}

/// One ordered batch of source edits published as one revision change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Change {
    /// The atomic edits in this change.
    pub edits: Vec<Edit>,
}

impl Change {
    /// Build one change from explicit edits.
    pub fn new(edits: Vec<Edit>) -> Self {
        Self { edits }
    }

    /// Build one empty change.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build one single-edit change.
    pub fn single(edit: Edit) -> Self {
        Self { edits: vec![edit] }
    }

    /// Build one single-file add change.
    pub fn add_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::single(Edit::add_text(path, content))
    }

    /// Build one single-file set change.
    pub fn set_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::single(Edit::set_text(path, content))
    }

    /// Build one single-file remove change.
    pub fn remove_file(path: impl AsRef<str>) -> Self {
        Self::single(Edit::remove_file(path))
    }

    /// Build one single-file move change.
    pub fn move_file(from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::single(Edit::move_file(from, to))
    }

    /// Return the edits in this change.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }
}

impl From<Edit> for Change {
    fn from(edit: Edit) -> Self {
        Self::single(edit)
    }
}

impl From<Vec<Edit>> for Change {
    fn from(edits: Vec<Edit>) -> Self {
        Self::new(edits)
    }
}

impl<const N: usize> From<[Edit; N]> for Change {
    fn from(edits: [Edit; N]) -> Self {
        Self::new(Vec::from(edits))
    }
}

impl Repository {
    /// Prune file revisions and file contents that are no longer reachable.
    pub fn prune_unreachable_file_state(&self) {
        let reachable_revisions = self.reachable_file_revisions();
        let reachable_file_contents = self.reachable_file_content_ids(&reachable_revisions);

        self.compact_retained_file_history(&reachable_revisions);
        self.revisions
            .retain(|revision, _| reachable_revisions.contains(revision));
        self.files.retain_reachable(&reachable_file_contents);
    }

    /// Collect all file revisions reachable from refs and pinned snapshots.
    fn reachable_file_revisions(&self) -> HashSet<Revision> {
        let mut reachable = HashSet::new();
        let mut visited_depths = HashMap::new();
        let mut pending = self.retained_revisions();
        let history_limit = self.history_depth_limit();

        while let Some((revision, depth)) = pending.pop() {
            if visited_depths
                .get(&revision)
                .is_some_and(|best_depth| *best_depth <= depth)
            {
                continue;
            }

            visited_depths.insert(revision, depth);
            reachable.insert(revision);

            let Some(revision_data) = self.revisions.get(&revision) else {
                continue;
            };

            // keep only the recent retained ancestry
            if depth >= history_limit.saturating_sub(1) {
                continue;
            }

            for parent in &revision_data.parents {
                pending.push((*parent, depth + 1));
            }
        }

        reachable
    }

    /// Collect all file content ids reachable from one revision set.
    fn reachable_file_content_ids(
        &self,
        reachable_revisions: &HashSet<Revision>,
    ) -> HashSet<FileContentId> {
        let mut reachable = HashSet::new();

        for revision in reachable_revisions {
            let Some(revision_data) = self.revisions.get(revision) else {
                continue;
            };

            for entry in revision_data.source.values() {
                reachable.insert(entry.content_id);
            }
        }

        reachable
    }

    /// Compact retained revision ancestry to the configured history depth.
    fn compact_retained_file_history(&self, reachable_revisions: &HashSet<Revision>) {
        let history_limit = self.history_depth_limit();
        let mut visited_depths = HashMap::new();
        let mut pending = self.retained_revisions();

        while let Some((revision, depth)) = pending.pop() {
            if visited_depths
                .get(&revision)
                .is_some_and(|best_depth| *best_depth <= depth)
            {
                continue;
            }

            visited_depths.insert(revision, depth);

            let Some(revision_entry) = self.revisions.get(&revision) else {
                continue;
            };

            let revision_data = Arc::clone(revision_entry.value());
            drop(revision_entry);
            let is_boundary = depth >= history_limit.saturating_sub(1);

            // compact the retained boundary
            if is_boundary && !revision_data.parents.is_empty() {
                let compacted_revision = Arc::new(RevisionState::new(
                    smallvec::SmallVec::new(),
                    Arc::clone(&revision_data.source),
                    Arc::clone(&revision_data.ambient),
                ));

                self.revisions.insert(revision, compacted_revision);
                continue;
            }

            for parent in &revision_data.parents {
                if reachable_revisions.contains(parent) {
                    pending.push((*parent, depth + 1));
                }
            }
        }
    }

    /// Return the retained history depth for one pinned revision.
    fn history_depth_limit(&self) -> usize {
        RETAINED_FILE_HISTORY_LIMIT
    }

    /// Collect all retained revisions with zero ancestry depth.
    fn retained_revisions(&self) -> Vec<(Revision, usize)> {
        let mut retained_revisions = self
            .refs
            .iter()
            .map(|entry| (*entry.value(), 0_usize))
            .collect::<Vec<_>>();

        // pinned snapshots
        retained_revisions.extend(
            self.pinned_revisions
                .iter()
                .map(|entry| (*entry.key(), 0_usize)),
        );

        retained_revisions
    }

    /// Seed the synthetic root source into one ref.
    pub fn seed_root_source(&self, reference: &Ref) -> Result<Revision, RepositoryError> {
        let mut seeded_source = SourceMap::new();

        self.insert_seeded_source(
            &mut seeded_source,
            FileOrigin::root(),
            FileContent::Text {
                content: String::new(),
            },
        );

        self.apply_seeded_source(reference, seeded_source)
    }

    /// Create one new ref pointing at another ref's current revision.
    pub fn fork(&self, from: &Ref, to: Ref) -> Result<Revision, RepositoryError> {
        let revision = self.current(from)?;
        self.refs.insert(to, revision);
        self.prune_unreachable_file_state();

        Ok(revision)
    }

    /// Point one ref at one already published revision.
    pub fn point(&self, reference: &Ref, revision: Revision) -> Result<Revision, RepositoryError> {
        let _revision = self.revision(revision)?;
        self.refs.insert(reference.clone(), revision);
        self.prune_unreachable_file_state();

        Ok(revision)
    }

    /// Apply one change set to one ref and publish one new revision.
    pub fn apply<C>(&self, reference: &Ref, change: C) -> Result<Revision, RepositoryError>
    where
        C: Into<Change>,
    {
        let base_revision_id = self.current(reference)?;
        let revision_id = self.apply_to_revision(base_revision_id, change)?;
        self.refs.insert(reference.clone(), revision_id);
        self.prune_unreachable_file_state();

        Ok(revision_id)
    }

    /// Apply one change set to one base revision and publish one anonymous revision.
    pub fn apply_to_revision<C>(
        &self,
        base_revision_id: Revision,
        change: C,
    ) -> Result<Revision, RepositoryError>
    where
        C: Into<Change>,
    {
        let base_revision = self.revision(base_revision_id)?;
        let source = base_revision.source.as_ref().clone();
        let source = self.apply_change(source, change.into())?;
        let revision = Arc::new(self.build_revision_data_with_mode(
            base_revision.mode,
            smallvec![base_revision_id],
            source,
            Arc::clone(&base_revision.ambient),
        ));
        let revision_id = revision.revision();

        self.revisions
            .entry(revision_id)
            .or_insert_with(|| Arc::clone(&revision));

        Ok(revision_id)
    }

    /// Load one workspace file payload from the attached file system.
    pub fn load_workspace_file_content(&self, path: &Path) -> Result<FileContent, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(path);

        if file_type.is_binary() {
            let content =
                self.fs
                    .read(path)
                    .map_err(|error| RepositoryError::ImportFileSystem {
                        operation: "read",
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    })?;

            return Ok(FileContent::Binary { content });
        }

        let content =
            self.fs
                .read_to_string(path)
                .map_err(|error| RepositoryError::ImportFileSystem {
                    operation: "read_to_string",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

        Ok(FileContent::Text { content })
    }

    /// Apply one explicit seeded source map to one ref.
    pub(crate) fn apply_seeded_source(
        &self,
        reference: &Ref,
        seeded_source: SourceMap,
    ) -> Result<Revision, RepositoryError> {
        let base_revision_id = self.current(reference)?;
        let base_revision = self.revision(base_revision_id)?;
        let mut source = base_revision.source.as_ref().clone();

        for (file_id, entry) in seeded_source {
            source.insert(file_id, entry);
        }

        let revision = Arc::new(self.build_revision_data_with_mode(
            base_revision.mode,
            smallvec![base_revision_id],
            source,
            Arc::clone(&base_revision.ambient),
        ));
        let revision_id = revision.revision();

        self.revisions
            .entry(revision_id)
            .or_insert_with(|| Arc::clone(&revision));
        self.refs.insert(reference.clone(), revision_id);
        self.prune_unreachable_file_state();

        Ok(revision_id)
    }

    /// Build one full revision state from one source map.
    fn build_revision_data(
        &self,
        parents: smallvec::SmallVec<[Revision; 2]>,
        source: SourceMap,
        ambient: Arc<crate::repository::AmbientSnapshot>,
    ) -> RevisionState {
        RevisionState::new(parents, Arc::new(source), ambient)
    }

    /// Build one full mutable revision state from one source map.
    fn build_mutable_revision_data(
        &self,
        parents: smallvec::SmallVec<[Revision; 2]>,
        source: SourceMap,
        ambient: Arc<crate::repository::AmbientSnapshot>,
    ) -> RevisionState {
        RevisionState::new_mutable(parents, Arc::new(source), ambient)
    }

    /// Build one full revision state from one source map and mode.
    fn build_revision_data_with_mode(
        &self,
        mode: RevisionMode,
        parents: smallvec::SmallVec<[Revision; 2]>,
        source: SourceMap,
        ambient: Arc<crate::repository::AmbientSnapshot>,
    ) -> RevisionState {
        match mode {
            RevisionMode::Mutable => self.build_mutable_revision_data(parents, source, ambient),
            RevisionMode::Immutable => self.build_revision_data(parents, source, ambient),
        }
    }

    /// Fork one immutable or mutable revision into a mutable working revision.
    pub fn fork_mutable_revision(
        &self,
        base_revision_id: Revision,
    ) -> Result<Revision, RepositoryError> {
        let base_revision = self.revision(base_revision_id)?;

        if base_revision.is_mutable() {
            return Ok(base_revision_id);
        }

        let source = base_revision.source.as_ref().clone();
        let revision = Arc::new(self.build_mutable_revision_data(
            smallvec![base_revision_id],
            source,
            Arc::clone(&base_revision.ambient),
        ));
        let revision_id = revision.revision();

        self.revisions
            .entry(revision_id)
            .or_insert_with(|| Arc::clone(&revision));

        Ok(revision_id)
    }

    /// Freeze one mutable revision into an immutable revision.
    pub fn freeze_revision(&self, revision_id: Revision) -> Result<Revision, RepositoryError> {
        let revision = self.revision(revision_id)?;

        if revision.is_immutable() {
            return Ok(revision_id);
        }

        let source = revision.source.as_ref().clone();
        let frozen = Arc::new(self.build_revision_data(
            smallvec![revision_id],
            source,
            Arc::clone(&revision.ambient),
        ));
        let frozen_revision_id = frozen.revision();

        self.revisions
            .entry(frozen_revision_id)
            .or_insert_with(|| Arc::clone(&frozen));

        Ok(frozen_revision_id)
    }

    /// Insert one explicit seeded source binding.
    pub(crate) fn insert_seeded_source(
        &self,
        source: &mut SourceMap,
        origin: FileOrigin,
        content: FileContent,
    ) {
        match &origin {
            FileOrigin::Workspace { logical_path } => {
                panic!("seeded source must not be one workspace path: {logical_path}");
            }
            FileOrigin::Builtin { logical_path } => {
                if self.builtins.module_source_for_path(logical_path).is_none() {
                    panic!("seeded builtin source must exist: {logical_path}");
                }
            }
            FileOrigin::Synthetic { .. } => {}
        }

        let file_id = self.file_id_for_origin(&origin);
        let content_id = self.files.intern(content);

        source.insert(file_id, FileEntry::loaded(origin, content_id));
    }

    /// Validate one workspace logical path.
    fn validate_workspace_logical_path(&self, logical_path: &str) -> Result<(), RepositoryError> {
        if logical_path.starts_with("builtin://") {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "builtin source is repository seeded".to_string(),
            });
        }

        if logical_path.starts_with("synthetic://") {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "synthetic source is repository seeded".to_string(),
            });
        }

        if logical_path.contains("://") {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "non workspace source is not writable through repository edits"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// Return one file id for one validated workspace logical path.
    fn file_id_for_workspace_logical_path(
        &self,
        logical_path: &str,
    ) -> Result<FileId, RepositoryError> {
        self.validate_workspace_logical_path(logical_path)?;

        Ok(self.file_id_for_logical_path(logical_path))
    }

    /// Apply one change set to one source map.
    fn apply_change(
        &self,
        mut source: SourceMap,
        change: Change,
    ) -> Result<SourceMap, RepositoryError> {
        for edit in change.edits {
            match edit {
                // write the requested file payload
                Edit::AddFile {
                    logical_path,
                    content,
                } => {
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    if source.contains_key(&file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: logical_path });
                    }

                    let file_id = self.file_id_for_logical_path(&logical_path);
                    let content = self.files.intern(content);
                    source.insert(
                        file_id,
                        FileEntry::loaded(FileOrigin::workspace(logical_path), content),
                    );
                }

                // set the requested file payload
                Edit::SetFile {
                    logical_path,
                    content,
                } => {
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    let content = self.files.intern(content);
                    source.insert(
                        file_id,
                        FileEntry::loaded(FileOrigin::workspace(logical_path), content),
                    );
                }

                // remove the requested file payload
                Edit::RemoveFile { logical_path } => {
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    if !source.contains_key(&file_id) {
                        return Err(RepositoryError::MissingFile { path: logical_path });
                    }

                    source.remove(&file_id);
                }

                // move one existing file binding
                Edit::MoveFile { from, to } => {
                    if from == to {
                        continue;
                    }

                    let from_file_id = self.file_id_for_workspace_logical_path(&from)?;
                    if !source.contains_key(&from_file_id) {
                        return Err(RepositoryError::MissingFile { path: from });
                    }
                    let Some(from_file) = source.get(&from_file_id).cloned() else {
                        return Err(RepositoryError::MissingFile { path: from });
                    };
                    let to_file_id = self.file_id_for_workspace_logical_path(&to)?;
                    if source.contains_key(&to_file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: to });
                    }

                    source.remove(&from_file_id);
                    let to_file_id = self.file_id_for_logical_path(&to);
                    let to_file =
                        FileEntry::loaded(FileOrigin::workspace(to), from_file.content_id);

                    source.insert(to_file_id, to_file);
                }
            }
        }

        Ok(source)
    }
}
