use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler};
use destack_source::{FileId, FileSystem, FileType, MemoryFileSystem, Uri};
use destack_workspace::Session;

use super::{TestMarkers, parse_markers};
use crate::mdtest::MdTestCase;

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
        let fs = Arc::new(MemoryFileSystem::new());
        let session = Arc::new(Session::new(PathBuf::from("/test")).with_fs(fs.clone()));

        // register the test file
        let uri = Uri::from_string("file:///test/test.ds");
        let file_id = session.files.next_id();

        // parse markers and get clean source
        let (clean_source, markers) = parse_markers(file_id, source);

        // write clean source to memory fs
        let _ = fs.write(&PathBuf::from("/test/test.ds"), clean_source.as_bytes());

        // register with session
        let program = session.add_root(PathBuf::from("/test"));
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
            file_id,
            markers,
            source: clean_source,
            files,
        }
    }

    /// Create a test session from multiple files.
    pub fn from_files(input_files: &[(&str, &str)]) -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let session = Arc::new(Session::new(PathBuf::from("/test")).with_fs(fs.clone()));
        let program = session.add_root(PathBuf::from("/test"));

        let mut primary_file_id = None;
        let mut primary_markers = TestMarkers::default();
        let mut primary_source = String::new();
        let mut files = HashMap::new();

        for (name, source) in input_files {
            let uri = Uri::from_string(format!("file:///test/{name}"));
            let file_id = session.files.next_id();

            let (clean_source, markers) = parse_markers(file_id, source);

            let _ = fs.write(&PathBuf::from(format!("/test/{name}")), clean_source.as_bytes());
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
        let memory_fs = Arc::new(MemoryFileSystem::new());
        let cwd = PathBuf::from("/test");

        // first pass: parse markers and collect clean sources
        let mut clean_files: Vec<(String, String, TestMarkers)> = Vec::new();

        for file in &test.files {
            // parse markers with placeholder file_id (will be updated after compilation)
            let (clean_source, markers) = parse_markers(FileId(0), &file.content);
            clean_files.push((file.path.clone(), clean_source, markers));
        }

        // populate filesystem with clean sources
        for (path, clean_source, _) in &clean_files {
            let file_path = cwd.join(path);
            memory_fs
                .add_file(&file_path, clean_source.as_bytes())
                .expect("failed to add test file");
        }

        // create session with in-memory filesystem
        let fs: Arc<dyn FileSystem> = memory_fs;
        let session = Arc::new(Session::new(cwd.clone()).with_fs(fs));
        let program = session.add_root(cwd.clone());

        // create compiler and run analysis
        let compiler = Compiler::new(
            session.clone(),
            program.clone(),
            CompileOptions {
                workers: 1,
                ..Default::default()
            },
        );

        // find the main file path
        let main_name = test
            .files
            .iter()
            .find(|f| f.path == "main.ds")
            .map(|f| &f.path)
            .unwrap_or(&test.files[0].path);
        let main_path = cwd.join(main_name);

        // resolve and compile the main module
        let module_id = compiler
            .resolve_path_to_module(&main_path)
            .expect("failed to resolve module");

        compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module: module_id });
        compiler.compile();
        drop(compiler);

        // build TestFile structs with actual file IDs from compiled modules
        let mut files = HashMap::new();
        let mut all_markers = TestMarkers::default();
        let mut primary_file_id = FileId(0);
        let mut primary_source = String::new();

        for (path, clean_source, markers) in &clean_files {
            let file_path = cwd.join(path);

            // get actual file_id from session
            let file_id = session
                .files
                .get_id_by_path(&file_path)
                .unwrap_or(FileId(0));

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
            if markers.test_type.is_some() {
                all_markers.test_type.clone_from(&markers.test_type);
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

            // main.ds or first file is primary
            if path == "main.ds" || primary_file_id == FileId(0) {
                primary_file_id = file_id;
                primary_source = clean_source.clone();
            }
        }

        Self {
            session,
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

/// Convenience function to create a test session.
pub fn test_session(source: &str) -> QueryTestSession {
    QueryTestSession::from_source(source)
}

/// Convenience function to create a multi-file test session.
pub fn test_session_multi(files: &[(&str, &str)]) -> QueryTestSession {
    QueryTestSession::from_files(files)
}
