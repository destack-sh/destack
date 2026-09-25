use tspp_lsp_types as lsp;

use super::tests::{TestServer, position, range};

/// Preserve prepared call items through incoming and outgoing call requests.
#[tokio::test]
async fn test_follow_call_hierarchy() {
    let source = r#"function target(): void {}
function source(): void {
    target();
}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "call-hierarchy",
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // prepare both ends of the call edge
    let target = server
        .request_one(document.call_hierarchy(position(0, 9)))
        .await;
    let source = server
        .request_one(document.call_hierarchy(position(1, 9)))
        .await;

    // compare the complete visible items and retain their continuations
    assert!(target.data.is_some());
    assert_eq!(
        lsp::CallHierarchyItem {
            data: None,
            ..target.clone()
        },
        lsp::CallHierarchyItem {
            name: "target".to_string(),
            kind: lsp::SymbolKind::FUNCTION,
            tags: None,
            detail: Some("target(): void".to_string()),
            uri: document.uri().clone(),
            range: range(0, 0, 0, 26),
            selection_range: range(0, 9, 0, 15),
            data: None,
        },
    );
    assert!(source.data.is_some());
    assert_eq!(
        lsp::CallHierarchyItem {
            data: None,
            ..source.clone()
        },
        lsp::CallHierarchyItem {
            name: "source".to_string(),
            kind: lsp::SymbolKind::FUNCTION,
            tags: None,
            detail: Some("source(): void".to_string()),
            uri: document.uri().clone(),
            range: range(1, 0, 3, 1),
            selection_range: range(1, 9, 1, 15),
            data: None,
        },
    );

    // preserve the caller and source range in the incoming edge
    let expected = Some(vec![lsp::CallHierarchyIncomingCall {
        from: source.clone(),
        from_ranges: vec![range(2, 4, 2, 12)],
    }]);
    let incoming = server.incoming_calls(target.clone()).await;
    assert_eq!(incoming, expected);

    // preserve the callee and source range in the outgoing edge
    let expected = Some(vec![lsp::CallHierarchyOutgoingCall {
        to: target,
        from_ranges: vec![range(2, 4, 2, 12)],
    }]);
    let outgoing = server.outgoing_calls(source).await;
    assert_eq!(outgoing, expected);
}

/// Preserve prepared type items through supertype and subtype requests.
#[tokio::test]
async fn test_follow_type_hierarchy() {
    let source = r#"class Base {}
class Derived extends Base {}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "type-hierarchy",
        &[("src/main.tspp", source)],
        "src/main.tspp",
    )
    .await;

    // prepare both ends of the inheritance edge
    let base = server
        .request_one(document.type_hierarchy(position(0, 6)))
        .await;
    let derived = server
        .request_one(document.type_hierarchy(position(1, 6)))
        .await;

    // compare the complete visible items and retain their continuations
    assert!(base.data.is_some());
    assert_eq!(
        lsp::TypeHierarchyItem {
            data: None,
            ..base.clone()
        },
        lsp::TypeHierarchyItem {
            name: "Base".to_string(),
            kind: lsp::SymbolKind::CLASS,
            tags: None,
            detail: None,
            uri: document.uri().clone(),
            range: range(0, 0, 0, 13),
            selection_range: range(0, 6, 0, 10),
            data: None,
        },
    );
    assert!(derived.data.is_some());
    assert_eq!(
        lsp::TypeHierarchyItem {
            data: None,
            ..derived.clone()
        },
        lsp::TypeHierarchyItem {
            name: "Derived".to_string(),
            kind: lsp::SymbolKind::CLASS,
            tags: None,
            detail: None,
            uri: document.uri().clone(),
            range: range(1, 0, 1, 29),
            selection_range: range(1, 6, 1, 13),
            data: None,
        },
    );

    // preserve the direct parent in the supertype response
    let supertypes = server.supertypes(derived.clone()).await;
    assert_eq!(supertypes, Some(vec![base.clone()]));

    // preserve the direct child in the subtype response
    let subtypes = server.subtypes(base).await;
    assert_eq!(subtypes, Some(vec![derived]));
}
