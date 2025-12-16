use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{FileId, FileSystem, FileType, MemoryFileSystem, Uri};
use destack_workspace::Session;

use super::{TestMarkers, parse_markers};

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
