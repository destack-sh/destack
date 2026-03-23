use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{Compiler, CompilerOptions};
use destack_source::{File, FileId, FileSystem, FileType, MemoryFileSystem, Uri};
use destack_workspace::{ArtifactKey, ProfileId, Session};

use super::{TestMarkers, parse_markers};
use crate::core::SharedMemoryWorkspace;
use crate::mdtest::{MdTestCase, select_profile_for_mdtest};

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
    /// The session.
    pub session: Arc<Session>,
    /// The workspace root for this test session.
    pub root: PathBuf,
    /// The primary file id.
    pub file_id: FileId,
    /// The extracted markers (from primary file).
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
        let fs = workspace.fs();
        let root = workspace.root().to_path_buf();
        let session = workspace.session();

        // register the test file
        let uri = Uri::from_string("file:///test/test.ds");
        let file_id = session.files.next_id();

        // parse markers and get clean source
        let (clean_source, markers) = parse_markers(file_id, source);

        // write clean source to memory fs
        let _ = fs.write(&root.join("test.ds"), clean_source.as_bytes());

        // register with session
        let program = session.add_root(root.clone());
        program.register_inline_module(uri, clean_source.clone(), FileType::Destack);

        let mut files = HashMap::new();
        files.insert(
            "test.ds".to_string(),
            TestFile {
                file_id,
                name: "test.ds".to_string(),
                markers: markers.clone(),
                source: clean_source.clone(),
            },
        );

        Self {
            session,
            root,
            file_id,
            markers,
            source: clean_source,
            files,
        }
    }

    /// Create a test session from multiple files.
    pub fn from_files(input_files: &[(&str, &str)]) -> Self {
        let workspace = SharedMemoryWorkspace::new("/test");
        let fs = workspace.fs();
        let root = workspace.root().to_path_buf();
        let session = workspace.session();
        let program = session.add_root(root.clone());

        let mut primary_file_id = None;
        let mut primary_markers = TestMarkers::default();
        let mut primary_source = String::new();
        let mut files = HashMap::new();

        for (name, source) in input_files {
            let uri = Uri::from_string(format!("file:///test/{name}"));
            let file_id = session.files.next_id();

            let (clean_source, markers) = parse_markers(file_id, source);

            let _ = fs.write(
                &PathBuf::from(format!("/test/{name}")),
                clean_source.as_bytes(),
            );
            program.register_inline_module(uri, clean_source.clone(), FileType::Destack);

            files.insert(
                name.to_string(),
                TestFile {
                    file_id,
                    name: name.to_string(),
                    markers: markers.clone(),
                    source: clean_source.clone(),
                },
            );

            // first file is primary
            if primary_file_id.is_none() {
                primary_file_id = Some(file_id);
                primary_markers = markers;
                primary_source = clean_source;
            }
        }

        Self {
            session,
            root,
            file_id: primary_file_id.unwrap(),
            markers: primary_markers,
            source: primary_source,
            files,
        }
    }

    /// Create a test session from a markdown test case.
    ///
    /// Uses the same setup as spec tests for proper initialization.
    /// Runs the compiler to populate DIR with resolved symbols.
    pub fn from_mdtest(test: &MdTestCase) -> Self {
        let workspace = SharedMemoryWorkspace::new("/test");
        let memory_fs = workspace.fs();
        let root = workspace.root().to_path_buf();
        let session = workspace.session();

        Self::from_mdtest_with_session(test, session, memory_fs, root)
    }

    /// Create a test session from a markdown test case using a shared session.
    pub fn from_mdtest_with_session(
        test: &MdTestCase,
        session: Arc<Session>,
        memory_fs: Arc<MemoryFileSystem>,
        root: PathBuf,
    ) -> Self {
        // first pass: parse markers and collect clean sources
        let mut clean_files: Vec<(String, String, TestMarkers)> = Vec::new();

        for file in &test.files {
            // parse markers with placeholder file_id (will be updated after compilation)
            let (clean_source, markers) = parse_markers(FileId(0), &file.content);
            clean_files.push((file.path.clone(), clean_source, markers));
        }

        // populate filesystem with clean sources
        for (path, clean_source, _) in &clean_files {
            let file_path = root.join(path);
            memory_fs
                .add_file(&file_path, clean_source.as_bytes())
                .expect("failed to add test file");
        }

        let program = session.add_root(root.clone());

        // create compiler and run analysis
        let mut compiler = Compiler::new(
            session.clone(),
            program.clone(),
            CompilerOptions {
                load_libraries: false,
                workers: 1,
                ..Default::default()
            },
        );

        // find the primary file for this query case
        let primary_name = select_primary_file_path(test, &clean_files);
        let main_path = root.join(primary_name);

        // resolve and compile the main module
        let main_module_id = compiler
            .resolve_path_to_module(&main_path)
            .expect("failed to resolve module");

        let (profile, load_libraries) =
            select_profile_for_mdtest(&program, main_module_id, test, false);
        compiler.options.load_libraries = load_libraries;

        // resolve all test modules so auto import queries can see exports
        let mut module_ids = vec![main_module_id];
        let mut modules_by_path = HashMap::new();
        modules_by_path.insert(main_path.clone(), main_module_id);
        for (path, _, _) in &clean_files {
            let file_path = root.join(path);
            if file_path == main_path {
                continue;
            }

            if let Ok(other_module_id) = compiler.resolve_path_to_module(&file_path) {
                modules_by_path.insert(file_path, other_module_id);
                module_ids.push(other_module_id);
            }
        }

        module_ids.sort();
        module_ids.dedup();

        let mut profiles_by_module = HashMap::<_, std::collections::HashSet<ProfileId>>::new();
        for module_id in &module_ids {
            let default_profile = program.default_profile_id_for_module(*module_id);
            let profiles = profiles_by_module.entry(*module_id).or_default();
            profiles.insert(default_profile);
            profiles.insert(profile);
        }

        for (module_id, profiles) in &profiles_by_module {
            for profile in profiles {
                // compile every test module through both analyzed and resolved dir
                // so semantic queries see a consistent prebuilt query context
                compiler.enqueue(ArtifactKey::DirAnalyzed {
                    module: *module_id,
                    profile: *profile,
                });
                compiler.enqueue(ArtifactKey::DirResolved {
                    module: *module_id,
                    profile: *profile,
                });
            }
        }

        compiler.compile();

        drop(compiler);

        // build TestFile structs with actual file IDs from compiled modules
        let mut files = HashMap::new();
        let mut all_markers = TestMarkers::default();
        let mut primary_file_id = None;
        let mut primary_source = None;

        for (path, clean_source, markers) in &clean_files {
            let file_path = root.join(path);
            let file_uri = Uri::from_path(&file_path);

            // prefer the resolved module file id when this file compiled as a module
            let file_id = if let Some(module_id) = modules_by_path.get(&file_path) {
                let module = program.modules.get(*module_id);
                module.file_id
            } else {
                ensure_test_file_id(&session, &file_path, &file_uri, clean_source)
            };

            // update marker spans with correct file_id
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

            // keep the exposed primary file aligned with the compiled main module
            if path == primary_name {
                primary_file_id = Some(file_id);
                primary_source = Some(clean_source.clone());
            }
        }

        let primary_file_id =
            primary_file_id.expect("failed to resolve the compiled primary query file");
        let primary_source = primary_source.expect("failed to capture the primary query source");

        Self {
            session,
            root,
            file_id: primary_file_id,
            markers: all_markers,
            source: primary_source,
            files,
        }
    }

    /// Get a file by name.
    pub fn file(&self, name: &str) -> Option<&TestFile> {
        self.files.get(name)
    }

    /// Get markers from a specific file.
    pub fn markers_for(&self, name: &str) -> Option<&TestMarkers> {
        self.files.get(name).map(|f| &f.markers)
    }
}

/// Select the primary source file for a markdown query case.
fn select_primary_file_path<'a>(
    test: &'a MdTestCase,
    clean_files: &'a [(String, String, TestMarkers)],
) -> &'a str {
    // prefer the canonical main file name first
    if let Some(file) = test.files.iter().find(|file| {
        Path::new(&file.path)
            .file_name()
            .is_some_and(|name| name == "main.ds")
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

/// Ensure the session has a registered file id for a test file.
fn ensure_test_file_id(
    session: &Session,
    file_path: &Path,
    file_uri: &Uri,
    source: &str,
) -> FileId {
    if let Some(file_id) = session.files.get_id_by_path(file_path) {
        return file_id;
    }

    if let Some(file_id) = session.files.get_id_by_uri(file_uri) {
        return file_id;
    }

    let file_id = session.files.next_id();
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<test>")
        .to_string();
    let file_type = FileType::from_path_or_unknown(file_path);
    let file = File::from_text(
        file_id,
        file_name,
        file_uri.clone(),
        Some(file_path.to_path_buf()),
        file_type,
        source.to_string(),
    );

    session.files.insert(file);

    file_id
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use destack_dir as dir;
    use destack_query::{
        CompletionContext, CompletionTrigger, completions, detect_completion_context,
        find_symbol_at_offset, get_canonical_symbol, get_symbol_definition_span, query_context,
    };

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
        ));

        // resolve the compiled module for the main file
        let module = session
            .session
            .modules
            .get_by_file_id(session.file_id)
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

    /// Returns visible local value completions for a simple mdtest session.
    #[test]
    fn test_completions_include_visible_namespace_values_in_mdtest_session() {
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

        // request completions at the statement cursor
        let completions = completions(
            session.session.as_ref(),
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
        let _ = QueryTestSession::from_mdtest_with_session(
            &mdtest_case(
                r#"
const first = 1;
$0
"#,
            ),
            workspace.session(),
            workspace.fs(),
            first_root,
        );

        // build the actual target case in the same shared session
        let second_root = workspace.allocate_root("second");
        let session = QueryTestSession::from_mdtest_with_session(
            &mdtest_case(
                r#"
const foo = 1;
$0
"#,
            ),
            workspace.session(),
            workspace.fs(),
            second_root,
        );
        let cursor = session
            .markers
            .cursors
            .first()
            .unwrap_or_else(|| panic!("expected cursor marker"))
            .offset;

        // request completions through the shared-session path
        let completions = completions(
            session.session.as_ref(),
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
