use crate::tests::harness::TestWorkspace;
use crate::{Error, RevisionPolicy};
use destack_query as query;
use destack_repository::Revision;

/// Advance the repository revision after file updates.
#[test]
fn test_apply_file_advances_revision() {
    let test = TestWorkspace::new("workspace_revision_updates");
    let source_a = "export const value = 1;\n";
    let source_b = "export const value = 2;\n";
    let path = test.write_text("main.ds", source_a);

    let _ = test.apply_text(&path, source_a);
    let revision_a = test
        .workspace
        .revision_at(&path)
        .expect("expected first revision");

    let _ = test.apply_text(&path, source_b);
    let revision_b = test
        .workspace
        .revision_at(&path)
        .expect("expected second revision");

    assert_ne!(revision_b, revision_a, "expected revision to change");
}

/// Require matching revision preconditions for selected queries.
#[test]
fn test_query_requires_matching_revision() {
    let test = TestWorkspace::new("workspace_mutation_revision");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.ds", source);

    let _ = test.apply_text(&path, source);
    let current_revision = test
        .workspace
        .revision_at(&path)
        .expect("expected current revision");

    // reject stale revision preconditions
    let stale_error = test
        .workspace
        .query(
            &path,
            query::QueryRequest::RenameFiles(query::RenameFilesRequest {
                profile_ids: Vec::new(),
                renames: Vec::new(),
            }),
            RevisionPolicy::Current(Revision::NULL),
        )
        .expect_err("expected stale revision error");
    assert!(matches!(stale_error, Error::StaleRevision { .. }));

    // accept matching revision preconditions
    let _ = test
        .workspace
        .query(
            &path,
            query::QueryRequest::RenameFiles(query::RenameFilesRequest {
                profile_ids: Vec::new(),
                renames: Vec::new(),
            }),
            RevisionPolicy::Current(current_revision),
        )
        .expect("expected query with matching revision");
}
