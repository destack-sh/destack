use std::path::PathBuf;

use destack_lsp_server::UriExt;
use destack_source::{File, FileId, FileType, Span, Uri};
use destack_workspace::Session;

use crate::query::common::span_to_location;

/// Return none when the span file is missing in the session registry.
#[test]
fn test_span_to_location_returns_none_for_unknown_file() {
    // create a session with no user files in the registry
    let session = Session::new(PathBuf::from("."));
    let span = Span::new(FileId::new(u32::MAX), 0, 0);

    // ensure missing file ids do not panic
    let location = span_to_location(&session, span);
    assert!(location.is_none());
}

/// Convert a span to a location when the file exists.
#[test]
fn test_span_to_location_converts_known_file() {
    // create a session and register a file with valid lsp uri resolution
    let session = Session::new(PathBuf::from("."));
    let file_id = session.files.next_id();
    let path = PathBuf::from("/tmp/destack_lsp_span_to_location.ds");
    let file = File::from_text(
        file_id,
        "destack_lsp_span_to_location.ds".to_string(),
        Uri::from_file_path(&path),
        Some(path.clone()),
        FileType::Destack,
        "export const value = 1;\n".to_string(),
    );
    session.files.insert(file);

    // convert the span and verify the path roundtrip
    let span = Span::new(file_id, 0, 6);
    let location = span_to_location(&session, span).expect("expected location");
    let location_path = location
        .uri
        .to_file_path()
        .expect("expected file path uri")
        .into_owned();
    assert_eq!(location_path, path);
}
