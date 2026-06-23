use destack_repository::Revision;

use crate::tests::harness::TestWorkspace;
use crate::{Error, RevisionPolicy};

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
            destack_query::QueryRequest::RenameFiles(destack_query::RenameFilesRequest {
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
            destack_query::QueryRequest::RenameFiles(destack_query::RenameFilesRequest {
                profile_ids: Vec::new(),
                renames: Vec::new(),
            }),
            RevisionPolicy::Current(current_revision),
        )
        .expect("expected query with matching revision");
}
