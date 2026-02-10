use std::path::PathBuf;

use destack_dir::{GlobalSymbolId, LocalSymbolId, SymbolType};
use destack_source::{File, FileId, FileType, ModuleId, PackageId, Span, Uri};
use destack_workspace::{Session, query};

use crate::query::navigation::{outgoing_call_to_lsp, workspace_symbol_to_lsp};

/// Build a placeholder symbol id for conversion tests.
fn test_symbol_id() -> GlobalSymbolId {
    GlobalSymbolId::new(
        ModuleId::new(PackageId::new(1), 1),
        LocalSymbolId::new_typed(1, SymbolType::Function),
    )
}

/// Return none for workspace symbols with unknown file ids.
#[test]
fn test_workspace_symbol_to_lsp_returns_none_for_unknown_file() {
    // create a session and keep the symbol file id unresolved
    let session = Session::new(PathBuf::from("."));
    let symbol = query::WorkspaceSymbol {
        name: "foo".to_string(),
        kind: query::SymbolKind::Function,
        file: FileId::new(u32::MAX),
        range: Span::new(FileId::new(u32::MAX), 0, 0),
        container: None,
    };

    // ensure conversion does not panic on missing files
    let result = workspace_symbol_to_lsp(&session, &symbol);
    assert!(result.is_none());
}

/// Skip outgoing call ranges that point to missing files.
#[test]
fn test_outgoing_call_to_lsp_skips_missing_from_ranges() {
    // create a session with a single known file for the hierarchy item
    let session = Session::new(PathBuf::from("."));
    let file_id = session.files.next_id();
    let path = PathBuf::from("/tmp/destack_lsp_navigation_call_hierarchy.ds");
    let file = File::from_text(
        file_id,
        "destack_lsp_navigation_call_hierarchy.ds".to_string(),
        Uri::from_file_path(&path),
        Some(path),
        FileType::Destack,
        "export function foo() {}\n".to_string(),
    );
    session.files.insert(file);

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
        from_ranges: vec![Span::new(FileId::new(u32::MAX), 0, 1)],
    };

    // ensure conversion succeeds and drops unresolved ranges
    let lsp_call = outgoing_call_to_lsp(&session, &call).expect("expected outgoing call");
    assert!(lsp_call.from_ranges.is_empty());
}
