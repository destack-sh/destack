use destack_lsp_types as lsp;

use super::tests::{TestServer, position};

/// Navigate an interface method to its implementing declarations.
#[tokio::test]
async fn test_navigate_an_interface_method_to_its_implementations() {
    let source = r#"interface Greeter {
    greet(): string;
}

class Robot {
    greet(): string {
        return "beep";
    }
}

extension of Robot implements Greeter {}
"#;
    let (mut server, document) = TestServer::open_workspace(
        "member-implementations",
        &[("src/main.ds", source)],
        "src/main.ds",
    )
    .await;
    let params = document.implementations(position(1, 4));
    let expected = Some(lsp::request::GotoImplementationResponse::Link(vec![
        lsp::LocationLink {
            origin_selection_range: Some(lsp::Range::new(
                lsp::Position::new(1, 4),
                lsp::Position::new(1, 9),
            )),
            target_uri: document.uri().clone(),
            target_range: lsp::Range::new(lsp::Position::new(5, 4), lsp::Position::new(7, 5)),
            target_selection_range: lsp::Range::new(
                lsp::Position::new(5, 4),
                lsp::Position::new(5, 9),
            ),
        },
    ]));
    server
        .assert_request::<lsp::request::GotoImplementation>(params, Ok(expected))
        .await;
}
