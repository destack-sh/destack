use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Combine structural matching with an ancestor relation.
#[test]
fn test_match_inside() {
    TestMatcher::new(
        "$OBJECT.$MEMBER",
        r#"
outside.name
consume(user.name)
"#,
    )
    .inside(dir::NodeType::Expression)
    .assert(
        r#"
outside.name
consume(user.name)
        ^^^^^^^^^ match OBJECT.node="user" MEMBER.name="name"
"#,
    );
}

/// Combine structural matching with a descendant relation.
#[test]
fn test_match_has() {
    TestMatcher::new(
        "$VALUE",
        r#"
1
fetch(url)
"#,
    )
    .has(dir::NodeType::Expression)
    .assert(
        r#"
1
fetch(url)
^^^^^^^^^^ match VALUE.node="fetch(url)"
"#,
    );
}

/// Match candidates with later and earlier siblings.
#[test]
fn test_match_sibling_relations() {
    let source = r#"
target(first)
between()
target(second)
"#;

    TestMatcher::new("target($VALUE)", source)
        .precedes(dir::NodeType::Expression)
        .assert(
            r#"
target(first)
^^^^^^^^^^^^^ match VALUE.node="first"
between()
target(second)
"#,
        );

    TestMatcher::new("target($VALUE)", source)
        .follows(dir::NodeType::Expression)
        .assert(
            r#"
target(first)
between()
target(second)
^^^^^^^^^^^^^^ match VALUE.node="second"
"#,
        );
}

/// Match one exact one-based sibling position.
#[test]
fn test_match_nth_child() {
    TestMatcher::new(
        "target($VALUE)",
        r#"
target(first)
target(second)
target(third)
"#,
    )
    .nth_child(2)
    .assert(
        r#"
target(first)
target(second)
^^^^^^^^^^^^^^ match VALUE.node="second"
target(third)
"#,
    );
}

/// Limit ancestor and descendant searches by structural distance.
#[test]
fn test_match_neighbor_relations() {
    let function = "function example<T>(value: string): void {}\n";

    TestMatcher::context(
        "function f<$NAME>() {}",
        dir::NodeType::GenericParameter,
        function,
    )
    .inside_neighbor(dir::NodeType::Declaration)
    .assert(
        r#"
function example<T>(value: string): void {}
                 ^ match NAME.node="T"
"#,
    );

    TestMatcher::new("function example<T>(value: string): void {}", function)
        .has_neighbor(dir::NodeType::Declaration)
        .assert(
            r#"
function example<T>(value: string): void {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match
"#,
        );
}

/// Stop relation searches before explicit structural boundaries.
#[test]
fn test_match_relation_stops() {
    let function = "function example<T>(value: string): void {}\n";

    TestMatcher::context(
        "function f($NAME: string): void {}",
        dir::NodeType::Parameter,
        function,
    )
    .inside_until(dir::NodeType::Expression, dir::NodeType::Declaration)
    .assert(function);

    TestMatcher::new("function example<T>(value: string): void {}", function)
        .has_until(dir::NodeType::Parameter, dir::NodeType::Declaration)
        .assert(function);
}

/// Limit sibling searches to the nearest node or an explicit stop.
#[test]
fn test_match_sibling_stops() {
    let function = "function example<First, Second>(value: string): void {}\n";
    let pattern = "function f<$NAME>() {}";
    let expected = r#"
function example<First, Second>(value: string): void {}
                        ^^^^^^ match NAME.node="Second"
"#;

    TestMatcher::context(pattern, dir::NodeType::GenericParameter, function)
        .precedes_neighbor(dir::NodeType::Parameter)
        .assert(expected);

    TestMatcher::context(pattern, dir::NodeType::GenericParameter, function)
        .precedes_until(dir::NodeType::Parameter, dir::NodeType::GenericParameter)
        .assert(expected);

    TestMatcher::context(pattern, dir::NodeType::GenericParameter, function)
        .follows_neighbor(dir::NodeType::GenericParameter)
        .assert(expected);
}

/// Match arithmetic and filtered sibling positions.
#[test]
fn test_match_nth_child_formulas() {
    let functions = r#"
target(first)
target(second)
target(third)
"#;
    TestMatcher::new("target($VALUE)", functions)
        .nth_child_formula(2, 1)
        .assert(
            r#"
target(first)
^^^^^^^^^^^^^ match VALUE.node="first"
target(second)
target(third)
^^^^^^^^^^^^^ match VALUE.node="third"
"#,
        );
    TestMatcher::new("target($VALUE)", functions)
        .nth_child_formula(-1, 2)
        .assert(
            r#"
target(first)
^^^^^^^^^^^^^ match VALUE.node="first"
target(second)
^^^^^^^^^^^^^^ match VALUE.node="second"
target(third)
"#,
        );

    let function = "function example<First, Second, Third>(value: string): void {}\n";
    TestMatcher::context(
        "function f<$NAME>() {}",
        dir::NodeType::GenericParameter,
        function,
    )
    .nth_child_of_type(0, 2, dir::NodeType::GenericParameter)
    .assert(
        r#"
function example<First, Second, Third>(value: string): void {}
                        ^^^^^^ match NAME.node="Second"
"#,
    );
}
