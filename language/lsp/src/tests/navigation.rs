use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_dir::{GlobalSymbolId, LocalSymbolId};
use destack_query as query;
use destack_source::{FileId, FileSystem, ModuleId, PackageId, PhysicalFileSystem, Span};
use destack_workspace::{Edit as RepositoryEdit, HostEnvironment, Ref, Repository};

use crate::query::navigation::{outgoing_call_to_lsp, workspace_symbol_to_lsp};

/// Build a placeholder symbol id for conversion tests.
fn test_symbol_id() -> GlobalSymbolId {
    GlobalSymbolId::new(
        ModuleId::from_relative_path(
            PackageId::from_synthetic_path(Path::new("lsp-navigation")),
            Path::new("symbol.ds"),
        ),
        LocalSymbolId::new(1),
    )
}

/// Return a logical missing file id for conversion tests.
fn missing_file_id() -> FileId {
    FileId::from_logical_str("missing/lsp-navigation.ds")
}

/// Create one empty test repository.
fn test_repository() -> Repository {
    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());

    Repository::new(
        PathBuf::from("."),
        Arc::new(DiskCacheStore::new()),
        file_system,
        HostEnvironment::capture_process(),
    )
}

/// Return none for workspace symbols with unknown file ids.
#[test]
fn test_workspace_symbol_to_lsp_returns_none_for_unknown_file() {
    // create a repository and keep the symbol file id unresolved
    let repository = test_repository();
    let symbol = query::WorkspaceSymbol {
        name: "foo".to_string(),
        kind: query::SymbolKind::Function,
        file: missing_file_id(),
        range: Span::new(missing_file_id(), 0, 0),
        container: None,
    };

    // ensure conversion does not panic on missing files
    let revision = repository
        .current(&Ref::for_workspace_root(repository.workspace_root()))
        .expect("expected workspace root revision");
    let result = workspace_symbol_to_lsp(&repository, revision, &symbol);
    assert!(result.is_none());
}

/// Skip outgoing call ranges that point to missing files.
#[test]
fn test_outgoing_call_to_lsp_skips_missing_from_ranges() {
    // create a repository with a single known file for the hierarchy item
    let repository = test_repository();
    let path = PathBuf::from("/tmp/destack_lsp_navigation_call_hierarchy.ds");
    let file_id = repository.file_id(&path);
    let logical_path = repository.logical_path(&path);
    let revision = repository
        .apply_to_ref(
            &Ref::for_workspace_root(repository.workspace_root()),
            [RepositoryEdit::set_text(
                &logical_path,
                "export function foo() {}\n",
            )],
        )
        .expect("expected revision write");

    // build an outgoing call with a missing call site span
    let item = query::CallHierarchyItem {
        name: "foo".to_string(),
        kind: query::CallHierarchyKind::Function,
        detail: None,
        file: file_id,
        range: Span::new(file_id, 0, 20),
        selection_range: Span::new(file_id, 16, 19),
        symbol_id: test_symbol_id(),
    };
    let call = query::CallHierarchyOutgoingCall {
        to: item,
        from_ranges: vec![Span::new(missing_file_id(), 0, 1)],
    };

    // ensure conversion succeeds and drops unresolved ranges
    let lsp_call =
        outgoing_call_to_lsp(&repository, revision, &call).expect("expected outgoing call");
    assert!(lsp_call.from_ranges.is_empty());
}
