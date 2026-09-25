use tspp_lsp_types as lsp;

use super::tests::{TestServer, position, range};

/// Return exact locations for every positional navigation request.
#[tokio::test]
async fn test_navigate_symbols() {
    let library = r#"export class Device {}
export function choose(device: Device): Device {
    return device;
}
"#;
    let main = r#"import { Device, choose } from "./library.tspp";

const first = new Device();
const second = choose(first);

interface Greeter {
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
        "navigation",
        &[("src/library.tspp", library), ("src/main.tspp", main)],
        "src/main.tspp",
    )
    .await;
    let library = server.document("src/library.tspp");

    // navigate the imported call to its authored function
    let definition = Some(lsp::GotoDefinitionResponse::Link(vec![library.link(
        range(3, 15, 3, 21),
        range(1, 0, 3, 1),
        range(1, 16, 1, 22),
    )]));
    server
        .assert_request(document.definition(position(3, 15)), Ok(definition))
        .await;

    // navigate the same call to its local import declaration
    let declaration = Some(lsp::GotoDefinitionResponse::Link(vec![document.link(
        range(3, 15, 3, 21),
        range(0, 17, 0, 23),
        range(0, 17, 0, 23),
    )]));
    server
        .assert_request(document.declaration(position(3, 15)), Ok(declaration))
        .await;

    // navigate one value reference to its nominal type
    let type_definition = Some(lsp::GotoDefinitionResponse::Link(vec![library.link(
        range(3, 22, 3, 27),
        range(0, 0, 0, 22),
        range(0, 13, 0, 19),
    )]));
    server
        .assert_request(
            document.type_definition(position(3, 22)),
            Ok(type_definition),
        )
        .await;

    // return the exact declaration and reference locations
    let references = Some(vec![
        document.location(range(2, 6, 2, 11)),
        document.location(range(3, 22, 3, 27)),
    ]);
    server
        .assert_request(document.references(position(3, 22), true), Ok(references))
        .await;

    // preserve read and write kinds in document highlights
    let highlights = Some(vec![
        lsp::DocumentHighlight {
            range: range(2, 6, 2, 11),
            kind: Some(lsp::DocumentHighlightKind::WRITE),
        },
        lsp::DocumentHighlight {
            range: range(3, 22, 3, 27),
            kind: Some(lsp::DocumentHighlightKind::READ),
        },
    ]);
    server
        .assert_request(document.highlights(position(3, 22)), Ok(highlights))
        .await;

    // navigate an interface method to its implementing declaration
    let implementations = Some(lsp::GotoDefinitionResponse::Link(vec![document.link(
        range(6, 4, 6, 9),
        range(10, 4, 12, 5),
        range(10, 4, 10, 9),
    )]));
    server
        .assert_request(
            document.implementations(position(6, 4)),
            Ok(implementations),
        )
        .await;
}
