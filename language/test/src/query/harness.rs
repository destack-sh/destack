use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_query::{self as query, ModuleQueryContext, WorkspaceQueryContext};
use destack_repository::{ProfileId, Ref, Repository, Revision};
use destack_source::{FileId, FileType, MemoryFileSystem, ModuleId};

use super::{TestMarkers, parse_markers};
use crate::core::{
    SharedMemoryWorkspace, module_id_for_path, profile_id_for_builtin_default_target,
    provide_workspace_artifacts, write_workspace_text_file,
};
use crate::mdtest::{MdTestCase, select_profile_for_mdtest};

/// One timing breakdown for building a query test session.
#[derive(Debug, Clone, Copy, Default)]
struct QuerySessionBuildTimings {
    /// The time spent materializing test files into the memory filesystem.
    populate_files: Duration,
    /// The time spent resolving the module set.
    resolve_modules: Duration,
    /// The time spent rebuilding file and marker metadata.
    rebuild_metadata: Duration,
}

/// Information about a single file in a test session.
#[derive(Debug, Clone)]
pub struct TestFile {
    /// The file id.
    pub file_id: FileId,
    /// The file name.
    pub name: String,
    /// The extracted markers.
    pub markers: TestMarkers,
    /// The clean source (without markers).
    pub source: String,
}

/// A test session for query testing.
#[derive(Debug)]
pub struct QueryTestSession {
    /// The shared repository.
    pub repository: Arc<Repository>,
    /// The active revision for this test session.
    pub revision: Revision,
    /// The workspace root for this test session.
    pub root: PathBuf,
    /// The primary file id.
    pub file_id: FileId,
    /// The extracted markers across every test file.
    pub markers: TestMarkers,
    /// The clean source (without markers, from primary file).
    pub source: String,
    /// The query profiles included in this session.
    pub profile_ids: Vec<ProfileId>,
    /// The selected query profile keyed by module.
    profile_by_module: HashMap<ModuleId, ProfileId>,
    /// All files in the session (keyed by file name).
    pub files: HashMap<String, TestFile>,
}

impl QueryTestSession {
    /// Create a test session from source with markers.
    pub fn from_source(source: &str) -> Self {
        let workspace = SharedMemoryWorkspace::new("/test");
        let memory_fs = workspace.fs();
        let root = workspace.root().to_path_buf();
        let repository = workspace.repository();

        // parse markers before compiling the file
        let (clean_source, markers) =
            parse_markers(FileId(0), "test.ds", source).expect("failed to parse query markers");
        let clean_files = vec![("test.ds".to_string(), clean_source, markers)];

        Self::from_clean_files_with_repository(
            clean_files,
            "test.ds".to_string(),
            repository,
            memory_fs,
            root,
            QueryTestProfileMode::Default,
        )
    }

    /// Create a test session from multiple files.
    pub fn from_files(input_files: &[(&str, &str)]) -> Self {
        let workspace = SharedMemoryWorkspace::new("/test");
        let memory_fs = workspace.fs();
        let root = workspace.root().to_path_buf();
        let repository = workspace.repository();
        let mut clean_files = Vec::new();
        let mut primary_name = None;

        // parse every file before compiling any module
        for (name, source) in input_files {
            let (clean_source, markers) =
                parse_markers(FileId(0), name, source).expect("failed to parse query markers");

            if primary_name.is_none() {
                primary_name = Some((*name).to_string());
            }

            clean_files.push(((*name).to_string(), clean_source, markers));
        }

        Self::from_clean_files_with_repository(
            clean_files,
            primary_name.expect("expected at least one query test file"),
            repository,
            memory_fs,
            root,
            QueryTestProfileMode::Default,
        )
    }

    /// Create a test session from a markdown test case.
    ///
    /// Uses the same setup as spec tests for proper initialization.
    /// Runs the compiler to populate DIR with resolved symbols.
    pub fn from_mdtest(test: &MdTestCase) -> Result<Self, String> {
        let workspace = SharedMemoryWorkspace::new("/test");
        let memory_fs = workspace.fs();
        let root = workspace.root().to_path_buf();
        let repository = workspace.repository();

        Self::from_mdtest_with_repository(test, repository, memory_fs, root)
    }

    /// Create a test session from a markdown test case using a shared repository.
    pub fn from_mdtest_with_repository(
        test: &MdTestCase,
        repository: Arc<Repository>,
        memory_fs: Arc<MemoryFileSystem>,
        root: PathBuf,
    ) -> Result<Self, String> {
        // first pass: parse markers and collect clean sources
        let mut clean_files: Vec<(String, String, TestMarkers)> = Vec::new();

        for file in &test.files {
            // parse markers with placeholder file_id (will be updated after compilation)
            let (clean_source, markers) = parse_markers(FileId(0), &file.path, &file.content)?;
            clean_files.push((file.path.clone(), clean_source, markers));
        }

        // find the primary file for this query case
        let primary_name = select_primary_file_path(test, &clean_files).to_string();

        Ok(Self::from_clean_files_with_repository(
            clean_files,
            primary_name,
            repository,
            memory_fs,
            root,
            QueryTestProfileMode::MdTest(test),
        ))
    }

    /// Get a file by name.
    pub fn file(&self, name: &str) -> Option<&TestFile> {
        self.files.get(name)
    }

    /// Get markers from a specific file.
    pub fn markers_for(&self, name: &str) -> Option<&TestMarkers> {
        self.files.get(name).map(|f| &f.markers)
    }

    /// Return the query context for the primary file.
    pub fn primary_module_context(&self) -> ModuleQueryContext<'_> {
        self.module_context(self.file_id)
    }

    /// Return the query context for one file.
    pub fn module_context(&self, file_id: FileId) -> ModuleQueryContext<'_> {
        let module_id = self
            .repository
            .module_id_for_file(self.revision, file_id)
            .expect("failed to resolve query module for file")
            .expect("missing query module for file");
        let profile_id = self.module_profile_id(module_id);
        self.require_artifacts(&[
            ArtifactKey::dir_checked(module_id, profile_id),
            ArtifactKey::global_environment(profile_id),
        ]);

        query::module_query_context(
            self.repository.as_ref(),
            self.revision,
            module_id,
            profile_id,
        )
        .expect("missing module query context")
    }

    /// Return the selected query profile for one module.
    pub fn module_profile_id(&self, module_id: ModuleId) -> ProfileId {
        self.profile_by_module
            .get(&module_id)
            .copied()
            .expect("missing query profile for module")
    }

    /// Return the workspace query context for this session.
    pub fn workspace_context(&self) -> WorkspaceQueryContext<'_> {
        let mut indexes = Vec::with_capacity(self.profile_ids.len());

        for profile_id in &self.profile_ids {
            let key = ArtifactKey::workspace_query_index(*profile_id);
            self.require_artifact(key);
            let version = self
                .repository
                .artifact_version(self.revision, &key)
                .expect("failed to resolve workspace query index version")
                .expect("missing workspace query index version");
            let index = self
                .repository
                .artifact_store()
                .workspace_query_index(&version)
                .expect("missing workspace query index payload");

            indexes.push((*profile_id, index));
        }

        query::workspace_query_context(self.repository.as_ref(), self.revision, indexes)
            .expect("missing workspace query context")
    }

    /// Require one artifact in this session revision.
    fn require_artifact(&self, key: ArtifactKey) {
        self.require_artifacts(&[key]);
    }

    /// Require artifacts in this session revision.
    fn require_artifacts(&self, keys: &[ArtifactKey]) {
        let missing_keys: Vec<_> = keys
            .iter()
            .filter(|key| {
                self.repository
                    .artifact_version(self.revision, key)
                    .expect("failed to read query artifact version")
                    .is_none()
            })
            .cloned()
            .collect();
        if missing_keys.is_empty() {
            return;
        }

        let compiler = Arc::new(Compiler::new(self.repository.clone()));
        let revision =
            provide_workspace_artifacts(self.repository.clone(), compiler, &missing_keys);

        assert_eq!(
            revision, self.revision,
            "query artifact provider changed the source revision"
        );
    }

    /// Build one compiled query test session from clean files.
    fn from_clean_files_with_repository(
        clean_files: Vec<(String, String, TestMarkers)>,
        primary_name: String,
        repository: Arc<Repository>,
        memory_fs: Arc<MemoryFileSystem>,
        root: PathBuf,
        profile_mode: QueryTestProfileMode<'_>,
    ) -> Self {
        // stage timings
        let mut timings = QuerySessionBuildTimings::default();

        // populate the in-memory filesystem first
        let populate_start = Instant::now();
        for (path, clean_source, _) in &clean_files {
            let file_path = root.join(path);
            memory_fs
                .add_file(&file_path, clean_source.as_bytes())
                .expect("failed to add test file");
        }
        timings.populate_files = populate_start.elapsed();

        // build and index the compiled module set
        let main_path = root.join(&primary_name);
        let (revision, modules_by_path, profile_by_module, profile_ids) = build_query_modules(
            &repository,
            &root,
            &clean_files,
            &main_path,
            profile_mode,
            &mut timings,
        );

        let mut files = HashMap::new();
        let mut all_markers = TestMarkers::default();
        let mut primary_file_id = None;
        let mut primary_source = None;

        // rebuild file metadata with the compiled file ids
        let rebuild_start = Instant::now();
        for (path, clean_source, markers) in &clean_files {
            let file_path = root.join(path);
            let file_id = if let Some(module_id) = modules_by_path.get(&file_path) {
                let module = repository
                    .module(revision, *module_id)
                    .expect("failed to load query test module")
                    .expect("missing query test module");

                module.file_id
            } else {
                repository.file_id(&file_path)
            };

            for range in &markers.ranges {
                all_markers.ranges.push(super::RangeMarker {
                    name: range.name.clone(),
                    span: destack_source::Span::new(file_id, range.span.start, range.span.end),
                    target: range.target.clone(),
                });
            }

            for cursor in &markers.cursors {
                all_markers.cursors.push(super::CursorMarker {
                    index: cursor.index,
                    file_id,
                    offset: cursor.offset,
                });
            }

            files.insert(
                path.clone(),
                TestFile {
                    file_id,
                    name: path.clone(),
                    markers: markers.clone(),
                    source: clean_source.clone(),
                },
            );

            if path == &primary_name {
                primary_file_id = Some(file_id);
                primary_source = Some(clean_source.clone());
            }
        }
        timings.rebuild_metadata = rebuild_start.elapsed();

        // log the build breakdown when requested
        log_query_session_build_timing(&root, &timings);

        let primary_file_id =
            primary_file_id.expect("failed to resolve the compiled primary query file");
        let primary_source = primary_source.expect("failed to capture the primary query source");

        Self {
            repository,
            revision,
            root,
            file_id: primary_file_id,
            markers: all_markers,
            source: primary_source,
            profile_ids,
            profile_by_module,
            files,
        }
    }
}

/// The profile-selection mode for one query test session.
enum QueryTestProfileMode<'a> {
    /// Compile every module through its default profile only.
    Default,
    /// Compile using the mdtest-selected profile in addition to the default profile.
    MdTest(&'a MdTestCase),
}

/// Select the primary source file for a markdown query case.
fn select_primary_file_path<'a>(
    test: &'a MdTestCase,
    clean_files: &'a [(String, String, TestMarkers)],
) -> &'a str {
    // prefer the canonical main file name first
    if let Some(file) = test.files.iter().find(|file| {
        Path::new(&file.path)
            .file_stem()
            .is_some_and(|name| name == "main")
    }) {
        return &file.path;
    }

    // otherwise prefer the unique file referenced by query markers
    let mut selected_path = None;
    for target in query_targets(test) {
        if target.starts_with('$') {
            continue;
        }

        let Some(path) = file_path_for_marker_target(clean_files, target) else {
            continue;
        };

        match selected_path {
            None => selected_path = Some(path),
            Some(existing_path) if existing_path == path => {}
            Some(_) => {
                selected_path = None;
                break;
            }
        }
    }

    if let Some(path) = selected_path {
        return path;
    }

    // otherwise prefer the first destack source file
    if let Some((path, _, _)) = clean_files.iter().find(|(path, _, _)| {
        matches!(
            FileType::from_path(Path::new(path)),
            Some(FileType::Destack | FileType::TypeScript)
        )
    }) {
        return path;
    }

    // otherwise use the first listed file
    &test.files[0].path
}

/// Iterate the query target names declared in markdown blocks.
fn query_targets(test: &MdTestCase) -> impl Iterator<Item = &str> {
    test.extra_blocks.iter().filter_map(|block| {
        let mut parts = block.language.split_whitespace();
        let head = parts.next()?;
        let _kind = parts.next()?;
        let target = parts.next()?;

        (head == "query").then_some(target)
    })
}

/// Resolve the file path that owns one marker target.
fn file_path_for_marker_target<'a>(
    clean_files: &'a [(String, String, TestMarkers)],
    target: &str,
) -> Option<&'a str> {
    for (path, _, markers) in clean_files {
        if markers.range(target).is_some() {
            return Some(path);
        }
    }

    None
}

/// Convenience function to create a test session.
pub fn test_session(source: &str) -> QueryTestSession {
    QueryTestSession::from_source(source)
}

/// Convenience function to create a multi-file test session.
pub fn test_session_multi(files: &[(&str, &str)]) -> QueryTestSession {
    QueryTestSession::from_files(files)
}

/// Resolve one query test module set.
fn build_query_modules(
    repository: &Arc<Repository>,
    root: &Path,
    clean_files: &[(String, String, TestMarkers)],
    main_path: &PathBuf,
    profile_mode: QueryTestProfileMode<'_>,
    timings: &mut QuerySessionBuildTimings,
) -> (
    Revision,
    HashMap<PathBuf, ModuleId>,
    HashMap<ModuleId, ProfileId>,
    Vec<ProfileId>,
) {
    // materialize all test files into the active revision first
    for (path, clean_source, _) in clean_files {
        let file_path = root.join(path);
        write_workspace_text_file(repository, &file_path, clean_source);
    }

    // pin the active workspace revision for this query session
    let revision = current_workspace_revision(repository);

    // resolve the primary module first
    let resolve_start = Instant::now();
    let main_module_id = module_id_for_path(repository, revision, main_path);

    let mut module_ids = vec![main_module_id];
    let mut modules_by_path = HashMap::new();
    modules_by_path.insert(main_path.to_path_buf(), main_module_id);

    // resolve the rest of the module set
    for (path, _, _) in clean_files {
        let file_path = root.join(path);
        if &file_path == main_path {
            continue;
        }

        if let Ok(Some(module_id)) = repository.module_id_for_path(revision, &file_path) {
            modules_by_path.insert(file_path, module_id);
            module_ids.push(module_id);
        }
    }

    module_ids.sort();
    module_ids.dedup();
    timings.resolve_modules = resolve_start.elapsed();

    // select one explicit profile for every query module
    let mut profile_by_module = HashMap::<ModuleId, ProfileId>::new();
    for module_id in &module_ids {
        let profile = match &profile_mode {
            QueryTestProfileMode::Default => {
                profile_id_for_builtin_default_target(repository, revision, *module_id)
            }
            QueryTestProfileMode::MdTest(test) => {
                let (profile, _load_libraries) =
                    select_profile_for_mdtest(repository, revision, *module_id, test, false);
                profile.id()
            }
        };

        profile_by_module.insert(*module_id, profile);
    }

    // collect the profiles used by this query case
    let mut profile_ids = HashSet::new();
    for profile in profile_by_module.values() {
        profile_ids.insert(*profile);
    }

    let mut profile_ids = profile_ids.into_iter().collect::<Vec<_>>();
    profile_ids.sort();
    profile_ids.dedup();

    (revision, modules_by_path, profile_by_module, profile_ids)
}

/// Print one query session build timing line when timing is enabled.
fn log_query_session_build_timing(root: &Path, timings: &QuerySessionBuildTimings) {
    if std::env::var_os("DESTACK_QUERY_TIMING").is_none() {
        return;
    }

    let total = timings.populate_files + timings.resolve_modules + timings.rebuild_metadata;

    eprintln!(
        "query build {}: files={:?} resolve={:?} rebuild={:?} total={:?}",
        root.display(),
        timings.populate_files,
        timings.resolve_modules,
        timings.rebuild_metadata,
        total,
    );
}

/// Return the current workspace revision for one repository.
fn current_workspace_revision(repository: &Repository) -> Revision {
    let reference = Ref::for_root(repository.path());

    repository
        .current(&reference)
        .expect("missing current query test revision")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use destack_query::{CompletionTrigger, completions};

    use super::QueryTestSession;
    use crate::core::SharedMemoryWorkspace;
    use crate::mdtest::{MdTestCase, MdTestFile};

    /// Builds a minimal markdown query test case.
    fn mdtest_case(source: &str) -> MdTestCase {
        MdTestCase {
            name: "local completion".to_string(),
            section: "query".to_string(),
            options: HashMap::new(),
            files: vec![MdTestFile {
                path: "main.ds".to_string(),
                content: source.to_string(),
                options: HashMap::new(),
            }],
            bullet_items: Vec::new(),
            extra_blocks: Vec::new(),
            line: 1,
            skip: false,
        }
    }

    /// Produces query context for the mdtest main file.
    #[test]
    fn test_builds_query_context_for_mdtest_main_file() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
const foo = 1;
$0
"#,
        ))
        .expect("query session");

        let context = session.primary_module_context();

        assert_eq!(context.file_id(), session.file_id);
    }

    /// Returns member completions inside one initializer even when the binding has the same label.
    #[test]
    fn test_completions_keep_member_access_inside_initializer_with_same_label() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
class Calculator {
    add(a: int32, b: int32): int32 {
        return a + b;
    }
}

function main() {
    const calc = new Calculator();
    const add = calc.ad$0
}
"#,
        ))
        .expect("query session");
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // request completions at the member access cursor
        let context = session.primary_module_context();
        let workspace = session.workspace_context();
        let completions = completions(&context, &workspace, cursor, CompletionTrigger::Invoked);
        let labels: Vec<_> = completions
            .iter()
            .map(|completion| completion.label.clone())
            .collect();

        // keep the class method completion visible
        assert!(
            labels.iter().any(|label| label == "add"),
            "expected method completion, found {labels:?}"
        );
    }

    /// Returns visible local value completions for a simple mdtest session.
    #[test]
    fn test_completions_include_visible_namespace_values_in_mdtest_session() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
const foo = 1;
$0
"#,
        ))
        .expect("query session");
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // request completions at the statement cursor
        let context = session.primary_module_context();
        let workspace = session.workspace_context();
        let completions = completions(&context, &workspace, cursor, CompletionTrigger::Invoked);
        let labels: Vec<_> = completions
            .iter()
            .map(|completion| completion.label.clone())
            .collect();

        // require the local binding to be present
        assert!(
            labels.iter().any(|label| label == "foo"),
            "expected foo completion, labels={labels:?}"
        );
    }

    /// Returns visible local value completions through the shared suite workspace path.
    #[test]
    fn test_completions_include_visible_values_in_shared_mdtest_session() {
        let workspace = SharedMemoryWorkspace::new("/test/query");

        // populate one earlier case to mirror the shared suite shape
        let first_root = workspace.allocate_root("first");
        let _ = QueryTestSession::from_mdtest_with_repository(
            &mdtest_case(
                r#"
const first = 1;
$0
"#,
            ),
            workspace.repository(),
            workspace.fs(),
            first_root,
        )
        .expect("query session");

        // build the actual target case in the same shared session
        let second_root = workspace.allocate_root("second");
        let session = QueryTestSession::from_mdtest_with_repository(
            &mdtest_case(
                r#"
const foo = 1;
$0
"#,
            ),
            workspace.repository(),
            workspace.fs(),
            second_root,
        )
        .expect("query session");
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // request completions through the shared-session path
        let context = session.primary_module_context();
        let workspace = session.workspace_context();
        let completions = completions(&context, &workspace, cursor, CompletionTrigger::Invoked);
        let labels: Vec<_> = completions
            .iter()
            .map(|completion| completion.label.clone())
            .collect();
        // require the local binding to survive shared-session reuse
        assert!(
            labels.iter().any(|label| label == "foo"),
            "expected foo completion in shared session, labels={labels:?}",
        );
    }
}
