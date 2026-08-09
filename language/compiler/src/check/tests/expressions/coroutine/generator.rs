use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_generator_functions_and_yields() {
    let session = TestSession::single(
        r#"
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}

=== checked ===
function* count(limit: int32): Generator<int32, void, void> {
/// @type.symbol symbol=count type=(int32) => *Generator<int32, void, void>
/// @type.symbol symbol=count.limit source="limit: int32" type=int32
/// @resolution.name source=Generator target=async.generator.Generator

    for (let value: int32 = 0; value < limit; value += 1) {
    /// @type.symbol symbol=count.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=count.value
    /// @resolution.name source=value target=count.value
    /// @resolution.operator source="value < limit" type=boolean operator="<" kind=builtin operands=[value as int32 families=(integer), limit as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=count.value
    /// @resolution.name source=limit target=count.limit
    /// @resolution.place source=limit placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=limit root=count.limit
    /// @resolution.name source=value target=count.value
    /// @resolution.operator source="value += 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.pattern.assign source=value kind=place
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=value read=binding(count.value) write=binding(count.value) type=int32
    /// @resolution.access source=value root=count.value

        yield value;
        /// @resolution.name source=value target=count.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=count.value

    }
}

/// @generic.instance id="Generator<int32, void, void>" template=async.generator.Generator arguments=(int32, void, void)
"#);
}
