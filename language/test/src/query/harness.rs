use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_artifact::ArtifactKey;
use destack_compiler::Compiler;
use destack_source::{FileId, FileType, MemoryFileSystem, ModuleId};
use destack_workspace::{ProfileId, Ref, Repository, Revision};

use super::{TestMarkers, parse_markers};
use crate::core::{
    SharedMemoryWorkspace, default_profile_id_for_module, module_id_for_path,
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
    /// The time spent compiling DIR artifacts.
    compile: Duration,
    /// The time spent indexing module-backed query slices.
    index_modules: Duration,
    /// The time spent indexing repository-backed import slices.
    index_imports: Duration,
    /// The time spent rebuilding file and marker metadata.
    rebuild_metadata: Duration,
}

/// The workspace query indexes needed by one query test case.
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryIndexRequirements {
    /// Whether module-backed query indexes are required.
    pub needs_module_indexes: bool,
    /// Whether repository-backed import indexes are required.
    pub needs_import_indexes: bool,
}

impl QueryIndexRequirements {
    /// Build requirements for one query test case.
    pub fn for_mdtest(test: &MdTestCase) -> Self {
        let mut requirements = Self::default();

        for kind in query_kinds_for_mdtest(test) {
            requirements.include_query_kind(kind);
        }

        requirements
    }

    /// Expand requirements for one query kind.
    pub fn include_query_kind(&mut self, kind: &str) {
        // import search surfaces
        if matches!(kind, "completion" | "code_actions") {
            self.needs_import_indexes = true;
        }

        // syntax local surfaces don't need workspace indexes
        if !matches!(
            kind,
            "document_link"
                | "resolve_document_link"
                | "document_symbols"
                | "folding_ranges"
                | "selection_range"
                | "semantic_tokens"
                | "semantic_tokens_range"
                | "extract_function"
                | "extract_variable"
        ) {
            self.needs_module_indexes = true;
        }
    }
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
            QueryIndexRequirements {
                needs_module_indexes: true,
                needs_import_indexes: true,
            },
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
            QueryIndexRequirements {
                needs_module_indexes: true,
                needs_import_indexes: true,
            },
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
        let index_requirements = QueryIndexRequirements::for_mdtest(test);

        Self::from_mdtest_with_repository(test, repository, memory_fs, root, index_requirements)
    }

    /// Create a test session from a markdown test case using a shared repository.
    pub fn from_mdtest_with_repository(
        test: &MdTestCase,
        repository: Arc<Repository>,
        memory_fs: Arc<MemoryFileSystem>,
        root: PathBuf,
        index_requirements: QueryIndexRequirements,
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
            index_requirements,
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

    /// Build one compiled query test session from clean files.
    fn from_clean_files_with_repository(
        clean_files: Vec<(String, String, TestMarkers)>,
        primary_name: String,
        repository: Arc<Repository>,
        memory_fs: Arc<MemoryFileSystem>,
        root: PathBuf,
        profile_mode: QueryTestProfileMode<'_>,
        index_requirements: QueryIndexRequirements,
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
        let (revision, modules_by_path) = compile_and_index_query_modules(
            &repository,
            &root,
            &clean_files,
            &main_path,
            profile_mode,
            index_requirements,
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

/// Resolve, compile, and index one query test module set.
fn compile_and_index_query_modules(
    repository: &Arc<Repository>,
    root: &Path,
    clean_files: &[(String, String, TestMarkers)],
    main_path: &PathBuf,
    profile_mode: QueryTestProfileMode<'_>,
    index_requirements: QueryIndexRequirements,
    timings: &mut QuerySessionBuildTimings,
) -> (Revision, HashMap<PathBuf, ModuleId>) {
    let compiler = Compiler::new(repository.clone());

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

    // choose the profile set for every compiled module
    let mut profiles_by_module = HashMap::<ModuleId, HashSet<ProfileId>>::new();
    let extra_profile = match profile_mode {
        QueryTestProfileMode::Default => None,
        QueryTestProfileMode::MdTest(test) => {
            let (profile, _load_libraries) =
                select_profile_for_mdtest(repository, revision, main_module_id, test, false);
            Some(profile.id())
        }
    };

    for module_id in &module_ids {
        let default_profile = default_profile_id_for_module(repository, revision, *module_id);
        let profiles = profiles_by_module.entry(*module_id).or_default();
        profiles.insert(default_profile);

        if let Some(profile) = extra_profile {
            profiles.insert(profile);
        }
    }

    // compile the checked DIR for every relevant profile
    //
    // checked depends on declared and exported DIR, so this materializes both
    // artifacts for query_context consumers
    let mut artifact_keys = Vec::new();
    let mut profile_ids = HashSet::new();
    for (module_id, profiles) in &profiles_by_module {
        for profile in profiles {
            profile_ids.insert(*profile);
            artifact_keys.push(ArtifactKey::DirChecked {
                module: *module_id,
                profile: *profile,
            });

            if index_requirements.needs_module_indexes {
                artifact_keys.push(ArtifactKey::module_query_index(*module_id, *profile));
            }
        }
    }

    if index_requirements.needs_import_indexes {
        for profile in profile_ids {
            artifact_keys.push(ArtifactKey::workspace_query_index(profile));
        }
    }

    let compiler = Arc::new(compiler);

    let compile_start = Instant::now();
    let revision = provide_workspace_artifacts(repository.clone(), compiler, &artifact_keys);
    timings.compile = compile_start.elapsed();

    (revision, modules_by_path)
}

/// Print one query session build timing line when timing is enabled.
fn log_query_session_build_timing(root: &Path, timings: &QuerySessionBuildTimings) {
    if std::env::var_os("DESTACK_QUERY_TIMING").is_none() {
        return;
    }

    let total = timings.populate_files
        + timings.resolve_modules
        + timings.compile
        + timings.index_modules
        + timings.index_imports
        + timings.rebuild_metadata;

    eprintln!(
        "query build {}: files={:?} resolve={:?} compile={:?} index_modules={:?} index_imports={:?} rebuild={:?} total={:?}",
        root.display(),
        timings.populate_files,
        timings.resolve_modules,
        timings.compile,
        timings.index_modules,
        timings.index_imports,
        timings.rebuild_metadata,
        total,
    );
}

/// Iterate the query kinds declared in one mdtest case.
fn query_kinds_for_mdtest(test: &MdTestCase) -> impl Iterator<Item = &str> + '_ {
    test.extra_blocks.iter().filter_map(|block| {
        let parts: Vec<_> = block.language.split_whitespace().collect();
        if parts.len() < 3 || parts[0] != "query" {
            return None;
        }

        Some(parts[1])
    })
}

/// Return the current workspace revision for one repository.
fn current_workspace_revision(repository: &Repository) -> Revision {
    let reference = Ref::for_workspace_root(repository.workspace_root());

    repository
        .current(&reference)
        .expect("missing current query test revision")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use destack_query::{CompletionTrigger, completions};

    use super::{QueryIndexRequirements, QueryTestSession};
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
        ));

        // resolve the compiled module for the main file
        let module = session
            .session
            .primary_module_for_file(session.file_id)
            .unwrap_or_else(|| panic!("expected compiled module for {:?}", session.file_id));
        let module = module.as_ref();

        // require the standard query context surface
        let ctx = query_context(session.session.as_ref(), module);
        assert!(ctx.is_some(), "expected query context for main file");
    }

    /// Detects statement completion context in mdtest sessions.
    #[test]
    fn test_detects_statement_completion_context_in_mdtest_session() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
const foo = 1;
$0
"#,
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require statement position at the cursor
        match result.context {
            CompletionContext::StatementPosition { .. } => {}
            other => panic!("expected statement completion context, found {other:?}"),
        }
    }

    /// Detects new-expression completion context after a bare `new` keyword.
    #[test]
    fn test_detects_new_expression_completion_context_after_new_keyword() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
class Engine {}

function main() {
    new $0
}
"#,
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require new-expression completion at the cursor
        match result.context {
            CompletionContext::NewExpression { .. } => {}
            other => panic!("expected new-expression completion context, found {other:?}"),
        }
    }

    /// Detects new-expression completion context while typing a constructor name.
    #[test]
    fn test_detects_new_expression_completion_context_for_constructor_prefix() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
class Engine {}

function main() {
    new Eng$0
}
"#,
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require new-expression completion at the cursor
        match result.context {
            CompletionContext::NewExpression { .. } => {}
            other => panic!("expected new-expression completion context, found {other:?}"),
        }
    }

    /// Detects value completion context in one missing return slot.
    #[test]
    fn test_detects_value_completion_context_after_return_keyword() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
function helper(): void {}

function main() {
    return $0
}
"#,
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require value completion at the cursor
        match result.context {
            CompletionContext::ValuePosition { .. } => {}
            other => panic!("expected value completion context, found {other:?}"),
        }
    }

    /// Detects value completion context in one missing yield slot.
    #[test]
    fn test_detects_value_completion_context_after_yield_keyword() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
function* main() {
    yield $0
}
"#,
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require value completion at the cursor
        match result.context {
            CompletionContext::ValuePosition { .. } => {}
            other => panic!("expected value completion context, found {other:?}"),
        }
    }

    /// Detects value completion context in one missing `yield*` operand slot.
    #[test]
    fn test_detects_value_completion_context_after_yield_star() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
function* main() {
    yield* $0
}
"#,
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require value completion at the cursor
        match result.context {
            CompletionContext::ValuePosition { .. } => {}
            other => panic!("expected value completion context, found {other:?}"),
        }
    }

    /// Detects value completion context in one missing throw slot.
    #[test]
    fn test_detects_value_completion_context_after_throw_keyword() {
        let session = QueryTestSession::from_mdtest(&mdtest_case(
            r#"
function main() {
    throw $0
}
"#,
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require value completion at the cursor
        match result.context {
            CompletionContext::ValuePosition { .. } => {}
            other => panic!("expected value completion context, found {other:?}"),
        }
    }

    /// Detects member access completion context inside one initializer with the same binding name.
    #[test]
    fn test_detects_member_access_context_inside_initializer_with_same_label() {
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
        ));
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // detect completion context at the cursor
        let result = detect_completion_context(session.session.as_ref(), session.file_id, cursor);

        // require member access completion at the cursor
        match result.context {
            CompletionContext::MemberAccess { .. } => {}
            other => panic!("expected member access completion context, found {other:?}"),
        }
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
        let completions = completions(
            session.repository.as_ref(),
            session.revision,
            session.file_id,
            cursor,
            CompletionTrigger::Invoked,
        );
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
        let completions = completions(
            session.repository.as_ref(),
            session.revision,
            session.file_id,
            cursor,
            CompletionTrigger::Invoked,
        );
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
            QueryIndexRequirements {
                needs_module_indexes: true,
                needs_import_indexes: true,
            },
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
            QueryIndexRequirements {
                needs_module_indexes: true,
                needs_import_indexes: true,
            },
        )
        .expect("query session");
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // request completions through the shared-session path
        let completions = completions(
            session.repository.as_ref(),
            session.revision,
            session.file_id,
            cursor,
            CompletionTrigger::Invoked,
        );
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
