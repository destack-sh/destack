use crate::tests::{DirRows, TestSession};

#[test]
fn test_optional_call_selects_non_nullish_callee() {
    let session = TestSession::single(
        r#"
function invoke(service: { callback?: () => int32 } | null): int32 | undefined {
    return service?.callback?.();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function invoke(service: { callback?: () => int32 } | null): int32 | undefined {
    return service?.callback?.() as int32 | undefined;
}

=== dir ===
function invoke(service: { callback?: () => int32 } | null): int32 | undefined {
/// @type.symbol symbol=invoke type=({ callback?: () => int32 } | null) => int32 | undefined
/// @type.symbol symbol=invoke.service source="service: { callback?: () => int32 } | null" type={ callback?: () => int32 } | null
/// @type.symbol symbol=invoke.callback source="callback?: () => int32" type=() => int32

    return service?.callback?.();
    /// @type.node source=service?.callback type=() => int32 | undefined
    /// @type.node source=service?.callback?.() type=int32
    /// @type.node source=service?.callback?.() type=int32 | undefined
    /// @resolution.name source=service target=invoke.service
    /// @resolution.member source=service?.callback receiver={ callback?: () => int32 } | null type=() => int32 | undefined kind=field target_receiver={ callback?: () => int32 } | null adjustments=(union.payload({ callback?: () => int32 } | null, { callback?: () => int32 }, { callback?: () => int32 })) key=callback target_type=() => int32 | undefined
    /// @resolution.call source=service?.callback?.() parameters=() return=int32 kind=expression target=expression
    /// @resolution.place source=service placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=service root=invoke.service
    /// @resolution.place source=service?.callback placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=service?.callback root=invoke.service keys=[callback]

}
"#,
    );
}

#[test]
fn test_call_reports_nullish_callee() {
    let session = TestSession::single(
        r#"
function invoke(callback: (() => int32) | undefined): int32 {
    return callback();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function invoke(callback: (() => int32) | undefined): int32 {
    return callback();
}

=== dir ===
function invoke(callback: (() => int32) | undefined): int32 {
/// @type.symbol symbol=invoke type=(() => int32 | undefined) => int32
/// @type.symbol symbol=invoke.callback source="callback: (() => int32) | undefined" type=() => int32 | undefined

    return callback();
    /// @type.node source=callback() type=int32
    /// @resolution.name source=callback target=invoke.callback
    /// @resolution.call source=callback() parameters=() return=int32 kind=expression target=expression
    /// @resolution.place source=callback placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=callback root=invoke.callback

}
"#,
        r#"
/// @diagnostic.error id=possibly-nullish message="value is possibly undefined"
/// @diagnostic.label line=3 column=12 span="callback()" line_source="return callback();"
/// @diagnostic.help message="narrow the value with a check or access it with '?.'"
"#,
    );
}
