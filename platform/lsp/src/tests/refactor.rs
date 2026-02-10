use std::path::PathBuf;

use destack_lsp_server::UriExt;
use destack_source::{BatchEdit, Edit, File, FileEdit, FileType, Span, Uri};
use destack_workspace::Session;

use crate::query::refactor::batch_edit_to_workspace_edit;

/// Skip edits for files that are missing from the registry.
#[test]
fn test_batch_edit_to_workspace_edit_skips_unknown_files() {
    // create a session with one known file
    let session = Session::new(PathBuf::from("."));
    let known_file_id = session.files.next_id();
    let known_path = PathBuf::from("/tmp/destack_lsp_refactor_known.ds");
    let known_file = File::from_text(
        known_file_id,
        "destack_lsp_refactor_known.ds".to_string(),
        Uri::from_file_path(&known_path),
        Some(known_path.clone()),
        FileType::Destack,
        "export const value = 1;\n".to_string(),
    );
    session.files.insert(known_file);

    // build a batch edit with one known file and one unknown file
    let unknown_file_id = destack_source::FileId::new(u32::MAX);
    let known_edit = FileEdit::with_edits(
        known_file_id,
        vec![Edit::replace(Span::new(known_file_id, 13, 18), "answer")],
    );
    let unknown_edit = FileEdit::with_edits(
        unknown_file_id,
        vec![Edit::replace(Span::new(unknown_file_id, 0, 0), "noop")],
    );
    let batch = BatchEdit::from_files(vec![known_edit, unknown_edit]);

    // ensure workspace edit output keeps only known file edits
    let workspace_edit = batch_edit_to_workspace_edit(&session, &batch);
    let changes = workspace_edit.changes.expect("expected workspace changes");
    assert_eq!(changes.len(), 1);
    let only_uri = changes.keys().next().expect("expected one uri");
    let uri_path = only_uri
        .to_file_path()
        .expect("expected file uri")
        .into_owned();
    assert_eq!(uri_path, known_path);
}
