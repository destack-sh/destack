use destack_dir as dir;

use crate::tests::TestMatcher;

/// Match tuple elements selected from a type alias context.
#[test]
fn test_match_tuple_element() {
    TestMatcher::context(
        "type Placeholder = ($NAME: $TYPE)",
        dir::NodeType::TupleElement,
        r#"
type UserEntry = (name: string)
type CountEntry = (count: int32)
"#,
    )
    .assert(
        r#"
type UserEntry = (name: string)
                  ^^^^^^^^^^^^ match NAME.name="name" TYPE.node="string"
type CountEntry = (count: int32)
                   ^^^^^^^^^^^^ match NAME.name="count" TYPE.node="int32"
"#,
    );
}

/// Match mapped type parameters and their source types.
#[test]
fn test_match_mapped_parameter() {
    TestMatcher::context(
        "type Placeholder<T> = { [$KEY in $SOURCE]: T }",
        dir::NodeType::TypeMappedParameter,
        r#"
type Values<T> = { [Name in keyof T]: T }
type Other<T> = { [Index in string]: T }
"#,
    )
    .assert(
        r#"
type Values<T> = { [Name in keyof T]: T }
                   ^^^^^^^^^^^^^^^^^ match KEY.name="Name" SOURCE.node="keyof T"
type Other<T> = { [Index in string]: T }
                  ^^^^^^^^^^^^^^^^^ match KEY.name="Index" SOURCE.node="string"
"#,
    );
}

/// Match where clauses selected from a declaration context.
#[test]
fn test_match_where_clause() {
    TestMatcher::context(
        "function placeholder<T>(): void where $LEFT: $RIGHT {}",
        dir::NodeType::WhereClause,
        r#"
function first<T>(): void where T: Serializable {}
function second<T>(): void where T: Comparable {}
"#,
    )
    .assert(
        r#"
function first<T>(): void where T: Serializable {}
                                ^^^^^^^^^^^^^^^ match LEFT.node="T" RIGHT.node="Serializable"
function second<T>(): void where T: Comparable {}
                                 ^^^^^^^^^^^^^ match LEFT.node="T" RIGHT.node="Comparable"
"#,
    );
}

/// Match switch cases selected from an expression context.
#[test]
fn test_match_switch_case() {
    TestMatcher::context(
        "switch (value) { case $VALUE: $BODY; break }",
        dir::NodeType::SwitchCase,
        r#"
switch (value) {
    case 1:
        first();
        break;
    case 2:
        second();
        break
}
"#,
    )
    .assert(
        r#"
switch (value) {
    case 1:
    ^ match:start VALUE.node="1" BODY.node="first()"
        first();
        break;
             ^ match:end
    case 2:
    ^ match:start VALUE.node="2" BODY.node="second()"
        second();
        break
            ^ match:end
}
"#,
    );
}

/// Match equivalent borrow qualifier orders and reject different guarantees.
#[test]
fn test_match_borrow_type_qualifiers() {
    TestMatcher::context(
        "type X = &readonly exclusive $TYPE",
        dir::NodeType::TypeExpression,
        r#"
type A = &readonly exclusive First
type B = &exclusive readonly Second
type C = &readonly Third
type D = &exclusive Fourth
"#,
    )
    .assert(
        r#"
type A = &readonly exclusive First
         ^^^^^^^^^^^^^^^^^^^^^^^^^ match TYPE.node="First"
type B = &exclusive readonly Second
         ^^^^^^^^^^^^^^^^^^^^^^^^^^ match TYPE.node="Second"
type C = &readonly Third
type D = &exclusive Fourth
"#,
    );
}
