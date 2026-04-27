use std::path::PathBuf;

use destack_lsp_server::UriExt;
use destack_source::{File, FileId, FileType, Span, Uri};
use destack_workspace::{Change, Ref, Repository};

use crate::query::common::{
    byte_span_to_range, byte_to_utf16_position, position_to_byte, span_to_location,
};

/// Return a logical missing file id for conversion tests.
fn missing_file_id() -> FileId {
    FileId::from_logical_str("missing/lsp-common.ds")
}

/// Return none when the span file is missing in the repository registry.
#[test]
fn test_span_to_location_returns_none_for_unknown_file() {
    // create a repository with no user files in the registry
    let repository = Repository::open_root(PathBuf::from("."));
    let revision = repository
        .current(&Ref::for_workspace_root(repository.workspace_root()))
        .expect("expected workspace root revision");
    let span = Span::new(missing_file_id(), 0, 0);

    // ensure missing file ids do not panic
    let location = span_to_location(&repository, revision, span);
    assert!(location.is_none());
}

/// Convert a span to a location when the file exists.
#[test]
fn test_span_to_location_converts_known_file() {
    // create a repository and materialize a file with valid lsp uri resolution
    let repository = Repository::open_root(PathBuf::from("."));
    let path = PathBuf::from("/tmp/destack_lsp_span_to_location.ds");
    let file_id = repository.file_id_for_workspace_path(&path);
    let logical_path = repository.normalize_workspace_path(&path);
    let revision = repository
        .apply(
            &Ref::for_workspace_root(repository.workspace_root()),
            Change::set_text(&logical_path, "export const value = 1;\n"),
        )
        .expect("expected revision write");

    // convert the span and verify the path roundtrip
    let span = Span::new(file_id, 0, 6);
    let location = span_to_location(&repository, revision, span).expect("expected location");
    let location_path = location
        .uri
        .to_file_path()
        .expect("expected file path uri")
        .into_owned();
    assert_eq!(location_path, path);
}

/// Convert a span to a location for one absolute workspace path.
#[test]
fn test_span_to_location_accepts_absolute_workspace_path() {
    // create a repository and materialize a file outside the repository root
    let repository = Repository::open_root(PathBuf::from("."));
    let path = PathBuf::from("/tmp/destack_lsp_virtual_only.ds");
    let file_id = repository.file_id_for_workspace_path(&path);
    let logical_path = repository.normalize_workspace_path(&path);
    let revision = repository
        .apply(
            &Ref::for_workspace_root(repository.workspace_root()),
            Change::set_text(&logical_path, "export const value = 1;\n"),
        )
        .expect("expected revision write");

    // ensure absolute paths still resolve to stable lsp locations
    let span = Span::new(file_id, 0, 6);
    let location = span_to_location(&repository, revision, span).expect("expected location");
    let location_path = location
        .uri
        .to_file_path()
        .expect("expected file path uri")
        .into_owned();
    assert_eq!(location_path, path);
}

/// Clamp out-of-bounds byte spans to the file end instead of defaulting to zero positions.
#[test]
fn test_byte_span_to_range_clamps_out_of_bounds_end() {
    // create one source file with known two-line boundaries
    let file = File::from_text(
        FileId::from_logical_str("test/lsp/span_range.ds"),
        "span_range.ds".to_string(),
        Uri::from_file_path(PathBuf::from("/tmp/span_range.ds")),
        None,
        FileType::Destack,
        "hello\nworld".to_string(),
    );

    // convert one span with an oversized end offset
    let range = byte_span_to_range(&file, Span::new(file.id, 0, u32::MAX));

    // assert that the start remains unchanged
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);

    // assert that the end clamps to EOF: line 1, character 5
    assert_eq!(range.end.line, 1);
    assert_eq!(range.end.character, 5);
}

/// Resolve byte offsets even when precomputed line offsets are missing.
#[test]
fn test_position_to_byte_recovers_without_line_start_offsets() {
    // create one source file and remove precomputed line offsets
    let mut file = File::from_text(
        FileId::from_logical_str("test/lsp/position_to_byte.ds"),
        "position_to_byte.ds".to_string(),
        Uri::from_file_path(PathBuf::from("/tmp/position_to_byte.ds")),
        None,
        FileType::Destack,
        "alpha\nbeta".to_string(),
    );
    file.clear_line_index();

    // resolve one position on the second line
    let position = destack_lsp_types::Position {
        line: 1,
        character: 2,
    };
    let offset = position_to_byte(&file, &position).expect("expected offset");

    // assert exact byte index: "alpha\n" (6 bytes) plus 2 bytes into "beta"
    assert_eq!(offset, 8);
}

/// Count UTF-16 units correctly across surrogate pairs.
#[test]
fn test_byte_to_utf16_position_counts_surrogate_pairs() {
    // create one source file with a four-byte emoji between ascii characters
    let file = File::from_text(
        FileId::from_logical_str("test/lsp/utf16_units.ds"),
        "utf16_units.ds".to_string(),
        Uri::from_file_path(PathBuf::from("/tmp/utf16_units.ds")),
        None,
        FileType::Destack,
        "a😀b".to_string(),
    );

    // resolve the byte offset at the start of "b": 1 byte + 4 bytes
    let position = byte_to_utf16_position(&file, 5).expect("expected position");

    // assert utf16 column: "a" = 1 unit, "😀" = 2 units
    assert_eq!(position, (0, 3));
}

/// Clamp byte offsets beyond EOF to the final position.
#[test]
fn test_byte_to_utf16_position_clamps_to_end_of_file() {
    // create one source file and clear line offsets to exercise recompute path
    let mut file = File::from_text(
        FileId::from_logical_str("test/lsp/clamp_eof.ds"),
        "clamp_eof.ds".to_string(),
        Uri::from_file_path(PathBuf::from("/tmp/clamp_eof.ds")),
        None,
        FileType::Destack,
        "line\n".to_string(),
    );
    file.clear_line_index();

    // resolve one byte offset past EOF
    let position = byte_to_utf16_position(&file, u32::MAX).expect("expected position");

    // assert clamped EOF: line index 1, character 0
    assert_eq!(position, (1, 0));
}
