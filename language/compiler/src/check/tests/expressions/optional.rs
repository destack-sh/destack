use crate::tests::{DirRows, TestSession};

#[test]
fn test_optional_member_selects_non_nullish_receiver() {
    let session = TestSession::single(
        r#"
function name(user: { name: string } | null): string | undefined {
    return user?.name;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function name(user: { name: string } | null): string | undefined {
    return user?.name;
}

=== checked ===
function name(user: { name: string } | null): string | undefined {
/// @type.symbol symbol=name type=({ name: string } | null) => string | undefined
/// @type.symbol symbol=name.user source="user: { name: string } | null" type={ name: string } | null

    return user?.name;
    /// @type.node source=user?.name type=string
    /// @type.node source=user?.name type=string | undefined
    /// @resolution.name source=user target=name.user
    /// @resolution.member source=user?.name receiver={ name: string } type=string kind=field target_receiver={ name: string } key=name target_type=string
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=name.user
    /// @resolution.place source=user?.name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user?.name root=name.user keys=[name]

}
"#,
    );
}

#[test]
fn test_member_reports_nullish_receiver() {
    let session = TestSession::single(
        r#"
function name(user: { name: string } | null): string {
    return user.name;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function name(user: { name: string } | null): string {
    return user.name;
}

=== checked ===
function name(user: { name: string } | null): string {
/// @type.symbol symbol=name type=({ name: string } | null) => string
/// @type.symbol symbol=name.user source="user: { name: string } | null" type={ name: string } | null

    return user.name;
    /// @type.node source=user.name type=string
    /// @resolution.name source=user target=name.user
    /// @resolution.member source=user.name receiver={ name: string } type=string kind=field target_receiver={ name: string } key=name target_type=string
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=name.user
    /// @resolution.place source=user.name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user.name root=name.user keys=[name]

}
"#,
        r#"
/// @diagnostic.error id=possibly-nullish message="value is possibly null"
/// @diagnostic.label line=3 column=17 span="name" line_source="return user.name;"
/// @diagnostic.help message="narrow the value with a check or access it with '?.'"
"#,
    );
}

#[test]
fn test_optional_subscript_selects_non_nullish_receiver() {
    let session = TestSession::single(
        r#"
function name(user: { name: string } | null): string | undefined {
    return user?.["name"];
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function name(user: { name: string } | null): string | undefined {
    return user?.["name"];
}

=== checked ===
function name(user: { name: string } | null): string | undefined {
/// @type.symbol symbol=name type=({ name: string } | null) => string | undefined
/// @type.symbol symbol=name.user source="user: { name: string } | null" type={ name: string } | null

    return user?.["name"];
    /// @type.node source="user?.[\"name\"]" type=string
    /// @type.node source="user?.[\"name\"]" type=string | undefined
    /// @resolution.name source=user target=name.user
    /// @resolution.place source="user?.[\"name\"]" placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source="user?.[\"name\"]" root=name.user keys=[name]
    /// @resolution.subscript source="user?.[\"name\"]" type=string kind=member target="receiver={ name: string }, target=field(receiver={ name: string }, target=name, type=string), type=string"
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=name.user
    /// @type.node source="\"name\"" type="name"

}
"#,
    );
}

#[test]
fn test_subscript_reports_nullish_receiver() {
    let session = TestSession::single(
        r#"
function name(user: { name: string } | null): string {
    return user["name"];
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function name(user: { name: string } | null): string {
    return user["name"];
}

=== checked ===
function name(user: { name: string } | null): string {
/// @type.symbol symbol=name type=({ name: string } | null) => string
/// @type.symbol symbol=name.user source="user: { name: string } | null" type={ name: string } | null

    return user["name"];
    /// @type.node source="user[\"name\"]" type=string
    /// @resolution.name source=user target=name.user
    /// @resolution.place source="user[\"name\"]" placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source="user[\"name\"]" root=name.user keys=[name]
    /// @resolution.subscript source="user[\"name\"]" type=string kind=member target="receiver={ name: string }, target=field(receiver={ name: string }, target=name, type=string), type=string"
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=name.user
    /// @type.node source="\"name\"" type="name"

}
"#,
        r#"
/// @diagnostic.error id=possibly-nullish message="value is possibly null"
/// @diagnostic.label line=3 column=16 span="[" line_source="return user[\"name\"];"
/// @diagnostic.help message="narrow the value with a check or access it with '?.'"
"#,
    );
}

#[test]
fn test_optional_call_selects_non_nullish_callee() {
    let session = TestSession::single(
        r#"
function invoke(service: { callback?: () => int32 } | null): int32 | undefined {
    return service?.callback?.();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function invoke(service: { callback?: () => int32 } | null): int32 | undefined {
    return service?.callback?.();
}

=== checked ===
function invoke(service: { callback?: () => int32 } | null): int32 | undefined {
/// @type.symbol symbol=invoke type=({ callback?: Function<(), int32> } | null) => int32 | undefined
/// @type.symbol symbol=invoke.service source="service: { callback?: () => int32 } | null" type={ callback?: Function<(), int32> } | null

    return service?.callback?.();
    /// @type.node source=service?.callback type=Function<(), int32> | undefined
    /// @type.node source=service?.callback?.() type=int32
    /// @type.node source=service?.callback?.() type=int32 | undefined
    /// @resolution.name source=service target=invoke.service
    /// @resolution.member source=service?.callback receiver={ callback?: Function<(), int32> } type=Function<(), int32> | undefined kind=field target_receiver={ callback?: Function<(), int32> } key=callback target_type=Function<(), int32> | undefined
    /// @resolution.call source=service?.callback?.() parameters=() return=int32 kind=expression target=expression
    /// @resolution.place source=service placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=service root=invoke.service
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function invoke(callback: (() => int32) | undefined): int32 {
    return callback();
}

=== checked ===
function invoke(callback: (() => int32) | undefined): int32 {
/// @type.symbol symbol=invoke type=(Function<(), int32> | undefined) => int32
/// @type.symbol symbol=invoke.callback source="callback: (() => int32) | undefined" type=Function<(), int32> | undefined

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
