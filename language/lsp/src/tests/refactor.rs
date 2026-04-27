use std::path::PathBuf;

use destack_lsp_server::UriExt;
use destack_source::{BatchEdit, Edit, FileEdit, FileId, Span};
use destack_workspace::{Change, Ref, Repository};

use crate::query::refactor::batch_edit_to_workspace_edit;

/// Skip edits for files that are missing from the registry.
#[test]
fn test_batch_edit_to_workspace_edit_skips_unknown_files() {
    // create a repository with one known file
    let repository = Repository::open_root(PathBuf::from("."));
    let known_path = PathBuf::from("/tmp/destack_lsp_refactor_known.ds");
    let known_file_id = repository.file_id_for_workspace_path(&known_path);
    let logical_path = repository.normalize_workspace_path(&known_path);
    let revision = repository
        .apply(
            &Ref::for_workspace_root(repository.workspace_root()),
            Change::set_text(&logical_path, "export const value = 1;\n"),
        )
        .expect("expected revision write");

    // build a batch edit with one known file and one unknown file
    let unknown_file_id = FileId::from_logical_str("missing/lsp-refactor.ds");
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
    let workspace_edit = batch_edit_to_workspace_edit(&repository, revision, &batch);
    let changes = workspace_edit.changes.expect("expected workspace changes");
    assert_eq!(changes.len(), 1);
    let only_uri = changes.keys().next().expect("expected one uri");
    let uri_path = only_uri
        .to_file_path()
        .expect("expected file uri")
        .into_owned();
    assert_eq!(uri_path, known_path);
}
