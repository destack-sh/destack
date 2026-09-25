use crate::tests::{DirRows, TestSession};

/// A default narrows the body binding below its declared union.
#[test]
fn test_bind_defaulted_parameters_without_undefined() {
    let session = TestSession::single(
        r#"
function greet(count: int32 | undefined = 3): int32 {
    return count;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function greet(count: int32 = 3): int32 {
    return count;
}

=== dir ===
function greet(count: int32 | undefined = 3): int32 {
/// @type.symbol symbol=greet type=(int32 | undefined?) => int32
/// @type.symbol symbol=greet.count source="count: int32 | undefined = 3" type=int32

    return count;
    /// @resolution.name source=count target=greet.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=greet.count

}
"#,
    );
}

#[test]
fn test_reject_defaults_that_produce_undefined() {
    // a parameter default must produce the narrowed body value
    let session = TestSession::single(
        r#"
function maybe(): int32 | undefined {
    return undefined;
}

function greet(count: int32 | undefined = maybe()): int32 {
    return count ?? 0;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function maybe(): int32 | undefined {
    return undefined as int32 | undefined;
}

function greet(count: int32 = maybe()): int32 {
    return count ?? 0;
}

=== dir ===
function maybe(): int32 | undefined {
/// @type.symbol symbol=maybe type=() => int32 | undefined

    return undefined;
}

function greet(count: int32 | undefined = maybe()): int32 {
/// @type.symbol symbol=greet type=(int32 | undefined?) => int32
/// @type.symbol symbol=greet.count source="count: int32 | undefined = maybe()" type=int32
/// @resolution.name source=maybe target=maybe
/// @resolution.call source=maybe() parameters=() return=int32 | undefined kind=symbol target=maybe

    return count ?? 0;
    /// @resolution.name source=count target=greet.count
    /// @resolution.operator source="count ?? 0" type=int32 operator="??" kind=builtin operands=[count as int32 families=(integer), 0 as 0 families=(integer)]
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=greet.count

}
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'int32 | undefined' is not assignable to type 'int32'"
/// @diagnostic.label line=6 column=43 span="maybe()" line_source="function greet(count: int32 | undefined = maybe()): int32 {"
/// @diagnostic.related line=6 column=29 span="|" line_source="function greet(count: int32 | undefined = maybe()): int32 {" message="expected due to this annotation"
/// @diagnostic.note message="expected 'int32', found 'undefined'"
"#,
    );
}
