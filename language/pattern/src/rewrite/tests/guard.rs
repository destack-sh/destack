use crate::tests::TestRewriter;

/// Rewrite direct calls resolving to one declaration without touching indirect callees.
#[test]
fn test_rewrite_symbol_guard() {
    TestRewriter::new(
        "$CALLEE($VALUE)",
        "client.fetch($VALUE)",
        r#"
function fetch(value: string): string {
    return value;
}

function send(value: string): string {
    return value;
}

const request = fetch;

fetch("first");
request("second");
send("third");
"#,
    )
    .guard("$CALLEE == fetch")
    .assert(
        r#"
function fetch(value: string): string {
    return value;
}

function send(value: string): string {
    return value;
}

const request = fetch;

client.fetch("first");
request("second");
send("third");
"#,
    );
}

/// Rewrite one qualified call selected through an imported module namespace.
#[test]
fn test_rewrite_imported_namespace_symbol() {
    TestRewriter::new(
        "$CALLEE($VALUE)",
        "client.fetch($VALUE)",
        r#"
import * as net from "./net.tspp";

const request = net.fetch;

net.fetch("first");
request("second");
"#,
    )
    .file(
        "net.tspp",
        r#"
export function fetch(value: string): string {
    return value;
}
"#,
    )
    .guard("$CALLEE == net.fetch")
    .assert(
        r#"
import * as net from "./net.tspp";

const request = net.fetch;

client.fetch("first");
request("second");
"#,
    );
}
