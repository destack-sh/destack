use crate::tests::TestRewriter;

/// Reorder repeated callable parameters while preserving their source.
#[test]
fn test_rewrite_parameter_sequence() {
    TestRewriter::new(
        "function collect($FIRST, $$$REST): void {}",
        "function collect($$$REST, $FIRST): void {}",
        r#"
function collect(first: string): void {}
function collect(first: string, second: number, third: boolean): void {}
"#,
    )
    .assert(
        r#"
function collect(first: string): void {}
function collect(second: number, third: boolean, first: string): void {}
"#,
    );
}

/// Reorder a union while preserving separators inside its repeated capture.
#[test]
fn test_rewrite_type_sequence() {
    TestRewriter::new(
        "type Values = $FIRST | $$$REST",
        "type Values = $$$REST | $FIRST",
        "type Values = First | Second | Third\n",
    )
    .assert("type Values = Second | Third | First;\n");
}

/// Preserve complete decorator elements when inserting a new decorator.
#[test]
fn test_rewrite_decorator_sequence() {
    TestRewriter::new(
        "@$$$DECORATORS class Example {}",
        "@deprecated\n@$$$DECORATORS class Example {}",
        r#"
@sealed
@logged
class Example {}
"#,
    )
    .assert(
        r#"
@deprecated
@sealed
@logged
class Example {}
"#,
    );
}

/// Preserve decorators attached to nodes inside one repeated capture.
#[test]
fn test_rewrite_decorated_node_sequence() {
    TestRewriter::new(
        "class Example { $$$MEMBERS }",
        "class Renamed { $$$MEMBERS }",
        r#"
class Example {
    @trace
    method(): void {}
}
"#,
    )
    .assert(
        r#"
class Renamed {
    @trace
    method(): void {}
}
"#,
    );
}

/// Preserve complete block bodies across the semantic tail split.
#[test]
fn test_rewrite_block_sequence() {
    TestRewriter::new(
        "function example(): void { $$$EXPRESSIONS }",
        "function example(): void { trace(); $$$EXPRESSIONS }",
        r#"
function example(): void {}
function example(): void { first(); second() }
"#,
    )
    .assert(
        r#"
function example(): void {
    trace();
}
function example(): void {
    trace();
    first();
    second();
}
"#,
    );
}

/// Preserve complete match arms while inserting one preceding arm.
#[test]
fn test_rewrite_match_arm_sequence() {
    TestRewriter::new(
        "match (value) { $$$ARMS }",
        "match (value) { Default => fallback $$$ARMS }",
        r#"
match (value) {
    First => one
    Second => two
}
"#,
    )
    .assert(
        r#"
match (value) {
    Default => fallback
    First => one
    Second => two
}
"#,
    );
}

/// Preserve adjacent tree children while replacing their parent element.
#[test]
fn test_rewrite_tree_child_sequence() {
    TestRewriter::new(
        r#"
<div>
    $$$CHILDREN
</div>
"#,
        r#"
<section>
    $$$CHILDREN
</section>
"#,
        "<div>first<span /></div>\n",
    )
    .assert("<section>first<span /></section>;\n");
}
