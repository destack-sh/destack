use futures::executor::block_on;
use tspp_repository::Revision;
use tspp_source::Uri;

use crate::tests::harness::TestWorkspace;
use crate::{Error, RevisionPolicy, RunQueryInput};

/// Require an explicit target when resolving a query file.
#[test]
fn test_resolve_query_file_requires_target() {
    let test = TestWorkspace::new("query-file-target");
    let config_source = "{ \"name\": \"test\" }\n";
    let config = test.write_text("destack.json", config_source);
    let _ = test.apply_text(&config, config_source);
    let source = "export const value = 1;\n";
    let path = test.write_text("main.tspp", source);
    let _ = test.apply_text(&path, source);
    let revision = test.workspace.revision().expect("read revision");
    let error = test
        .workspace
        .resolve_query_file(revision, Uri::from_path(path))
        .expect_err("query file without target should fail");

    assert!(matches!(error, Error::TargetNotSelected { .. }));
}

/// Resolve canonical URIs through the selected authored Builtin Package.
#[test]
fn test_resolve_query_file_from_canonical_uri() {
    let test = TestWorkspace::new("query-canonical-uri");
    let manifest = r#"{
  "name": "tspp",
  "targets": {
    "default": {
      "entry": ["src/custom.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;
    test.file("destack.json", manifest);
    let source = "export const custom = 1;\n";
    test.file("src/custom.tspp", source);
    let revision = test.workspace.revision().expect("read revision");

    // resolve the URI to the authored file in this exact revision
    let uri = Uri::from_string("tspp://custom.tspp");
    let file = test
        .workspace
        .resolve_query_file(revision, uri)
        .expect("resolve query file")
        .expect("canonical URI should resolve");
    assert_eq!(file.file.text(), source);
}

/// Executes latest and historical queries against their exact selected revisions.
#[test]
fn test_run_query_selects_exact_revision() {
    let test = TestWorkspace::with_entry("query-exact-revision", "main.tspp");
    let path = test.write_text("main.tspp", "export const value = 1;\n");
    let _ = test.apply_text(&path, "export const value = 1;\n");
    let first = test.workspace.revision().expect("first revision");
    let _ = test.apply_text(&path, "export const value = 2;\n");
    let latest = test.workspace.revision().expect("latest revision");
    // execute one continuation against its historical immutable revision
    let exact = block_on(test.workspace.run_query(RunQueryInput {
        revision: RevisionPolicy::Exact(first),
        request: empty_rename_files(),
    }))
    .expect("exact query");

    // execute one fresh program query against the current root head
    let current = block_on(test.workspace.run_query(RunQueryInput {
        revision: RevisionPolicy::Latest,
        request: empty_rename_files(),
    }))
    .expect("latest query");

    assert_eq!(exact.revision, first);
    assert_eq!(current.revision, latest);
    assert_ne!(first, latest);
}

/// Require matching revision preconditions for selected queries.
#[test]
fn test_run_query_requires_matching_revision() {
    let test = TestWorkspace::new("workspace_mutation_revision");
    let source = "export const value = 1;\n";
    let path = test.write_text("main.tspp", source);

    let _ = test.apply_text(&path, source);
    let current_revision = test
        .workspace
        .revision()
        .expect("expected current revision");

    // reject stale revision preconditions
    let stale_error = block_on(test.workspace.run_query(RunQueryInput {
        revision: RevisionPolicy::Current(Revision::NULL),
        request: empty_rename_files(),
    }))
    .expect_err("expected stale revision error");
    assert!(matches!(stale_error, Error::StaleRevision { .. }));

    // accept matching revision preconditions
    let _ = block_on(test.workspace.run_query(RunQueryInput {
        revision: RevisionPolicy::Current(current_revision),
        request: empty_rename_files(),
    }))
    .expect("expected query with matching revision");
}

/// Builds one empty file-rename query for revision selection exercises.
fn empty_rename_files() -> tspp_query::QueryRequest {
    tspp_query::QueryRequest::RenameFiles(tspp_query::RenameFilesRequest {
        renames: Vec::new(),
    })
}
