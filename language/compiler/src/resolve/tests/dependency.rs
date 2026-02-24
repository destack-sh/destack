use destack_dir::DependencySource;
use destack_workspace::ImportEdgeKind;

use super::*;

/// Use require conditions for static imports in TypeScript CommonJS modules.
#[test]
fn test_import_edge_kind_for_typescript_commonjs_static_import() {
    let edge_kind =
        Compiler::import_edge_kind_for_dependency(DependencySource::ImportStatement, true);

    assert_eq!(edge_kind, ImportEdgeKind::Require);
}

/// Keep import-call dependencies on import conditions in TypeScript CommonJS modules.
#[test]
fn test_import_edge_kind_for_typescript_commonjs_import_call() {
    let edge_kind = Compiler::import_edge_kind_for_dependency(DependencySource::ImportCall, true);

    assert_eq!(edge_kind, ImportEdgeKind::Import);
}

/// Keep triple slash directives on import conditions in TypeScript CommonJS modules.
#[test]
fn test_import_edge_kind_for_typescript_commonjs_reference_path_directive() {
    let edge_kind =
        Compiler::import_edge_kind_for_dependency(DependencySource::ReferencePathDirective, true);

    assert_eq!(edge_kind, ImportEdgeKind::Import);
}
